use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use nix::sched::{setns, CloneFlags};
use nix::unistd::Pid;
use posix_mq::{Message, Name, Queue};
use serde::Deserialize;
use serde_json::Value;
use std::fs::{read_dir, File};
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::thread::sleep;
use std::time::Duration;

#[derive(Parser)]
struct Opts {
    #[clap(long, default_value = "umq")]
    image_name: String,
    #[clap(subcommand)]
    command: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Remove one or more message queues
    Rm {
        queue_names: Vec<String>,
    },
    /// List all message queues from the current namespace
    Ls,
    /// Make a new message queue, with options
    Mk {
        #[clap(short, long)]
        verbose: bool,
        #[clap(short, long)]
        unshare: bool,
        #[clap(short, long)]
        number: Option<usize>,
        queue_name: String,
    },
    /// Receive messages from message queue, with options
    Rx {
        /// number of messages to receive, then close
        #[clap(short, long)]
        number: Option<usize>,
        #[clap(short, long)]
        enter: Option<i32>,
        queue_name: String,
    },
    /// Send message to a message queue, with options
    Tx {
        /// pid of the namespace process
        #[clap(short, long)]
        namespace: Option<i32>,
        queue_name: String,
        message: String,
    },
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    let done = Arc::new(AtomicBool::new(false));
    ctrlc::set_handler({
        let done = done.clone();
        move || done.store(true, Ordering::Relaxed)
    })?;

    match opts.command {
        Cmd::Ls => {
            let mqs = list_mqs()?;
            if mqs.is_empty() {
                println!("No MQs found");
            } else {
                println!("{}", mqs.join("\n"));
            }
        }
        Cmd::Rm { queue_names } => {
            for name in queue_names {
                let qname = Name::new(&name)?;
                let q = Queue::open(qname)?;
                q.delete()?;
                println!("Deleted MQ {name}");
            }
        }
        Cmd::Mk {
            verbose,
            queue_name,
            number,
            unshare,
        } => {
            let pid: Pid = nix::unistd::getpid();
            if unshare {
                println!("unsharing ipc for pid {pid}");
                nix::sched::unshare(CloneFlags::CLONE_NEWIPC | CloneFlags::CLONE_FS)?;
                println!("unshare {pid}, lsns: {:?}", find_namespace(pid)?);
            }
            let name = Name::new(&queue_name)?;
            let q = Queue::create(name, 1, 128)?;
            println!("queue {queue_name} created, ipc @ /proc/{pid}/ns/ipc");
            if verbose {
                println!("{}", make_podman_cmd(pid, &queue_name, &opts.image_name));
            }
            println!("waiting on messages..");
            rx_messages(q, done, number)?;
            println!("done");
        }
        Cmd::Rx { queue_name, number, enter } => {
            if let Some(pid) = enter {
                let ipc_ns = File::open(format!("/proc/{}/ns/ipc", pid))?;
                setns(ipc_ns, CloneFlags::CLONE_NEWIPC)?;
            }
            let name = Name::new(&queue_name)?;
            let q = Queue::open(name)?;
            println!("waiting on messages..");
            rx_messages(q, done, number)?;
            println!("done");
        }
        Cmd::Tx { queue_name, namespace, message } => {
            if let Some(pid) = namespace {
                let ipc_ns = File::open(format!("/proc/{}/ns/ipc", pid))?;
                setns(ipc_ns, CloneFlags::CLONE_NEWIPC)?;
            }
            let name = Name::new(&queue_name)?;
            let q = Queue::open(name)?;
            q.send(&Message {
                data: message.into_bytes(),
                priority: 0,
            })
                .expect("send");
        }
    }

    Ok(())
}

fn load_namespaces() -> Result<Vec<Namespace>> {
    let x = Command::new("lsns").arg("--json").stdout(Stdio::piped()).spawn()?.wait_with_output()?.stdout;
    let v: Value = serde_json::from_slice(x.as_slice())?;
    let v = v.get("namespaces").expect("{namespaces=[]}").clone();
    Ok(serde_json::from_value::<Vec<Namespace>>(v)?)
}

fn find_namespace(pid: Pid) -> Result<Namespace> {
    let ns_list = load_namespaces()?;
    ns_list.into_iter()
        .find(|ns| ns.pid.map(Pid::from_raw).map(|p| p == pid).unwrap_or(false))
        .ok_or_else(|| anyhow!("no such namespace"))
}

fn list_mqs() -> Result<Vec<String>> {
    let dev_mqs = read_dir("/dev/mqueue")?;
    let mut queues = vec![];
    for queue in dev_mqs {
        let path = queue?.path();
        let status = {
            let mut file = File::open(&path)?;
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            content
        };
        let queue_name = path
            .components()
            .last()
            .unwrap()
            .as_os_str()
            .to_string_lossy();
        queues.push(format!("/{}: {}", queue_name, status));
    }
    Ok(queues)
}

// a helper to make a podman run cmd that maps the active namespace
fn make_podman_cmd(pid: Pid, q_name: &str, image_name: &str) -> String {
    [
        format!("sudo podman run --rm -it --ipc=ns:/proc/{pid}/ns/ipc {image_name} ls"),
        format!("sudo podman run --rm -it --ipc=ns:/proc/{pid}/ns/ipc {image_name} tx {q_name} 'Hello, from podman'")
    ].map(|s| format!("\t{s}")).join("\n")
}

fn rx_messages(mut q: Queue, done: Arc<AtomicBool>, number: Option<usize>) -> Result<()> {
    let mut cnt = 0;
    'o1: loop {
        let x = thread::spawn(move || {
            let m = q.receive();
            (m, q)
        });
        while !x.is_finished() {
            sleep(Duration::from_millis(100));
            if done.load(Ordering::Relaxed) {
                break 'o1;
            }
        }
        let r = x.join().expect("join");
        q = r.1;

        let result = r.0?;
        println!("{:?}", String::from_utf8(result.data)?);

        cnt += 1;
        if number.map(|n| !(cnt < n)).unwrap_or(false) {
            break;
        }
    }

    Ok(())
}

#[derive(Clone, Debug, Deserialize)]
pub struct Namespace {
    pub ns: i64,
    //#[serde(rename="type")]
    //pub ns_type: String,
    pub nprocs: i64,
    pub pid: Option<i32>,
    pub user: String,
    //pub command: String,
}
