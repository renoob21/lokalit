use std::error::Error;

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
