use nix::{
    sched::{CloneFlags, clone},
    sys::{signal::Signal, wait::waitpid},
    unistd::sethostname,
};

fn main() {
    println!("=> Host process started. (Host: {})", get_hostname());

    // 1. Allocate memory for the child's stack (1MB)
    const STACK_SIZE: usize = 1024 * 1024;
    let mut stack = vec![0u8; STACK_SIZE];

    // 2. Define the namespaces to isolate.
    // (currently only hostname/UTS)
    let clone_flags = CloneFlags::CLONE_NEWUTS;

    // 3. Define the closure that will run IN the new container/child process.
    let child_function = || -> isize {
        println!("Container process started inside new namespace.");

        if let Err(e) = sethostname("lokalit") {
            eprintln!("Failed to set hostname: {}", e);
            return -1;
        }

        println!("=> Container hostname changed to: {}", get_hostname());

        0
    };

    let child_pid = unsafe {
        clone(
            Box::new(child_function),
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

    // 5. Wait for the container to exit before closing the host process.
    waitpid(child_pid, None).expect("Failed to wait for child");
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
