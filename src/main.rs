use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};
use std::time::Instant;
use crate::args::{CONNECT, ProgramArgs, HOST};
use crate::config::Config;
use crate::packet::{FIELD_OFFSET, FileOfferPacket, FilePacket, Packet, AnswerPacket, SpeedPacket};

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
                        continue
                    }
                }

                let peer_addr = stream.peer_addr().unwrap().ip();
                println!("Connected to {peer_addr}!");
                established_connection_stage(stream);
            }
            Err(err) => {
                eprintln!("Failed to accept connection: {err}");
                continue
            }
        };
    };

}

const SPEEDTEST_TRANSFERS: usize = 3000;
const MB_1: usize = 1048576;
fn established_connection_stage(mut stream: TcpStream) {
    loop {
        println!("[shutdown, send <count>, receive <count>, speedtest in, speedtest out]");
        let line = read_line();
        let command = line.as_str();
        println!("[{command}]");
        if command.starts_with("shutdown") {
            let _ = stream.shutdown(Shutdown::Both);
            break;
        } else if command.starts_with("send") {
            let whitespace = command.find(' ').unwrap();
            let count = command[whitespace + 1..command.len()].parse::<usize>().unwrap();
            for _ in 0..count {
                let packet = FileOfferPacket::new(55555, "file1.txt".to_string());
                stream.write(&packet.parcel()).unwrap();
            }
        } else if command.starts_with("receive") {
            let whitespace = command.find(' ').unwrap();
            let count = command[whitespace + 1..command.len()].parse::<usize>().unwrap();
            for _ in 0..count {
                read_and_handle_packet(&mut stream);
            }
        } else if command.starts_with("speedtest in") {
            let start = Instant::now();
            for _ in 0..SPEEDTEST_TRANSFERS {
                read_and_handle_packet(&mut stream);
            }
            let elapsed = start.elapsed();
            let megabytes = SPEEDTEST_TRANSFERS as f64;
            let seconds = elapsed.as_secs() as f64;
            println!("Time taken: {:?}", elapsed);
            println!("Speed: {:.2} MB/s", megabytes/seconds);
        } else if command.starts_with("speedtest out") {
            println!("Preparing to send {SPEEDTEST_TRANSFERS} packets of size = {MB_1}");
            let mut payload = vec![0u8; MB_1];
            for i in 0..MB_1 {
                payload[i] = i as u8;
            }
            let bytes = SpeedPacket::new(payload).parcel();
            println!("Starting..");
            let start = Instant::now();
            for _ in 0..SPEEDTEST_TRANSFERS {
                stream.write(&bytes).unwrap();
            };
            let elapsed = start.elapsed();
            let megabytes = SPEEDTEST_TRANSFERS as f64;
            let seconds = elapsed.as_secs() as f64;
            println!("Time taken: {:?}", elapsed);
            println!("Speed: {:.2} MB/s", megabytes/seconds);
        }
    }

    /*let packet2 = FileOfferPacket::new(55556, "file2.txt".to_string());
    let packet = FilePacket::new(0, 7, vec![7,6,5,4,3,2,1]);

    stream.write(&packet2.parcel()).unwrap();
    stream.write(&packet.parcel()).unwrap();*/

}

fn read_and_handle_packet(stream: &mut TcpStream) {
    let mut id = [0u8; 4];
    if !read_to_buffer(&mut id, stream) {
        eprintln!("Connection closed abruptly");
        return;
    }
    let id = packet::read_id(id);

    let mut packet_size = [0u8; 4];
    if !read_to_buffer(&mut packet_size, stream) {
        eprintln!("Connection closed abruptly");
    }
    let packet_size = packet::read_content_size(packet_size);

    let mut field_buffer = vec![0u8; packet_size as usize];
    if !read_to_buffer(&mut field_buffer, stream) {
        eprintln!("Connection closed abruptly");
    }
    match id {
        FileOfferPacket::ID => {
            let construct_res = FileOfferPacket::construct_packet(&field_buffer);
            match construct_res {
                Ok(packet) => {
                    println!("TransferOfferPacket {} {}", packet.file_size, packet.file_name)
                }
                Err(err) => {
                    eprintln!("Failure {err}");
                }
            }

        }
        FilePacket::ID => {
            let construct_res = FilePacket::construct_packet(&field_buffer);
            match construct_res {
                Ok(packet) => {
                    println!("FilePacket {} {} {:?}", packet.chunk_id, packet.payload_size, packet.file_bytes)
                }
                Err(err) => {
                    eprintln!("Failure {err}");
                }
            }
        }
        AnswerPacket::ID => {
            let construct_res = AnswerPacket::construct_packet(&field_buffer);
            match construct_res {
                Ok(packet) => {
                    println!("AnswerPacket {}", packet.yes)
                }
                Err(err) => {
                    eprintln!("Failure {err}");
                }
            }
        }
        SpeedPacket::ID => {
            // don't construct packet - waste of time
        }
        _ => {
            println!("Unrecognized packet {id}");
            return
        }
    }
}

type Failed = bool;
fn read_to_buffer(buffer: &mut [u8], stream: &mut TcpStream) -> Failed {
    return match stream.read_exact(buffer) {
        Ok(bytes_read) => true,
        Err(e) => {
            eprintln!("Failed to read packet fields: {e}");
            false
        }
    }
}

fn read_line() -> String {
    let mut buffer = String::new();
    return match std::io::stdin().read_line(&mut buffer) {
        Ok(_) => buffer.trim_end().to_string(),
        Err(_) => "".into(),
    }
}