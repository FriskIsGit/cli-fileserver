use crate::args::{ProgramArgs, CONNECT, HOST};
use crate::config::Config;

mod connection;
mod config;
mod file_operator;
mod packet;
mod args;
#[cfg(test)]
mod tests;
mod speedtest;
mod util;
mod cli;

fn main() {
    let mut config = Config::read_config();
    // fileserver -> fs
    // SETUP: fileserver host / fileserver connect
    // EXCHANGE: share path / accept (id)
    let program_args = ProgramArgs::retrieve();
    if program_args.args.is_empty() {
        ProgramArgs::print_info();
        return;
    }

    // Listen to connections, y/n, if n listen for another connection,
    match program_args.args[0].to_lowercase().as_str() {
        HOST => {
            if let Some(address) = program_args.address {
                config.host_ip = Some(address);
            }
            if let Some(port) = program_args.port {
                config.host_port = Some(port);
            }
            cli::server_impl(config)
        },
        CONNECT => {
            if let Some(address) = program_args.address {
                config.connect_ip = Some(address);
            }
            if let Some(port) = program_args.port {
                config.connect_port = Some(port);
            }
            cli::client_impl(config)
        },
        _ => {}
    }
}
