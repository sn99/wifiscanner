#![cfg(windows)]
use regex::Regex;

use anyhow::Context;
use std::os::windows::process::CommandExt;
use std::process::Command;

use crate::Wifi;

const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Returns a list of WiFi hotspots in your area - (Windows) uses `netsh`
pub fn scan() -> anyhow::Result<Vec<Wifi>> {
    let output = Command::new("netsh.exe")
        .args(["wlan", "show", "networks", "mode=Bssid"])
        .output()?;

    let data = String::from_utf8_lossy(&output.stdout);

    parse_netsh_network_list(&data)
}

/// Returns a list of WiFi interfaces - (Windows) uses `netsh`  
pub fn show_interfaces() -> anyhow::Result<Vec<Wifi>> {
    let output = Command::new("netsh.exe")
        .args(["wlan", "show", "interfaces"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()?;

    let data = String::from_utf8_lossy(&output.stdout);
    parse_netsh_interface_list(&data)
}

fn parse_netsh_interface_list(interface_list: &str) -> anyhow::Result<Vec<Wifi>> {
    let mut wifis = Vec::new();

    // Regex for matching split, SSID and MAC, since these aren't pulled directly
    let split_regex = Regex::new("\nName")?;

    for block in split_regex.split(interface_list) {
        let mut wifi_ssid = String::new();
        let mut wifi_bssid = String::new();
        let mut wifi_security = String::new();
        let mut wifi_rssi = 0i32;
        let mut wifi_channel = String::new();

        for line in block.lines() {
            if line.contains("Authentication") {
                wifi_security = line.split(':').nth(1).unwrap_or("").trim().to_string();
            } else if line.contains("BSSID") {
                wifi_bssid = line
                    .split_once(':')
                    .map(|x| x.1)
                    .unwrap_or("")
                    .trim()
                    .to_string();
            } else if line.contains("SSID") {
                wifi_ssid = line
                    .split_once(':')
                    .map(|x| x.1)
                    .unwrap_or("")
                    .trim()
                    .to_string();
            } else if line.contains("Signal") {
                let percent = line
                    .split_once(':')
                    .map(|x| x.1)
                    .unwrap_or("")
                    .trim()
                    .replace('%', "");
                let percent: i32 = percent.parse()?;
                wifi_rssi = percent / 2 - 100;
            } else if line.contains("Channel") {
                wifi_channel = line.split(':').nth(1).unwrap_or("").trim().to_string();
            }
        }

        wifis.push(Wifi {
            mac: wifi_bssid.as_str().to_string(),
            ssid: wifi_ssid.to_string(),
            channel: wifi_channel.to_string(),
            signal_level: wifi_rssi.to_string(),
            security: wifi_security.to_string(),
        });
    }
    Ok(wifis)
}

fn parse_netsh_network_list(network_list: &str) -> anyhow::Result<Vec<Wifi>> {
    let mut wifis = Vec::new();

    // Regex for matching split, SSID and MAC, since these aren't pulled directly
    let split_regex = Regex::new("\nSSID")?;
    let ssid_regex = Regex::new("^ [0-9]* : ")?;
    let mac_regex = Regex::new("[a-fA-F0-9:]{17}")?;

    for block in split_regex.split(network_list) {
        let mut wifi_macs = Vec::new();
        let mut wifi_ssid = String::new();
        let mut wifi_channels = Vec::new();
        let mut wifi_rssi = Vec::new();
        let mut wifi_security = String::new();

        for line in block.lines() {
            if ssid_regex.is_match(line) {
                wifi_ssid = line.split(':').nth(1).unwrap_or("").trim().to_string();
            } else if line.contains("Authentication") {
                wifi_security = line.split(':').nth(1).unwrap_or("").trim().to_string();
            } else if line.contains("BSSID") {
                let captures = mac_regex.captures(line).context("RegexSyntaxError")?;
                wifi_macs.push(captures.get(0).context("SyntaxRegexError")?);
            } else if line.contains("Signal") {
                let percent = line.split(':').nth(1).unwrap_or("").trim().replace('%', "");
                let percent: i32 = percent.parse()?;
                wifi_rssi.push(percent / 2 - 100);
            } else if line.contains("Channel") {
                wifi_channels.push(line.split(':').nth(1).unwrap_or("").trim().to_string());
            }
        }

        for (mac, channel, rssi) in izip!(wifi_macs, wifi_channels, wifi_rssi) {
            wifis.push(Wifi {
                mac: mac.as_str().to_string(),
                ssid: wifi_ssid.to_string(),
                channel: channel.to_string(),
                signal_level: rssi.to_string(),
                security: wifi_security.to_string(),
            });
        }
    }

    Ok(wifis)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_parse_netsh() {
        use std::fs;

        // Note: formula for % to dBm is (% / 100) - 100
        let expected = vec![
            Wifi {
                mac: "ab:cd:ef:01:23:45".to_string(),
                ssid: "Vodafone Hotspot".to_string(),
                channel: "6".to_string(),
                signal_level: "-92".to_string(),
                security: "Open".to_string(),
            },
            Wifi {
                mac: "ab:cd:ef:01:23:45".to_string(),
                ssid: "Vodafone Hotspot".to_string(),
                channel: "6".to_string(),
                signal_level: "-73".to_string(),
                security: "Open".to_string(),
            },
            Wifi {
                mac: "ab:cd:ef:01:23:45".to_string(),
                ssid: "EdaBox".to_string(),
                channel: "11".to_string(),
                signal_level: "-82".to_string(),
                security: "WPA2-Personal".to_string(),
            },
            Wifi {
                mac: "ab:cd:ef:01:23:45".to_string(),
                ssid: "FRITZ!Box 2345 Cable".to_string(),
                channel: "1".to_string(),
                signal_level: "-50".to_string(),
                security: "WPA2-Personal".to_string(),
            },
        ];

        // Load test fixtures
        let fixture = fs::read_to_string("tests/fixtures/netsh/netsh01_windows81.txt").unwrap();

        let result = parse_netsh_network_list(&fixture).unwrap();
        assert_eq!(expected[0], result[0]);
        assert_eq!(expected[1], result[1]);
        assert_eq!(expected[2], result[2]);
        assert_eq!(expected[3], result[3]);
    }
}
