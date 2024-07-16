use crate::{Error, Result, Wifi};
use std::env;
use std::process::Command;

/// Returns a list of WiFi hotspots in your area - (Linux). uses `nmcli` or `iw`.
pub(crate) fn scan() -> Result<Vec<Wifi>> {
    scan_nm().and_then(|_| scan_iw())
}

/// Returns a list of WiFi hotspots in your area - (Linux) uses `nmcli`
fn scan_nm() -> Result<Vec<Wifi>> {
    let output = Command::new("nmcli")
        .arg("--color")
        .arg("no")
        .arg("--terse")
        .arg("-f")
        .arg("ssid,chan,signal,security,bssid")
        .arg("dev")
        .arg("wifi")
        .arg("list")
        .output()
        .map_err(|_| Error::CommandNotFound)?;

    let data = String::from_utf8_lossy(&output.stdout);

    let mut result = vec![];
    for line in data.lines() {
        let mut wifi = Wifi::default();
        let mut fs = line.splitn(5, ':');
        if let Some(ssid) = fs.next() {
            wifi.ssid = ssid.to_string();
        } else {
            continue;
        }
        if let Some(channel) = fs.next() {
            wifi.channel = channel.to_string();
        } else {
            continue;
        }
        if let Some(signal_level) = fs.next() {
            wifi.signal_level = signal_level.to_string();
        } else {
            continue;
        }
        if let Some(security) = fs.next() {
            wifi.security = security.to_string();
        } else {
            continue;
        }
        if let Some(mac) = fs.next() {
            wifi.mac = mac.replace(r"\:", ":").to_string();
        } else {
            continue;
        }
        result.push(wifi);
    }
    Ok(result)
}

/// Returns a list of WiFi hotspots in your area - (Linux) uses `iw`
fn scan_iw() -> Result<Vec<Wifi>> {
    const PATH_ENV: &str = "PATH";
    let path_system = "/usr/sbin:/sbin";
    let path = env::var_os(PATH_ENV).map_or(path_system.to_string(), |v| {
        format!("{}:{}", v.to_string_lossy().into_owned(), path_system)
    });

    let output = Command::new("iw")
        .env(PATH_ENV, path.clone())
        .arg("dev")
        .output()
        .map_err(|_| Error::CommandNotFound)?;
    let data = String::from_utf8_lossy(&output.stdout);
    let interface = parse_iw_dev(&data)?;

    let output = Command::new("iw")
        .env(PATH_ENV, path)
        .arg("dev")
        .arg(interface)
        .arg("scan")
        .output()
        .map_err(|_| Error::CommandNotFound)?;
    if !output.status.success() {
        return Err(Error::CommandFailed(
            output.status,
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }
    let data = String::from_utf8_lossy(&output.stdout);
    parse_iw_dev_scan(&data)
}

fn parse_iw_dev(interfaces: &str) -> Result<String> {
    interfaces
        .split("\tInterface ")
        .take(2)
        .last()
        .ok_or(Error::NoValue)?
        .lines()
        .next()
        .ok_or(Error::NoValue)
        .map(|text| text.to_string())
}

fn parse_iw_dev_scan(network_list: &str) -> Result<Vec<Wifi>> {
    let mut wifis: Vec<Wifi> = Vec::new();
    let mut wifi = Wifi::default();
    for line in network_list.lines() {
        if let Ok(mac) = extract_value(line, "BSS ", Some("(")) {
            if !wifi.mac.is_empty()
                && !wifi.signal_level.is_empty()
                && !wifi.channel.is_empty()
                && !wifi.ssid.is_empty()
            {
                wifis.push(wifi);
                wifi = Wifi::default();
            }
            wifi.mac = mac;
        } else if let Ok(signal) = extract_value(line, "\tsignal: ", Some(" dBm")) {
            wifi.signal_level = signal;
        } else if let Ok(channel) = extract_value(line, "\t\t * primary channel: ", None) {
            wifi.channel = channel;
        } else if let Ok(ssid) = extract_value(line, "\tSSID: ", None) {
            wifi.ssid = ssid;
        } else if let Ok(security) = extract_value(line, "\t\t * Authentication suites: ", None) {
            wifi.security = security;
        }
    }
    // push the last wifi
    if !wifi.mac.is_empty()
        && !wifi.signal_level.is_empty()
        && !wifi.channel.is_empty()
        && !wifi.ssid.is_empty()
    {
        wifis.push(wifi);
    }

    Ok(wifis)
}

fn extract_value(line: &str, pattern_start: &str, pattern_end: Option<&str>) -> Result<String> {
    let start = pattern_start.len();
    if start < line.len() && &line[0..start] == pattern_start {
        let end = match pattern_end {
            Some(end) => line.find(end).ok_or(Error::NoValue)?,
            None => line.len(),
        };
        Ok(line[start..end].to_string())
    } else {
        Err(Error::NoValue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Read;
    use std::path::PathBuf;

    #[test]
    fn test_nmcli() {
        if let Err(e) = Command::new("nmcli").arg("--version").output() {
            eprintln!("nmlci is not found: {e}");
            return;
        }
        let wifis = scan_nm().expect("failed to scan");
        println!("Wifis: {wifis:?}");
    }

    #[test]
    fn should_parse_iw_dev() {
        let expected = "wlp2s0";

        // FIXME: should be a better way to create test fixtures
        let path = PathBuf::from("tests/fixtures/iw/iw_dev_01.txt");
        let mut file = File::open(path).unwrap();

        let mut filestr = String::new();
        let _ = file.read_to_string(&mut filestr).unwrap();

        let result = parse_iw_dev(&filestr).unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn should_parse_iw_dev_scan() {
        let mut expected: Vec<Wifi> = vec![
            Wifi {
                mac: "11:22:33:44:55:66".to_string(),
                ssid: "hello".to_string(),
                channel: "10".to_string(),
                signal_level: "-67.00".to_string(),
                security: "PSK".to_string(),
            },
            Wifi {
                mac: "66:77:88:99:aa:bb".to_string(),
                ssid: "hello-world-foo-bar".to_string(),
                channel: "8".to_string(),
                signal_level: "-89.00".to_string(),
                security: "PSK".to_string(),
            },
        ];

        // FIXME: should be a better way to create test fixtures
        let path = PathBuf::from("tests/fixtures/iw/iw_dev_scan_01.txt");
        let mut file = File::open(path).unwrap();
        let mut filestr = String::new();
        let _ = file.read_to_string(&mut filestr).unwrap();

        let result = parse_iw_dev_scan(&filestr).unwrap();
        assert_eq!(expected[0], result[0]);
        assert_eq!(expected[1], result[4]);
    }
}
