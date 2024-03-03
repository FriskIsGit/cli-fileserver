use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};
use std::path::Path;
use std::time::{Instant};
use crate::args::{CONNECT, ProgramArgs, HOST};
use crate::config::Config;
use crate::file_operator::FileFeeder;
use crate::packet::{BeginUploadPacket, FileOfferPacket, FilePacket, MB_1, Packet, PingPacket, ResponsePacket, SpeedPacket};
use crate::speedtest::{round_trip_time, speedtest_in, speedtest_out};

mod connection;
mod config;
mod file_operator;
mod packet;
mod args;
mod tests;
mod speedtest;
mod util;

const PINGS: usize = 100;
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
            server_impl(config)
        },
        CONNECT => {
            if let Some(address) = program_args.address {
                config.connect_ip = Some(address);
            }
            if let Some(port) = program_args.port {
                config.connect_port = Some(port);
            }
            client_impl(config)
        },
        _ => {}
    }
}

fn client_impl(config: Config) {
    let target_address = config.connect_ip.as_ref().unwrap();
    let port = config.connect_port.unwrap();
    println!("Attempting connection to {target_address}");
    let mut stream = match connection::connect_ipv4(target_address, port) {
        Ok(tcp_stream) => tcp_stream,
        Err(err) => {
            let err_kind = err.kind();
            eprintln!("Error: \"{err_kind}\" - {err}");
            return;
        }
    };
    println!("Connected!");
    config.apply_timeouts(&mut stream);
    established_connection_stage(stream);
}

fn server_impl(mut config: Config) {
    println!("Setting up server");
    if config.host_ip.is_none() {
        config.host_ip = Some(select_local_ip());
    }

    let host_address = config.host_ip.as_ref().unwrap();
    let port = config.host_port.unwrap();
    let listener = connection::create_server(host_address, port);
    let local_address = listener.local_addr().unwrap();
    println!("Hosting server on {}:{}", local_address.ip(), local_address.port());

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
                config.apply_timeouts(&mut stream);
                established_connection_stage(stream);
                println!("Closed socket, listening for new connections..");
            }
            Err(err) => {
                eprintln!("Failed to accept connection: {err}");
                continue;
            }
        };
    };
}

pub fn read_line() -> String {
    let mut buffer = String::new();
    return match std::io::stdin().read_line(&mut buffer) {
        Ok(_) => buffer.trim_end().to_string(),
        Err(_) => "".into(),
    };
}

// eth or enp - ETHERNET
// wlan or wlp - WIFI
// lo - local


pub fn select_local_ip() -> String {
    match local_ip_address::local_ip() {
        Ok(ip) => {
            println!("LOCAL IP: {:?}", ip);
            return ip.to_string();
        }
        Err(err) => panic!("Couldn't assign default ip: {err}")
    }
}

fn established_connection_stage(mut stream: TcpStream) {
    loop {
        println!("[share <path>, read, test_send, ping send, ping get, speedtest in, speedtest out, shutdown]");
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
            let file_path = command[whitespace + 1..].trim_matches('\"');
            let path = Path::new(file_path);
            if !path.exists() {
                eprintln!("File not found");
                continue
            }
            if path.is_dir() {
                eprintln!("File is a directory");
                continue
            }
            let file_name = Path::new(file_path).file_name().unwrap().to_str().unwrap();
            let upload = send_offer_and_read_response(file_path, &file_name, &mut stream);
            if upload.start {
                println!("File was accepted.");
                stream_file(file_path, upload.cursor, &mut stream);
            } else {
                println!("File denied!");
            }
        } else if command.starts_with("read") {
            // let whitespace = command.find(' ').unwrap();
            // let count = command[whitespace + 1..].parse::<usize>().unwrap();
            // for _ in 0..count { }
            read_and_handle_packet(&mut stream);
        } else if command.starts_with("speedtest in") || command.starts_with("si") {
            speedtest_in(&mut stream);
        } else if command.starts_with("speedtest out") || command.starts_with("so") {
            speedtest_out(&mut stream);
        } else if command.starts_with("ping send") {
            for p in 0..PINGS {
                let rtt = round_trip_time(&mut stream);
                println!("{p}# RTT: {:?}", rtt);
            }
            write_ping(&mut stream);
        } else if command.starts_with("ping get") {
            read_ping(&mut stream);
            for p in 0..PINGS {
                let rtt = round_trip_time(&mut stream);
                println!("{p}# RTT: {:?}", rtt);
            }
        } else if command.starts_with("test_send") {
            write_ping(&mut stream);
        }
    }
}

type Accepted = bool;
fn send_offer_and_read_response(file_path: &str, file_name: &str, stream: &mut TcpStream) -> BeginUploadPacket {
    let file = File::open(file_path).expect("File should exist by now");
    let Ok(metadata) = file.metadata() else {
        eprintln!("Cannot read metadata of file at {file_path}");
        return BeginUploadPacket::deny();
    };

    let offer = FileOfferPacket::new(1, metadata.len(), file_name.to_string());
    offer.write_header(stream);
    offer.write(stream);
    println!("Sent offer");
    let id = packet::read_id(stream);
    if id != BeginUploadPacket::ID {
        eprintln!("Upload information was expected");
        return BeginUploadPacket::deny();
    }
    let packet_size = packet::read_content_size(stream);

    let mut field_buffer = vec![0u8; packet_size as usize];
    packet::tcp_read_safe(&mut field_buffer, stream);

    BeginUploadPacket::from_bytes(&field_buffer)
}

fn read_and_handle_packet(stream: &mut TcpStream) {
    let id = packet::read_id(stream);
    let packet_size = packet::read_content_size(stream);

    let mut field_buffer = vec![0u8; packet_size as usize];
    packet::tcp_read_safe(&mut field_buffer, stream);
    match id {
        FileOfferPacket::ID => {
            match FileOfferPacket::construct(&field_buffer) {
                Ok(file_offer) => {

                    let path = Path::new(&file_offer.file_name);
                    let mut current_size = 0;
                    // Resume download from cursor pos
                    if path.exists() {
                        current_size = path.metadata().unwrap().len();
                        if current_size >= file_offer.file_size {
                            write_denied_packet(stream);
                            eprintln!("Denied offer because current size >= offered");
                            return;
                        }
                        let remaining = util::format_size(file_offer.file_size - current_size);
                        println!("Resume downloading {}? {remaining} remaining (y/n)", file_offer.file_name);
                        let ok = read_line().starts_with('y');
                        if ok {
                            let denied_packet = BeginUploadPacket::accept(file_offer.transaction_id, current_size);
                            denied_packet.write_header(stream);
                            denied_packet.write(stream);
                        } else {
                            write_denied_packet(stream);
                            return;
                        }
                    } else {
                        let offer_size = util::format_size(file_offer.file_size);
                        println!("Download {}?  [{offer_size}] (y/n)", file_offer.file_name);
                        let ok = read_line().starts_with('y');
                        if ok {
                            if let Err(err) = File::create(&file_offer.file_name) {
                                eprintln!("{err}");
                                write_denied_packet(stream);
                                return;
                            }
                            let denied_packet = BeginUploadPacket::accept(file_offer.transaction_id, 0);
                            denied_packet.write_header(stream);
                            denied_packet.write(stream);
                        } else {
                            write_denied_packet(stream);
                            return;
                        }

                    }

                    let mut file = OpenOptions::new()
                        .append(true)
                        .open(file_offer.file_name).unwrap();

                    let mut buffer = vec![0u8; MB_1];
                    let mut bytes_read = 0;
                    let start = Instant::now();
                    while current_size < file_offer.file_size {
                        let id = packet::read_id(stream);
                        if id != FilePacket::ID {
                            eprintln!("{id} wasn't expected at this time");
                            return;
                        }
                        let content_size = packet::read_content_size(stream) as usize;

                        buffer.reserve_exact(content_size - buffer.len());
                        unsafe { buffer.set_len(content_size); }
                        packet::tcp_read_safe(&mut buffer, stream);
                        match FilePacket::wrap(&buffer) {
                            Ok(packet) => {
                                let content_len = packet.file_bytes.len() as u64;

                                match file.write_all(packet.file_bytes) {
                                    Ok(_) => {
                                        current_size += content_len;
                                        bytes_read += content_len;
                                    },
                                    Err(err) => eprintln!("Failed to write to file: {err}")
                                }
                                let seconds_so_far = start.elapsed().as_secs_f64();
                                let speed = bytes_read as f64 / MB_1 as f64 / seconds_so_far;
                                let progress = (current_size as f64 / file_offer.file_size as f64) * 100f64;
                                let eta = util::eta(bytes_read, file_offer.file_size, speed);
                                eprintln!("progress={progress:.2}% ({speed:.2}MB/s) ETA: {eta:?}");
                            }
                            Err(err) => {
                                println!("Error at FilePacket::wrap - {err}");
                            }
                        }
                        buffer.clear();
                    }
                    let elapsed = start.elapsed();
                    println!("Download completed in {:?}", elapsed);

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
        }
    }
}

fn stream_file(path: &str, cursor: u64, stream: &mut TcpStream) {
    let mut file_feeder = FileFeeder::new(path, MB_1).expect("Couldn't initialize file reader");
    file_feeder.set_cursor_pos(cursor);
    let size_goal = file_feeder.file_size();
    let mut cursor = cursor as usize;
    let mut bytes_written = 0u64;
    let mut chunk_id = 0;
    let start = Instant::now();
    while file_feeder.has_next_chunk() {
        let chunk = file_feeder.read_next_chunk().expect("No next chunk");
        let packet = FilePacket::new(1, chunk_id, chunk);
        packet.write_header(stream);
        packet.write(stream);
        chunk_id += 1;
        bytes_written += chunk.len() as u64;
        cursor += chunk.len();
        let seconds_so_far = start.elapsed().as_secs_f64();
        let speed = bytes_written as f64 / MB_1 as f64 / seconds_so_far;
        let progress = (cursor as f64 / size_goal as f64) * 100f64;
        let eta = util::eta(bytes_written, size_goal, speed);
        eprintln!("progress={progress:.2}% ({speed:.2}MB/s) ETA: {eta:?}");
    }

    let elapsed = start.elapsed();
    println!("Upload completed in {:?}", elapsed);
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

pub fn write_denied_packet(stream: &mut TcpStream) {
    let denied_packet = BeginUploadPacket::deny();
    denied_packet.write_header(stream);
    denied_packet.write(stream);
}