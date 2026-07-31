use std::{error::Error, fs, path::Path};

use nix::{
    mount::{MntFlags, MsFlags, mount, umount2},
    unistd::{chdir, pivot_root},
};

pub fn setup_filesystem(rootfs_path: &Path) -> Result<(), Box<dyn Error>> {
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
