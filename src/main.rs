use std::io::{Read, Write};
use crate::config::Config;

mod connection;
mod config;
mod file_operator;

const SERVE: &str = "serve";
const CONNECT: &str = "connect";

fn main() {
    let config = Config::read_config();

    // fileserver -> fs
    // SETUP: fileserver serve / fileserver connect
    // EXCHANGE: share path / accept (id)
    let program_args = ProgramArgs::retrieve();
    if !program_args.has_args() {
        print_info();
        return;
    }
    println!("ARGS: {:?}", program_args.args);
    if program_args.args[0] == SERVE {
        // setup server
        println!("Running server");
        let server = connection::receive_connection_at_port("localhost", 2152);
        println!("server_stream {:?}", server);
    } else if program_args.args[0] == CONNECT {
        println!("Attempting connection");
        let connection_res = connection::connect_to_localhost(2152);
        let Ok(mut stream) = connection_res  else {
            eprintln!("Failed to connect: {}", connection_res.unwrap_err());
            return;
        };
        println!("Connected!");

        let msg = b"Hello!";
        stream.write(msg).unwrap();
        println!("Sent Hello, awaiting reply...");

        let mut data = [0u8; 6];
        match stream.read_exact(&mut data) {
            Ok(_) => {
                if &data == msg {
                    println!("Reply is ok!");
                } else {
                    let text = String::from_utf8(data.to_vec()).unwrap();
                    println!("Unexpected reply: {}", text);
                }
            }
            Err(e) => {
                println!("Failed to receive data: {}", e);
            }
        }
    }
}

fn print_info() {
    println!("Specify server or client:");
    println!("{SERVE}");
    println!("{CONNECT}")
}

// args: [program.exe, 0, 1, 2, ...]
pub struct ProgramArgs {
    // We can use 'exe' path for determining the relative location of config.txt
    pub exe: String,
    pub args: Vec<String>,
}

impl ProgramArgs {
    pub fn retrieve() -> Self {
        let mut args: Vec<String> = std::env::args().collect();
        if args.len() == 0 {
            panic!("Is this possible?")
        }
        let exe_path = std::mem::take(&mut args[0]);
        args.rotate_left(1);
        unsafe {
            args.set_len(args.len() - 1)
        }
        Self { exe: exe_path, args }
    }

    pub fn has_args(&self) -> bool {
        self.args.len() > 0
    }
}