use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};
use std::path::Path;
use std::time::{Duration, Instant};
use crate::args::{CONNECT, ProgramArgs, HOST};
use crate::config::Config;
use crate::file_operator::FileFeeder;
use crate::packet::{FileOfferPacket, FilePacket, Packet, PingPacket, SpeedPacket, tcp_write_safe};
use crate::speedtest::{speedtest_in, speedtest_out};

mod connection;
mod config;
mod file_operator;
mod packet;
mod args;
mod tests;
mod speedtest;

const PINGS: usize = 100;
fn main() {
    let mut config = Config::read_config();
    config.assign_defaults();
    // fileserver -> fs
    // SETUP: fileserver host / fileserver connect
    // EXCHANGE: share path / accept (id)
    let program_args = ProgramArgs::retrieve();
    if !program_args.has_args() {
        ProgramArgs::print_info();
        return;
    }
    // Listen to connections, y/n, if n listen for another connection,
    match program_args.args[0].as_str() {
        HOST => server_impl(config),
        CONNECT => client_impl(config),
        _ => {}
    }
}

fn client_impl(config: Config) {
    println!("Attempting connection");
    let target_address = config.connect_address.as_ref().unwrap();
    let port = config.connect_port.unwrap();
    let connection_res = connection::connect_ipv4(target_address, port);
    let Ok(mut stream) = connection_res else {
        eprintln!("Failed to connect: {}", connection_res.unwrap_err());
        return;
    };
    let peer_addr = stream.peer_addr().unwrap().ip();
    println!("Connected to {peer_addr}!");
    apply_config_to_tcp(&config, &mut stream);
    established_connection_stage(stream);
}

fn apply_config_to_tcp(config: &Config, stream: &mut TcpStream) {
    if let Some(seconds) = config.write_timeout {
        let timeout = Some(Duration::from_secs(seconds as u64));
        let _ = stream.set_write_timeout(timeout);
    }
    if let Some(seconds) = config.read_timeout {
        let timeout = Some(Duration::from_secs(seconds as u64));
        let _ = stream.set_read_timeout(timeout);
    }
}

fn server_impl(config: Config) {
    println!("Running server");
    let host_address = config.host_address.as_ref().unwrap();
    let port = config.host_port.unwrap();
    let listener = connection::create_server(host_address, port);
    let port = listener.local_addr().unwrap().port();
    println!("Hosting server on port: {port}");

    let auto_accept = if let Some(accept) = config.auto_accept { accept } else { false };
    // Connection listener implementation
    for incoming_conn in listener.incoming() {
        match incoming_conn {
            Ok(mut stream) => {
                let ip = stream.peer_addr().unwrap().ip();
                if !auto_accept {
                    println!("Do you want to accept connection from: {} (y/n)", ip);
                    if !read_line().starts_with('y') {
                        let _ = stream.shutdown(Shutdown::Both);
                        continue;
                    }
                }
                let peer_addr = stream.peer_addr().unwrap().ip();
                println!("Connected to {peer_addr}!");
                apply_config_to_tcp(&config, &mut stream);
                established_connection_stage(stream);
            }
            Err(err) => {
                eprintln!("Failed to accept connection: {err}");
                continue;
            }
        };
    };
}

fn established_connection_stage(mut stream: TcpStream) {
    loop {
        println!("[shutdown, ping st, ping en, share <path>, read <count>, speedtest in, speedtest out]");
        let line = read_line();
        let command = line.as_str();
        println!("[{command}]");
        if command.starts_with("shutdown") {
            let _ = stream.shutdown(Shutdown::Both);
            return;
        } else if command.starts_with("share") {
            let Some(whitespace) = command.find(' ') else {
                continue
            };
            let path = &command[whitespace + 1..];
            if !Path::new(path).exists() {
                eprintln!("File not found");
                continue
            }
            stream_file(path, &mut stream);
        } else if command.starts_with("read") {
            let whitespace = command.find(' ').unwrap();
            let count = command[whitespace + 1..].parse::<usize>().unwrap();
            for _ in 0..count {
                read_and_handle_packet(&mut stream);
            }
        } else if command.starts_with("speedtest in") || command.starts_with("si") {
            speedtest_in(&mut stream);
        } else if command.starts_with("speedtest out") || command.starts_with("so") {
            speedtest_out(&mut stream);
        } else if command.starts_with("ping 1") {
            for _ in 0..PINGS {
                let ping_start = Instant::now();
                write_ping(&mut stream);
                read_ping(&mut stream);
                let end = ping_start.elapsed();
                println!("W/R: {:?}", end);
            }
        } else if command.starts_with("ping 2") {
            for _ in 0..PINGS {
                let ping_start = Instant::now();
                read_ping(&mut stream);
                write_ping(&mut stream);
                let end = ping_start.elapsed();
                println!("W/R: {:?}", end);
            }
        }
    }
}

fn read_and_handle_packet(stream: &mut TcpStream) {
    let id = packet::read_id(stream);
    let packet_size = packet::read_content_size(stream);

    let mut field_buffer = vec![0u8; packet_size as usize];
    packet::tcp_read_safe(&mut field_buffer, stream);
    match id {
        FileOfferPacket::ID => {
            match FileOfferPacket::construct(&field_buffer) {
                Ok(packet) => {
                    println!("Download {}?  [{}]", packet.file_name, packet.format_size())
                }
                Err(err) => eprintln!("Failure: {err}")
            }
        }
        FilePacket::ID => {
            match FilePacket::wrap(&field_buffer) {
                Ok(packet) => {
                    let transaction_id = packet.transaction_id;
                    let chunk = packet.chunk_id;
                    let content_len = packet.file_bytes.len();
                    println!("Transaction:{transaction_id} | File packet: chunk={chunk} content_len={content_len}")
                }
                Err(err) => eprintln!("Failure: {err}")
            }
        }
        PingPacket::ID => {
            let taken = PingPacket::millis_taken(&field_buffer);
            println!("Ping received after {taken}ms");
        }
        SpeedPacket::ID => {
            // don't construct packet - waste of time
        }
        _ => {
            println!("Unrecognized packet {id}");
            return;
        }
    }
}

fn read_line() -> String {
    let mut buffer = String::new();
    return match std::io::stdin().read_line(&mut buffer) {
        Ok(_) => buffer.trim_end().to_string(),
        Err(_) => "".into(),
    };
}

fn stream_file(path: &str, stream: &mut TcpStream) {
    let mut file_feeder = FileFeeder::new(path).expect("Couldn't initialize file reader");
    let mut chunk_id = 0;
    while file_feeder.has_next_chunk() {
        let chunk = file_feeder.read_next_chunk().expect("No next chunk");
        let packet = FilePacket::new(1, chunk_id, &chunk);
        packet.write_header(stream);
        packet.write(stream);
        chunk_id += 1;
    }
}

pub fn write_ping(stream: &mut TcpStream) {
    let ping = PingPacket::new_ping();
    ping.write_header(stream);
    ping.write(stream);
}

fn read_ping(stream: &mut TcpStream) {
    let id = packet::read_id(stream);
    let packet_size = packet::read_content_size(stream);

    if id != PingPacket::ID {
        eprintln!("ID {id} wasn't expected at this time");
    }
    let mut field_buffer = vec![0u8; packet_size as usize];
    packet::tcp_read_safe(&mut field_buffer, stream)
}