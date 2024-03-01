use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};
use std::path::Path;
use std::time::{Instant};
use crate::args::{CONNECT, ProgramArgs, HOST};
use crate::config::Config;
use crate::file_operator::FileFeeder;
use crate::packet::{FileOfferPacket, FilePacket, MB_1, MB_100, Packet, PingPacket, ResponsePacket, SpeedPacket, tcp_write_safe};
use crate::speedtest::{round_trip_time, speedtest_in, speedtest_out};

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
                config.host_address = Some(address);
            }
            if let Some(port) = program_args.port {
                config.host_port = Some(port);
            }
            server_impl(config)
        },
        CONNECT => {
            if let Some(address) = program_args.address {
                config.connect_address = Some(address);
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
    let target_address = config.connect_address.as_ref().unwrap();
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
    if config.host_address.is_none() {
        config.host_address = Some(select_local_ip());
    }

    let host_address = config.host_address.as_ref().unwrap();
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

    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        println!("Targetting linux or android");
        const TARGET_INTERFACES: [(&str, &str); 2] = [("eth", "enp"), ("wlan", "wlp")];
        let interfaces =  local_ip_address::list_afinet_netifas()
            .expect("Failed to retrieve network interfaces, specify host address explicitly.");
        for interface_type in TARGET_INTERFACES {
            for net in interfaces.iter() {
                let ip = net.1;
                if ip.is_loopback() || ip.is_ipv6() || ip.is_unspecified() {
                    continue
                }

                let name = &net.0;
                if name.starts_with(interface_type.0) || name.starts_with(interface_type.1) {
                    return name.to_owned();
                }
            }
        }
        return "127.0.0.1".to_owned();
    }

    #[cfg(target_os = "windows")]
    {
        match local_ip_address::local_ip() {
            Ok(ip) => {
                println!("LOCAL IP: {:?}", ip);
                return ip.to_string();
            }
            Err(err) => panic!("Couldn't assign default ip: {err}")
        }
    }
}

fn established_connection_stage(mut stream: TcpStream) {
    loop {
        println!("[share <path>, read, ping send, ping get, speedtest in, speedtest out, shutdown]");
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
            let file_path = &command[whitespace + 1..];
            let path = Path::new(file_path);
            if !path.exists() {
                eprintln!("File not found");
                continue
            }
            if path.is_dir() {
                eprintln!("File is a directory");
                continue
            }
            let accepted = send_offer_and_read_response(file_path, &mut stream);
            if accepted {
                println!("File was accepted.");
                stream_file(file_path, &mut stream);
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
        }
    }
}

type Accepted = bool;
fn send_offer_and_read_response(file_path: &str, stream: &mut TcpStream) -> Accepted {
    let file = File::open(file_path).expect("File should exist by now");
    let Ok(metadata) = file.metadata() else {
        eprintln!("Cannot read metadata of file at {file_path}");
        return false;
    };
    let file_name = Path::new(file_path).file_name().unwrap().to_str().unwrap();

    let offer = FileOfferPacket::new(1, metadata.len(), file_name.to_string());
    offer.write_header(stream);
    offer.write(stream);
    println!("Sent offer");
    let id = packet::read_id(stream);
    if id != ResponsePacket::ID {
        eprintln!("Response packet was expected");
        return false;
    }
    let packet_size = packet::read_content_size(stream);

    let mut field_buffer = vec![0u8; packet_size as usize];
    packet::tcp_read_safe(&mut field_buffer, stream);

    let response = ResponsePacket::from_bytes(&field_buffer);
    return response.accepted;
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
                    println!("Download {}?  [{}] (y/n)", file_offer.file_name, file_offer.format_size());
                    let ok = read_line().starts_with('y');
                    let response_packet = ResponsePacket::new(1, ok);
                    response_packet.write_header(stream);
                    response_packet.write(stream);

                    if !ok {
                        return;
                    }
                    if let Err(err) = File::create(&file_offer.file_name) {
                        eprintln!("{err}");
                        return;
                    }
                    let mut file = OpenOptions::new()
                        .append(true)
                        .open(file_offer.file_name).unwrap();

                    let size_goal = file_offer.file_size;
                    let mut size_so_far = 0;

                    let mut buffer = vec![0u8; MB_1];
                    let start = Instant::now();
                    while size_so_far < size_goal {
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
                                    Ok(_) => size_so_far += content_len,
                                    Err(err) => println!("Failed to write to file: {err}")
                                }
                                let seconds_so_far = start.elapsed().as_secs_f64();
                                let speed = size_so_far as f64 / MB_1 as f64 / seconds_so_far;
                                let progress = (size_so_far as f64 / size_goal as f64) * 100f64;
                                println!("progress={progress:.2}% | speed={speed:.2}MB/s");
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

fn stream_file(path: &str, stream: &mut TcpStream) {
    let mut file_feeder = FileFeeder::new(path, MB_1).expect("Couldn't initialize file reader");
    let size_goal = file_feeder.file_size();
    let mut bytes_written = 0;
    let mut chunk_id = 0;
    let start = Instant::now();
    while file_feeder.has_next_chunk() {
        let chunk = file_feeder.read_next_chunk().expect("No next chunk");
        let packet = FilePacket::new(1, chunk_id, chunk);
        packet.write_header(stream);
        packet.write(stream);
        chunk_id += 1;
        bytes_written += chunk.len();
        let seconds_so_far = start.elapsed().as_secs_f64();
        let speed = bytes_written as f64 / MB_1 as f64 / seconds_so_far;
        let progress = (bytes_written as f64 / size_goal as f64) * 100f64;
        println!("progress={progress:.2}% | speed={speed:.2}MB/s")
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