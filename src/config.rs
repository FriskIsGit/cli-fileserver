use std::net::TcpStream;
use std::time::Duration;

const HOST_AUTO_ACCEPT: &str = "host_auto_accept";

const HOST_IP: &str = "host";
const HOST_PORT: &str = "host_port";
const CONNECT_IP: &str = "connect";
const CONNECT_PORT: &str = "connect_port";
const WRITE_TIMEOUT: &str = "write_timeout";
const READ_TIMEOUT: &str = "read_timeout";

const CONFIG_NAME: &str = "config.txt";
/**
    Searched file name: config.txt
    Equal sign separate keys from values:
    KEY_NAME=VALUE
    Each pair is separated by a new line
    Quotation marks are not used for sequences of characters
*/

const DEFAULT_PORT: u16 = 10211;
const DEFAULT_ADDR: &str = "localhost";

pub struct Config {
    pub host_ip: Option<String>,
    pub host_port: Option<u16>,
    pub connect_ip: Option<String>,
    pub connect_port: Option<u16>,
    pub write_timeout: Option<u32>,
    pub read_timeout: Option<u32>,
    pub auto_accept: Option<bool>,
}

impl Config {
    pub fn empty() -> Self {
        Self {
            host_ip: None,
            host_port: None,
            connect_ip: None,
            connect_port: None,
            write_timeout: None,
            read_timeout: None,
            auto_accept: None,
        }
    }
    pub fn read_config() -> Config {
        let mut config = Config::empty();
        let content = match std::fs::read_to_string(CONFIG_NAME) {
            Ok(content) => content,
            Err(err) => {
                eprintln!("Failed to read: {CONFIG_NAME} | {err}");
                return config;
            }
        };

        for line in content.lines() {
            let Some(equal_sign) = line.find('=') else {
                continue;
            };
            if equal_sign == line.len() - 1 {
                continue;
            }
            let key = &line[0..equal_sign];
            let value_str = &line[equal_sign + 1..];
            match key {
                HOST_IP => config.host_ip = Some(value_str.to_string()),
                CONNECT_IP => config.connect_ip = Some(value_str.to_string()),
                HOST_PORT => config.host_port = Some(value_str.parse::<u16>().unwrap()),
                CONNECT_PORT => config.connect_port = Some(value_str.parse::<u16>().unwrap()),
                HOST_AUTO_ACCEPT => config.auto_accept = Some(value_str.parse::<bool>().unwrap()),
                READ_TIMEOUT => config.read_timeout = Some(value_str.parse::<u32>().unwrap()),
                WRITE_TIMEOUT => config.write_timeout = Some(value_str.parse::<u32>().unwrap()),
                _ => {}
            }
        }
        config
    }

    pub fn apply_timeouts(&self, stream: &mut TcpStream) {
        if let Some(seconds) = self.write_timeout {
            let timeout = Some(Duration::from_secs(seconds as u64));
            let _ = stream.set_write_timeout(timeout);
        }
        if let Some(seconds) = self.read_timeout {
            let timeout = Some(Duration::from_secs(seconds as u64));
            let _ = stream.set_read_timeout(timeout);
        }
    }
}
