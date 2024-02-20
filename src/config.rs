
const HOST_ADDR: &str = "host";
const HOST_PORT: &str = "host_port";
const CONNECT_ADDR: &str = "connect";
const CONNECT_PORT: &str = "connect_port";

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
    pub host_address: Option<String>,
    pub host_port: Option<u16>,
    pub client_address: Option<String>,
    pub client_port: Option<u16>,
}

impl Config {
    pub fn empty() -> Self {
        Self {
            host_address: None,
            host_port: None,
            client_address: None,
            client_port: None,
        }
    }
    pub fn read_config() -> Config {
        let mut config = Config::empty();
        let content = match std::fs::read_to_string(CONFIG_NAME) {
            Ok(content) => content,
            Err(err) => {
                eprintln!("{}; Failed to read: {CONFIG_NAME}", err.to_string());
                return config;
            }
        };

        for line in content.lines() {
            let Some(equal_sign) = line.find('=') else {
                continue
            };
            if equal_sign == line.len() - 1 {
                continue
            }
            let key = &line[0..equal_sign];
            let value_str = &line[equal_sign+1..];
            if key == HOST_ADDR {
                config.host_address = Some(value_str.to_string());
            }
            else if key == CONNECT_ADDR {
                config.client_address = Some(value_str.to_string());
            }
            else if key == HOST_PORT {
                config.host_port = Some(value_str.parse::<u16>().unwrap());
            }
            else if key == CONNECT_PORT {
                config.client_port = Some(value_str.parse::<u16>().unwrap());
            }
        }
        config
    }
    pub fn assign_defaults(&mut self) {
        if self.host_address.is_none() {
            self.host_address = Some(DEFAULT_ADDR.to_string())
        }
        if self.client_address.is_none() {
            self.client_address = Some(DEFAULT_ADDR.to_string())
        }
        if self.client_port.is_none() {
            self.client_port = Some(DEFAULT_PORT)
        }
        if self.host_port.is_none() {
            self.host_port = Some(DEFAULT_PORT)
        }
    }
}



