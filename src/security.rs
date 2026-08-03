use std::error::Error;

use caps::{CapSet, Capability};
use libseccomp::{ScmpAction, ScmpFilterContext, ScmpSyscall};

pub fn apply_seccomp_filter() -> Result<(), Box<dyn Error>> {
    // Default allow contex
    let mut ctx = ScmpFilterContext::new(ScmpAction::Allow)?;

    let block_action = ScmpAction::Errno(nix::libc::EPERM);

    // block list
    let dangerous_syscalls = ["ptrace", "unshare", "kexec_load", "bpf", "reboot", "mount"];

    for &syscall_name in &dangerous_syscalls {
        let syscall = ScmpSyscall::from_name(syscall_name)?;
        ctx.add_rule(block_action, syscall)?;
    }

    ctx.load()?;

    Ok(())
}

pub fn drop_privileges() -> Result<(), Box<dyn Error>> {
    let dangerous_caps = vec![
        Capability::CAP_SYS_ADMIN,
        Capability::CAP_SYS_BOOT,
        Capability::CAP_SYS_MODULE,
        Capability::CAP_NET_ADMIN,
        Capability::CAP_SYS_TIME,
        Capability::CAP_MAC_ADMIN,
        Capability::CAP_SYS_PTRACE,
    ];

    for cap in dangerous_caps {
        caps::drop(None, CapSet::Bounding, cap)?;
        caps::drop(None, CapSet::Effective, cap)?;
        caps::drop(None, CapSet::Permitted, cap)?;
        caps::drop(None, CapSet::Inheritable, cap)?;
    }

    Ok(())
}
