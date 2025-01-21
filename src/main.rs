use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use nix::unistd::Pid;
use std::process::{Command, Stdio};
use nix::sched::CloneFlags;
use posix_mq::{Message, Name, Queue};

#[derive(Parser)]
struct Opts {
    #[clap(long, default_value = "example")]
    image_name: String,

    #[clap(subcommand)]
    command: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    Rm {
        names: Vec<String>,
    },
    Ls,
    Mk {
        #[clap(short, long)]
        unshare: bool,

        #[clap(short, long)]
        delete: bool,

        #[clap(short, long)]
        wait: bool,

        name: String,
    },
    Tx {
        #[clap(short, long, default_value = "/foo")]
        name: String,
        message: String,
    },
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    match opts.command {
        Cmd::Ls => {
            let mqs = list_mqs()?;
            if mqs.is_empty() {
                println!("No MQs found");
            } else {
                println!("{}", mqs.join("\n"));
            }
        },
        Cmd::Rm { names } => {
            for name in names {
                let qname = Name::new(&name)?;
                let q = Queue::open(qname).expect("open");
                q.delete()?;
                println!("deleted {name}");
            }
        }
        Cmd::Mk { name, wait, delete, unshare } => {
            let pid: Pid = nix::unistd::getpid();
            if unshare {
                println!("unsharing ipc for pid {pid}");
                nix::sched::unshare(CloneFlags::CLONE_NEWIPC)?;
                println!("unshare {pid}, lsns: {}", find_namespace(pid)?);
            }
            let qname = Name::new(&name)?;
            let q = Queue::create(qname, 1, 128).expect("create");
            println!("queue {name} created");
            if wait {
                println!(" {}", make_podman_cmd(pid, &name, &opts.image_name));
                println!("waiting on message..");
                let result = q.receive()?;
                println!("received: {:?}", String::from_utf8(result.data)?);
            }

            if delete {
                q.delete()?;
                print!("deleted {name}");
            }
        }
        Cmd::Tx { name, message } => {
            let qname = Name::new(&name)?;
            let q = Queue::open(qname).expect("open");
            q.send(&Message {
                data: message.into_bytes(),
                priority: 0,
            }).expect("send");
        }
    }

    Ok(())
}

fn list_namespaces(pid: Pid) -> Result<String> {
    Ok(String::from_utf8(
        Command::new("lsns").stdout(Stdio::piped()).spawn()?.wait_with_output()?.stdout,
    )?)
}

fn find_namespace(pid: Pid) -> Result<String> {
    let ns_list = list_namespaces(pid)?;
    ns_list
        .lines()
        .find(|l| l.contains(&pid.to_string()))
        .map(|s| s.to_owned())
        .ok_or(anyhow!("no ns for {pid}"))
}

fn list_mqs() -> Result<Vec<String>> {
    let list: Vec<_> = String::from_utf8(Command::new("mq_cli").arg("ls").output().unwrap().stdout)?
        .lines()
        .map(|l| l.split_once(':'))
        .flatten()
        .map(|(s, _)| s.to_owned())
        .collect();
    Ok(list)
}

fn make_podman_cmd(pid: Pid, q_name: &str, image_name: &str) -> String {
    format!(
        r#"sudo podman run --rm -it --ipc=ns:/proc/{pid}/ns/ipc {image_name} bash -c 'mq_cli ls && mq_cli send {q_name} "Hello, from podman"'"#
    )
}
