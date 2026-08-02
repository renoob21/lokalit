use std::{error::Error, fs, path::Path};

use nix::unistd::Pid;

pub fn setup_cgroups(child_pid: Pid) -> Result<(), Box<dyn Error>> {
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

pub fn cleanup_cgroups() {
    let _ = fs::remove_dir("/sys/fs/cgroup/lokalit_container");
    println!("=> CGroups setting cleaned");
}
