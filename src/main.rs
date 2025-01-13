use anyhow::{anyhow, Result};
use clap::{Parser};
use nix::libc;
use nix::sched::unshare;
use nix::sched::CloneFlags;
use nix::unistd::Pid;
use posix_mq::{Name, Queue};
use std::ffi::CString;
use std::process::{Command, Stdio};

#[derive(Parser)]
struct Opts {
    #[clap(short, long)]
    unshare: bool,

    #[clap(short, long, default_value = "/foo")]
    name: String,
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    let pid: Pid = nix::unistd::getpid();
    if opts.unshare {
        unshare(CloneFlags::CLONE_NEWIPC)?;
        println!("unshare success: {pid}");
        println!("namespace: {}", find_namespace(pid)?);
    } else {
        let cstr = CString::new(opts.name.clone())?;
        unsafe {
            libc::mq_unlink(cstr.as_ptr());
        };
    }
    println!(
        "podman:\n    sudo podman run --rm -it --ipc=ns:/proc/{pid}/ns/ipc -v $HOME/.cargo/bin:/bin:ro ubuntu:22.04 mq_cli ls"
    );

    let name = Name::new(&opts.name)?;
    let queue = Queue::create(name, 1, 128).expect("failed to create queue");
    println!("queue created successfully");

    let result = queue.receive()?;
    println!("received: {:?}", String::from_utf8(result.data)?);

    queue.delete()?;

    Ok(())
}

fn find_namespace(pid: Pid) -> Result<String> {
    let out = String::from_utf8(Command::new("lsns").stdout(Stdio::piped()).spawn()?.wait_with_output()?.stdout)?;
    out.lines().find(|l| l.contains(&pid.to_string())).map(|s| s.to_owned()).ok_or(anyhow!(""))
}
