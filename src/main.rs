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
    let mode = program_args.args[0].to_lowercase();
    let mode = mode.as_str();
    if HOST.starts_with(mode) {
        if let Some(host_ip) = program_args.ip {
            config.host_ip = Some(host_ip);
        }
        if let Some(port) = program_args.port {
            config.host_port = Some(port);
        }
        if let Some(auto_accept) = program_args.host_auto_accept {
            config.auto_accept = Some(auto_accept);
        }
        cli::server_impl(config)

    } else if CONNECT.starts_with(mode) {
        if let Some(ip) = program_args.ip {
            config.connect_ip = Some(ip);
        }
        if let Some(port) = program_args.port {
            config.connect_port = Some(port);
        }
        cli::client_impl(config)
    }

}
