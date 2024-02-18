
const HOST_ADDR: &str = "host";
const HOST_PORT: &str = "host_port";
const CLIENT_ADDR: &str = "client";
const CLIENT_PORT: &str = "client_port";

const CONFIG_NAME: &str = "config.txt";
/**
    Searched file name: config.txt
    Equal sign separate keys from values:
    KEY_NAME=VALUE
    Each pair is separated by a new line
    Quotation marks are not used for sequences of characters
*/

pub struct Config {
    pub host_address: Option<String>,
    pub host_port: Option<u16>,
    pub client_address: Option<String>,
    pub client_port: Option<u16>,
}

impl Config {
    pub fn new() -> Self {
        Self {
            host_address: None,
            host_port: None,
            client_address: None,
            client_port: None,
        }
    }
    pub fn read_config() -> Config {
        let content = match std::fs::read_to_string(CONFIG_NAME) {
            Ok(content) => content,
            Err(err) => panic!("{}; Failed to read: {CONFIG_NAME}", err.to_string())
        };
        let mut config = Config::new();
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
            else if key == HOST_PORT {
                config.host_port = Some(value_str.parse::<u16>().unwrap());
            }
            else if key == CLIENT_ADDR {
                config.client_address = Some(value_str.to_string());
            }
            else if key == CLIENT_PORT {
                config.client_port = Some(value_str.parse::<u16>().unwrap());
            }
        }
        config
    }
}



