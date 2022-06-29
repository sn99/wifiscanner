fn main() {
    let networks = wifiscanner::scan().expect("Cannot scan network");
    println!("== List of networks");
    for network in networks {
        println!(
            "{} {:20} {:10} {:4} {}",
            network.mac, network.ssid, network.channel, network.signal_level, network.security
        );
    }

    #[cfg(target_os = "windows")]
    {
        let interfaces = wifiscanner::show_interfaces().expect("Cannot scan network");
        println!("== List of interfaces");
        for network in interfaces {
            println!(
                "{} {:20} {:10} {:4} {} {}",
                network.mac,
                network.ssid,
                network.channel,
                network.signal_level,
                network.security,
                network.state
            );
        }
    }
}
