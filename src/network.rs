use std::{error::Error, process::Command};

use nix::unistd::Pid;

pub fn setup_network(child_pid: Pid) -> Result<(), Box<dyn Error>> {
    let pid_str = child_pid.as_raw().to_string();

    // 1. Create veth pair
    Command::new("ip")
        .args([
            "link",
            "add",
            "veth-host",
            "type",
            "veth",
            "peer",
            "name",
            "veth-guest",
        ])
        .status()?;

    // 2. move Guest to container
    Command::new("ip")
        .args(["link", "set", "veth-guest", "netns", &pid_str])
        .status()?;
    // 3. Configure Host-end (set ip 10.0.0.1 and bring it up)
    Command::new("ip")
        .args(["addr", "add", "10.0.0.1/24", "dev", "veth-host"])
        .status()?;
    Command::new("ip")
        .args(["link", "set", "veth-host", "up"])
        .status()?;
    // 4. Configure Guest-End (set ip to 10.0.0.2, bring it up, and set the default route) -> use nsenter
    Command::new("nsenter")
        .args([
            "-t",
            &pid_str,
            "-n",
            "ip",
            "addr",
            "add",
            "10.0.0.2/24",
            "dev",
            "veth-guest",
        ])
        .status()?;
    Command::new("nsenter")
        .args([
            "-t",
            &pid_str,
            "-n",
            "ip",
            "link",
            "set",
            "veth-guest",
            "up",
        ])
        .status()?;
    Command::new("nsenter")
        .args([
            "-t", &pid_str, "-n", "ip", "route", "add", "default", "via", "10.0.0.1",
        ])
        .status()?;
    // Bring up the loopback interface inside container
    Command::new("nsenter")
        .args(["-t", &pid_str, "-n", "ip", "link", "set", "lo", "up"])
        .status()?;
    // 5. Enable IP forwarding on the host (allow the host to be router)
    Command::new("sysctl")
        .args(["-w", "net.ipv4.ip_forward=1"])
        .status()?;
    // 6. Setup NAT (Masquerading) to hide container IP behind host's IP
    Command::new("iptables")
        .args([
            "-t",
            "nat",
            "-A",
            "POSTROUTING",
            "-s",
            "10.0.0.0/24",
            "-j",
            "MASQUERADE",
        ])
        .status()?;
    Ok(())
}

pub fn cleanup_network() {
    let _ = Command::new("iptables")
        .args([
            "-t",
            "nat",
            "-D",
            "POSTROUTING",
            "-s",
            "10.0.0.0/24",
            "-j",
            "MASQUERADE",
        ])
        .status();
    println!("=> Network setting cleaned");
}
