use std::io::{Read, Write};
use std::net::TcpStream;
use crate::args::{CONNECT, ProgramArgs, HOST};
use crate::config::Config;
use crate::packet::{FIELD_OFFSET, TransferOfferPacket, FilePacket, Packet, AnswerPacket};

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
    println!("Connected!");
    read_and_handle_packet(&mut stream);
    read_and_handle_packet(&mut stream);
}

fn server_impl(config: Config) {
    println!("Running server");
    let host_address = &config.host_address.unwrap();
    let port = config.host_port.unwrap();
    let bind_res = connection::receive_connection_at_port(host_address, port);
    let Ok(mut stream) = bind_res else {
        eprintln!("Failed to bind: {}", bind_res.unwrap_err());
        return;
    };
    let port = stream.local_addr().unwrap().port();
    println!("Port assigned {port}");
    let packet1 = TransferOfferPacket::new(55555, "file1.txt".to_string());
    let packet2 = TransferOfferPacket::new(55556, "file2.txt".to_string());
    //let packet = FilePacket::new(0, 7, vec![7,6,5,4,3,2,1]);
    stream.write(&packet1.parcel()).unwrap();
    stream.write(&packet2.parcel()).unwrap();
}

fn read_and_handle_packet(stream: &mut TcpStream) {
    let mut id = [0u8; 4];
    read_to_buffer(&mut id, stream);
    let id = packet::read_id(id);

    let mut packet_size = [0u8; 4];
    read_to_buffer(&mut packet_size, stream);
    let packet_size = packet::read_content_size(packet_size);

    let mut field_buffer = vec![0u8; packet_size as usize];
    read_to_buffer(&mut field_buffer, stream);
    match id {
        TransferOfferPacket::ID => {
            let construct_res = TransferOfferPacket::construct_packet(&field_buffer);
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
        _ => {
            println!("Unrecognized packet {id}");
            return
        }
    }
}

fn read_to_buffer(buffer: &mut [u8], stream: &mut TcpStream) {
    match stream.read_exact(buffer) {
        Ok(bytes_read) => {}
        Err(e) => {
            eprintln!("Failed to read packet fields: {e}");
            return;
        }
    }
}