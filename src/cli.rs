use std::collections::{HashMap};
use std::fs::{File, OpenOptions};
use std::io::{Write};
use std::net::{Shutdown, TcpStream};
use std::path::Path;
use std::time::{Instant};
use crate::config::Config;
use crate::{connection, packet, util};
use crate::file_operator::FileFeeder;
use crate::packet::{ BeginUploadPacket, DirectoryOfferPacket, FileOfferPacket, FilePacket, MB_1, Packet, PingPacket, SpeedPacket};
use crate::speedtest::{round_trip_time, speedtest_in, speedtest_out};

pub fn client_impl(config: Config) {
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

pub fn server_impl(mut config: Config) {
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
                    if !util::read_line().starts_with('y') {
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

const PINGS: usize = 100;

fn established_connection_stage(mut stream: TcpStream) {
    loop {
        println!("[share <path>, read, rtt 1, rtt 2, speedtest in, speedtest out, shutdown, test_send]");
        let line = util::read_line();
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
            share_file_or_directory(file_path, &mut stream);

        } else if command.starts_with("read") {
            read_and_handle_packet(&mut stream);
        } else if command.starts_with("speedtest in") || command.starts_with("si") {
            speedtest_in(&mut stream);
        } else if command.starts_with("speedtest out") || command.starts_with("so") {
            speedtest_out(&mut stream);
        } else if command.starts_with("rtt 1") {
            for p in 0..PINGS {
                let rtt = round_trip_time(&mut stream);
                println!("{p}# RTT: {:?}", rtt);
            }
            write_ping(&mut stream);
        } else if command.starts_with("rtt 2") {
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

pub fn share_file_or_directory(shared_path: &str, stream: &mut TcpStream) {
    let path = Path::new(shared_path);
    if !path.exists() {
        eprintln!("File or directory not found!");
        return
    }
    if path.is_dir() {
        let dir_offer = DirectoryOfferPacket::new(shared_path);
        if dir_offer.file_count == 0 {
            println!("No files found");
            return;
        }
        let _ = dir_offer.write_header(stream);
        let _ = dir_offer.write(stream);
        println!("Offered {} files.", dir_offer.file_count);

        let id = packet::read_id(stream);
        if id != BeginUploadPacket::ID {
            eprintln!("Unexpected packet ID={id}");
            return;
        }
        let packet_size = packet::read_content_size(stream);
        let buffer: Vec<u8> = packet::read_into_new_buffer(stream, packet_size);
        let upload = BeginUploadPacket::from_bytes(&buffer);

        if !upload.has_any_files() {
            println!("Directory upload was cancelled!");
            return;
        }
        println!("Directory was accepted.");

        for (i, index) in upload.file_indexes.iter().enumerate() {
            let file_shared = &dir_offer.files[*index as usize];
            let cursor = upload.cursors[i];
            let relative_path = path.join(&file_shared.name);
            let path_str = relative_path.to_str().unwrap();
            stream_file(path_str, cursor, stream);
        }

        return;
    }

    let file_name = util::get_path_name(shared_path);
    let upload = offer_file(shared_path, file_name, stream);
    if upload.has_any_files() {
        println!("File was accepted.");
        stream_file(shared_path, upload.cursors[0], stream);
    } else {
        println!("File denied!");
    }
}

fn offer_file(file_path: &str, file_name: &str, stream: &mut TcpStream) -> BeginUploadPacket {
    let file = File::open(file_path).expect("File should exist by now");
    let Ok(metadata) = file.metadata() else {
        eprintln!("Cannot read metadata of file at {file_path}");
        return BeginUploadPacket::new_empty();
    };

    let offer = FileOfferPacket::new(1, metadata.len(), file_name.to_string());
    let _ = offer.write_header(stream);
    let _ = offer.write(stream);

    println!("Offered {file_name} file");
    let id = packet::read_id(stream);
    if id != BeginUploadPacket::ID {
        eprintln!("Upload information was expected");
        return BeginUploadPacket::new_empty();
    }
    let packet_size = packet::read_content_size(stream);
    let field_buffer = packet::read_into_new_buffer(stream, packet_size);

    BeginUploadPacket::from_bytes(&field_buffer)
}

fn read_and_handle_packet(stream: &mut TcpStream) {
    let id = packet::read_id(stream);
    let packet_size = packet::read_content_size(stream);
    let field_buffer = packet::read_into_new_buffer(stream, packet_size);

    match id {
        FileOfferPacket::ID => {
            let file_offer = match FileOfferPacket::construct(&field_buffer) {
                Ok(fo) => fo,
                Err(err) => {
                    eprintln!("Failure: {err}");
                    return;
                }
            };

            receive_file(file_offer, stream);
        }
        DirectoryOfferPacket::ID => {
            let dir_offer = DirectoryOfferPacket::from_bytes(&field_buffer);
            receive_directory(dir_offer, stream);
        }
        FilePacket::ID => {
            match FilePacket::wrap(&field_buffer) {
                Ok(packet) => {
                    let transaction_id = packet.transaction_id;
                    let chunk = packet.chunk_id;
                    let content_len = packet.file_bytes.len();
                    println!("File packet: chunk={chunk} content_len={content_len} | transaction_id:{transaction_id}")
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

fn receive_directory(offer: DirectoryOfferPacket, stream: &mut TcpStream) {
    let total_size = util::format_size(offer.total_size);
    println!("Download {} files to {}?  [{total_size}] (y/n)", offer.file_count, offer.directory_name);
    if !util::read_line().starts_with('y') {
        write_denied_packet(stream);
        return;
    }

    let dir_path = Path::new(&offer.directory_name);

    let upload = if dir_path.exists() {
        // Mark cursor positions per file
        let Ok(fs_entries) = std::fs::read_dir(dir_path) else {
            eprintln!("Cannot read directory, aborting");
            write_denied_packet(stream);
            return;
        };

        let mut fs_names: HashMap<String, u64> = HashMap::with_capacity(offer.file_count as usize);

        for maybe_entry in fs_entries {
            let Ok(entry) = maybe_entry else {
                continue;
            };
            let path = entry.path();
            if !path.is_file() {
                continue
            }
            let entry_name = entry.file_name().to_str().unwrap().to_string();
            let Ok(metadata) = path.metadata() else {
                eprintln!("Skipping {entry_name}, unable to retrieve metadata");
                continue
            };
            let size = metadata.len();
            fs_names.insert(entry_name, size);
        }

        let mut file_indexes: Vec<u32> = vec![];
        let mut cursors: Vec<u64> = vec![];

        for (i, file) in offer.files.iter().enumerate() {
            let Some(size) = fs_names.get(&file.name) else {
                file_indexes.push(i as u32);
                cursors.push(0);
                continue;
            };
            if *size < file.size {
                file_indexes.push(i as u32);
                cursors.push(*size);
            }
        }
        let accepted_upload = BeginUploadPacket::new(1, file_indexes, cursors);
        let _ = accepted_upload.write_header(stream);
        let _ = accepted_upload.write(stream);
        accepted_upload
    } else {
        match std::fs::create_dir(&offer.directory_name) {
            Ok(_) => println!("Directory created"),
            Err(err) => {
                eprintln!("{err}");
                return;
            },
        };
        let accepted_upload = BeginUploadPacket::accept_all(1, offer.file_count);
        let _ = accepted_upload.write_header(stream);
        let _ = accepted_upload.write(stream);
        accepted_upload
    };

    if !upload.has_any_files() {
        println!("No files were accepted");
        return;
    }

    println!("Accepting {} out of {} files", upload.files_accepted, offer.file_count);
    for (i, index) in upload.file_indexes.iter().enumerate() {
        let file_offered = &offer.files[*index as usize];
        let current_size = upload.cursors[i];
        let relative_path = Path::new(&offer.directory_name).join(&file_offered.name);
        let dest_file = if current_size > 0 {
            OpenOptions::new().append(true).open(relative_path).unwrap()
        } else {
            File::create(relative_path).expect("Failed to create destination file!")
        };
        read_and_write_file_to_disk(current_size, file_offered.size, dest_file, stream);
        println!("Received {}/{} files", i+1, offer.file_count);
    }
    println!("Downloads:");
    for file in offer.files {
        println!("{} [{}]", file.name, util::format_size(file.size));
    }

}

fn receive_file(file_offer: FileOfferPacket, stream: &mut TcpStream) {
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
        if util::read_line().starts_with('y') {
            let accept_upload = BeginUploadPacket::single_file(file_offer.transaction_id, current_size);
            let _ = accept_upload.write_header(stream);
            let _ = accept_upload.write(stream);
        } else {
            write_denied_packet(stream);
            return;
        }
    } else {
        let offer_size = util::format_size(file_offer.file_size);
        println!("Download {}?  [{offer_size}] (y/n)", file_offer.file_name);
        if util::read_line().starts_with('y') {
            if let Err(err) = File::create(&file_offer.file_name) {
                eprintln!("{err}");
                write_denied_packet(stream);
                return;
            }
            let accept_upload = BeginUploadPacket::single_file(file_offer.transaction_id, current_size);
            let _ = accept_upload.write_header(stream);
            let _ = accept_upload.write(stream);
        } else {
            write_denied_packet(stream);
            return;
        }
    }
    let file = OpenOptions::new()
        .append(true)
        .open(file_offer.file_name).unwrap();
    read_and_write_file_to_disk(current_size, file_offer.file_size, file, stream);
}

fn read_and_write_file_to_disk(mut current_size: u64, total_size: u64, mut file: File, stream: &mut TcpStream) {
    let mut buffer = vec![0u8; MB_1];
    let mut bytes_read = 0;
    let mut expected_chunk_id = 0;
    // Begin reading file packets
    let start = Instant::now();
    while current_size < total_size {
        let id = packet::read_id(stream);
        if id != FilePacket::ID {
            eprintln!("{id} wasn't expected at this time");
            return;
        }
        let content_size = packet::read_content_size(stream) as usize;
        if content_size > buffer.len() {
            buffer.reserve_exact(content_size - buffer.len());
        }
        unsafe { buffer.set_len(content_size); }
        if packet::tcp_read_safe(&mut buffer, stream).is_err() {
            eprintln!("Terminating read since buffer couldn't be filled");
            let _ = stream.shutdown(Shutdown::Read);
            return;
        }
        let packet = match FilePacket::wrap(&buffer) {
            Ok(file_packet) => file_packet,
            Err(err) => {
                eprintln!("Error at FilePacket::wrap - {err}");
                eprintln!("Terminating read since packet was made incorrectly..");
                let _ = stream.shutdown(Shutdown::Read);
                return;
            }
        };

        if packet.chunk_id != expected_chunk_id {
            eprintln!("Terminating read to avoid file corruption (packet was skipped)");
            let _ = stream.shutdown(Shutdown::Read);
            return;
        }

        let content_len = packet.file_bytes.len() as u64;
        while let Err(err) = file.write_all(packet.file_bytes) {
            eprintln!("Failed to write to file: {err}")
        }
        current_size += content_len;
        bytes_read += content_len;
        expected_chunk_id += 1;
        let seconds_so_far = start.elapsed().as_secs_f64();
        let speed = bytes_read as f64 / MB_1 as f64 / seconds_so_far;
        let progress = (current_size as f64 / total_size as f64) * 100.0;
        let eta = util::format_eta(current_size, total_size, speed);
        eprintln!("progress={progress:.2}% ({speed:.2}MB/s) ETA: {eta}");
        buffer.clear();
    }
    let elapsed = start.elapsed().as_secs_f64();
    let time_format = util::format_time(elapsed);
    println!("Download completed in {time_format}");
}

fn stream_file(path: &str, mut cursor: u64, stream: &mut TcpStream) {
    let mut file_feeder = FileFeeder::new(path, MB_1).expect("Couldn't initialize file reader");
    file_feeder.set_cursor_pos(cursor);
    let size_goal = file_feeder.file_size();
    let mut bytes_written: u64 = 0;
    let mut chunk_id = 0;
    let start = Instant::now();
    while file_feeder.has_next_chunk() {
        let chunk = file_feeder.read_next_chunk().expect("No next chunk");
        let packet = FilePacket::new(1, chunk_id, chunk);
        if packet.write_header(stream).and(packet.write(stream)).is_err() {
            println!("Upload couldn't complete");
            break;
        }

        chunk_id += 1;
        bytes_written += chunk.len() as u64;
        cursor += chunk.len() as u64;
        let seconds_so_far = start.elapsed().as_secs_f64();
        let speed = bytes_written as f64 / MB_1 as f64 / seconds_so_far;
        let progress = (cursor as f64 / size_goal as f64) * 100.0;
        let eta = util::format_eta(cursor, size_goal, speed);
        eprintln!("progress={progress:.2}% ({speed:.2}MB/s) ETA: {eta}");
    }

    let elapsed = start.elapsed().as_secs_f64();
    let time_format = util::format_time(elapsed);
    println!("Upload completed in {time_format}");
}

pub fn write_ping(stream: &mut TcpStream) {
    let ping = PingPacket::new_ping();
    let _ = ping.write_header(stream);
    let _ = ping.write(stream);
}

fn read_ping(stream: &mut TcpStream) {
    let id = packet::read_id(stream);
    let packet_size = packet::read_content_size(stream);

    if id != PingPacket::ID {
        eprintln!("ID {id} wasn't expected at this time");
    }
    let mut field_buffer = vec![0u8; packet_size as usize];
    let _ = packet::tcp_read_safe(&mut field_buffer, stream);
}

pub fn write_denied_packet(stream: &mut TcpStream) {
    let denied_packet = BeginUploadPacket::new_empty();
    let _ = denied_packet.write_header(stream);
    let _ = denied_packet.write(stream);
}