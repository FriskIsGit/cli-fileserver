use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};
use std::time::Instant;
use crate::args::{CONNECT, ProgramArgs, HOST};
use crate::config::Config;
use crate::packet::{FileOfferPacket, FilePacket, Packet, SpeedPacket};

mod connection;
mod config;
mod file_operator;
mod packet;
mod args;
mod tests;

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
    let connect_to = &config.connect_address.unwrap();
    let port = config.connect_port.unwrap();
    let connection_res = connection::connect_ipv4(connect_to, port);
    let Ok(mut stream) = connection_res else {
        eprintln!("Failed to connect: {}", connection_res.unwrap_err());
        return;
    };
    let peer_addr = stream.peer_addr().unwrap().ip();
    println!("Connected to {peer_addr}!");
    established_connection_stage(stream);
}

fn server_impl(config: Config) {
    println!("Running server");
    let host_address = &config.host_address.unwrap();
    let port = config.host_port.unwrap();
    let listener = connection::create_server(host_address, port);
    let port = listener.local_addr().unwrap().port();
    println!("Hosting server on port: {port}");

    let auto_accept = if let Some(accept) = config.auto_accept { accept } else { false };
    // Connection listener implementation
    for incoming_conn in listener.incoming() {
        match incoming_conn {
            Ok(stream) => {
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
                established_connection_stage(stream);
            }
            Err(err) => {
                eprintln!("Failed to accept connection: {err}");
                continue;
            }
        };
    };
}

const SPEEDTEST_TRANSFERS: usize = 100;
const MB_1: usize = 1048576;

fn established_connection_stage(mut stream: TcpStream) {
    loop {
        println!("[shutdown, send <count>, read <count>, speedtest in, speedtest out]");
        let line = read_line();
        let command = line.as_str();
        println!("[{command}]");
        if command.starts_with("shutdown") {
            let _ = stream.shutdown(Shutdown::Both);
            return;
        } else if command.starts_with("send") {
            let Some(whitespace) = command.find(' ') else {
                continue
            };
            let count = command[whitespace + 1..].parse::<usize>().unwrap();
            for _ in 0..count {
                let packet = FileOfferPacket::new(86242, "file1.txt".to_string());
                packet.write_header(&mut stream);
                packet.write(&mut stream);
            }
        } else if command.starts_with("read") {
            let whitespace = command.find(' ').unwrap();
            let count = command[whitespace + 1..].parse::<usize>().unwrap();
            for _ in 0..count {
                read_and_handle_packet(&mut stream);
            }
        } else if command.starts_with("speedtest in") {
            let mut start = Instant::now();
            for i in 0..SPEEDTEST_TRANSFERS {
                read_and_handle_packet(&mut stream);
                let elapsed = start.elapsed();
                let megabytes = (i + 1) as f64;
                let seconds = elapsed.as_millis() as f64 / 1000f64;
                println!("Received {}/{SPEEDTEST_TRANSFERS} packets ({:.2} MB/s)", i + 1, megabytes / seconds);
            }
            let elapsed = start.elapsed();
            let megabytes = SPEEDTEST_TRANSFERS as f64;
            let seconds = elapsed.as_millis() as f64 / 1000f64;
            println!("Time taken: {:?}", elapsed);
            println!("Speed: {:.2} MB/s", megabytes / seconds);
        } else if command.starts_with("speedtest out") {
            println!("Preparing to send {SPEEDTEST_TRANSFERS} packets of size = {MB_1}");
            let mut payload = vec![0u8; MB_1];
            for i in 0..MB_1 {
                payload[i] = i as u8;
            }
            let packet = SpeedPacket::wrap(&payload).unwrap();
            println!("Starting..");
            let start = Instant::now();
            for i in 0..SPEEDTEST_TRANSFERS {
                packet.write_header(&mut stream);
                packet.write(&mut stream);

                let elapsed = start.elapsed();
                let megabytes = (i + 1) as f64;
                let seconds = elapsed.as_millis() as f64 / 1000f64;
                println!("Written {}/{SPEEDTEST_TRANSFERS} packets ({:.2} MB/s)", i + 1, megabytes / seconds);
            };
            let elapsed = start.elapsed();
            let megabytes = SPEEDTEST_TRANSFERS as f64;
            let seconds = elapsed.as_millis() as f64 / 1000f64;
            println!("Time taken: {:?}", elapsed);
            println!("Speed: {:.2} MB/s", megabytes / seconds);
        }
    }
}

fn read_and_handle_packet(stream: &mut TcpStream) {
    let mut id = [0u8; 4];
    packet::tcp_read_safe(&mut id, stream);
    let id = packet::read_id(id);

    let mut packet_size = [0u8; 4];
    packet::tcp_read_safe(&mut packet_size, stream);
    let packet_size = packet::read_content_size(packet_size);

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
                    println!("FilePacket {} {} {:?}", packet.chunk_id, packet.payload_size, packet.file_bytes)
                }
                Err(err) => eprintln!("Failure: {err}")
            }
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