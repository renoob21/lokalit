use std::{error::Error, ffi::CString, fs, path::Path};

use nix::{
    mount::{MntFlags, MsFlags, mount, umount2},
    sched::{CloneFlags, clone},
    sys::{signal::Signal, wait::waitpid},
    unistd::{Pid, chdir, execve, pivot_root, sethostname},
};

fn main() {
    println!("=> Host process started. (Host: {})", get_hostname());

    // 1. Allocate memory for the child's stack (1MB)
    const STACK_SIZE: usize = 1024 * 1024;
    let mut stack = vec![0u8; STACK_SIZE];

    // 2. Define the namespaces to isolate.
    // (currently only hostname/UTS)
    let clone_flags = CloneFlags::CLONE_NEWUTS | CloneFlags::CLONE_NEWNS | CloneFlags::CLONE_NEWPID;

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

    // 5. Wait for the container to exit before closing the host process.
    waitpid(child_pid, None).expect("Failed to wait for child");

    let _ = fs::remove_dir("/sys/fs/cgroup/lokalit_container");

    println!(
        "=> Container stopped. Host hostname remains: {}",
        get_hostname()
    );
}

fn setup_filesystem(rootfs_path: &Path) -> Result<(), Box<dyn Error>> {
    // 1. Remount root filesystem as private to prevent leak to the host
    mount(
        None::<&str>,
        "/",
        None::<&str>,
        MsFlags::MS_PRIVATE | MsFlags::MS_REC,
        None::<&str>,
    )?;

    // 2. Bind the new root to itself
    mount(
        Some(rootfs_path),
        rootfs_path,
        None::<&str>,
        MsFlags::MS_BIND | MsFlags::MS_REC,
        None::<&str>,
    )?;

    let old_root = rootfs_path.join(".old_root");
    if !old_root.exists() {
        fs::create_dir(&old_root)?;
    }

    // 4. Perform pivot_root
    pivot_root(rootfs_path, &old_root)?;

    // 5. Change directory to new root
    chdir("/")?;

    // 6. Unmount old host filesystem
    umount2("/.old_root", MntFlags::MNT_DETACH)?;

    // 7. Cleanup
    fs::remove_dir("/.old_root")?;

    mount(
        Some("proc"),
        "/proc",
        Some("proc"),
        MsFlags::empty(),
        None::<&str>,
    )?;

    Ok(())
}

fn setup_cgroups(child_pid: Pid) -> Result<(), Box<dyn Error>> {
    let cgroup_path = Path::new("/sys/fs/cgroup/lokalit_container");

    if !cgroup_path.exists() {
        fs::create_dir(cgroup_path)?;
    }

    fs::write(cgroup_path.join("memory.max"), "100000000")?;

    fs::write(cgroup_path.join("cpu.max"), "20000 100000")?;

    fs::write(
        cgroup_path.join("cgroup.procs"),
        child_pid.as_raw().to_string(),
    )?;

    Ok(())
}

fn get_hostname() -> String {
    nix::unistd::gethostname()
        .expect("Failed to get hostname")
        .to_str()
        .unwrap()
        .to_string()
}
