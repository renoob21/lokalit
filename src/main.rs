use std::{ffi::CString, path::Path, thread, time::Duration};

use nix::{
    sched::{CloneFlags, clone},
    sys::{signal::Signal, wait::waitpid},
    unistd::{execve, sethostname},
};

use crate::{
    cgroups::{cleanup_cgroups, setup_cgroups},
    filesystem::setup_filesystem,
    network::{cleanup_network, setup_network},
};

mod cgroups;
mod filesystem;
mod network;

fn main() {
    println!("=> Host process started. (Host: {})", get_hostname());

    // 1. Allocate memory for the child's stack (1MB)
    const STACK_SIZE: usize = 1024 * 1024;
    let mut stack = vec![0u8; STACK_SIZE];

    // 2. Define the namespaces to isolate.
    // (currently only hostname/UTS)
    let clone_flags = CloneFlags::CLONE_NEWUTS
        | CloneFlags::CLONE_NEWNS
        | CloneFlags::CLONE_NEWPID
        | CloneFlags::CLONE_NEWNET;

    // 3. Define the closure that will run IN the new container/child process.
    let mut child_function = || -> isize {
        println!("Container process started inside new namespace.");

        if let Err(e) = sethostname("lokalit") {
            eprintln!("Failed to set hostname: {}", e);
            return -1;
        }

        println!("=> Container hostname changed to: {}", get_hostname());

        println!("=> Setting up container environment...");
        let rootfs_path = Path::new("./loka-rootfs");
        if let Err(e) = setup_filesystem(rootfs_path) {
            eprintln!("Filesystem setup failed: {}", e);
            return -1;
        }

        let shell = CString::new("/bin/sh").expect("Failed to create CString");
        let args = [CString::new("/bin/sh").unwrap()];

        let env = [
            CString::new("PATH=/bin:/usr/bin/:/sbin:/usr/sbin").unwrap(),
            CString::new("TERM=xterm").unwrap(),
        ];

        println!("=> Waiting for network setup...");
        thread::sleep(Duration::from_secs(1));

        match execve(&shell, &args, &env) {
            Ok(_) => unreachable!("execve replaces the process and never returns on success"),
            Err(e) => {
                eprintln!("=> execve failed: {}", e);
                return -1;
            }
        }
    };

    let child_pid = unsafe {
        clone(
            Box::new(&mut child_function),
            &mut stack,
            clone_flags,
            Some(Signal::SIGCHLD as i32), // Notify parent when child exits
        )
        .expect("Failed to clone process")
    };

    println!(
        "=> Parent waiting for container (PID: {}) to finish...",
        child_pid
    );

    if let Err(e) = setup_cgroups(child_pid) {
        eprintln!("=> Failed to set up cgroups: {}", e);
    } else {
        println!("=> Cgroups configured: Memory restricted to 100MB, CPU restricted to 20%");
    }

    if let Err(e) = setup_network(child_pid) {
        eprintln!("=> Failed to setup network: {}", e);
    } else {
        println!("Network configured (Container IP: 10.0.0.2)")
    }

    // 5. Wait for the container to exit before closing the host process.
    waitpid(child_pid, None).expect("Failed to wait for child");

    cleanup_cgroups();
    cleanup_network();

    println!(
        "=> Container stopped. Host hostname remains: {}",
        get_hostname()
    );
}

fn get_hostname() -> String {
    nix::unistd::gethostname()
        .expect("Failed to get hostname")
        .to_str()
        .unwrap()
        .to_string()
}
