use std::io::{Read, Write};
use std::net::TcpStream;
use crate::args::{CONNECT, ProgramArgs, SERVER};
use crate::config::Config;
use crate::packet::{FIELD_OFFSET, FileInfoPacket, FilePacket, Packet};

mod connection;
mod config;
mod file_operator;
mod packet;
mod args;

fn main() {

    check_file_packet();
    check_file_info_packet();
    let config = Config::read_config();
    // fileserver -> fs
    // SETUP: fileserver serve / fileserver connect
    // EXCHANGE: share path / accept (id)
    let program_args = ProgramArgs::retrieve();
    if !program_args.has_args() {
        ProgramArgs::print_info();
        return;
    }
    println!("ARGS: {:?}", program_args.args);
    let PORT = 2152;
    if program_args.args[0] == SERVER {
        // setup server
        println!("Running server");
        let bind_res = connection::receive_connection_at_port("localhost", PORT);
        let Ok(mut stream) = bind_res else {
            eprintln!("Failed to bind: {}", bind_res.unwrap_err());
            return;
        };
        let port = stream.local_addr().unwrap().port();
        println!("Port assigned {port}");
        write_data(b"Hello!", &mut stream);
    } else if program_args.args[0] == CONNECT {
        println!("Attempting connection");
        let connection_res = connection::connect_to_localhost(PORT);
        let Ok(mut stream) = connection_res else {
            eprintln!("Failed to connect: {}", connection_res.unwrap_err());
            return;
        };
        println!("Connected!");
        read_data(&mut stream);
    }
}

fn write_data(data: &[u8], stream: &mut TcpStream) {
    stream.write(data).unwrap();
}

// try: stream.read_to_end() see if it buffers in disk
fn read_data(stream: &mut TcpStream) {
    let mut data = [0u8; 10];

    match stream.read(&mut data) {
        Ok(_) => {
            let text = String::from_utf8(data.to_vec()).unwrap();
            println!("Reply: {}", text);
        }
        Err(e) => {
            println!("Failed to receive data: {}", e);
        }
    }
}

fn check_file_packet() {
    let file_packet = FilePacket::new(0, 12, vec![1,2,3,4,5,6,7,8,9,10,11,12]);
    println!("TO TRANSPORT: {} {} {:?}", file_packet.chunk_id, file_packet.payload_size, file_packet.file_bytes);
    let parcel = file_packet.parcel();
    println!("Parcel {:?}", parcel);
    let field_bytes = &parcel[FIELD_OFFSET..parcel.len()];
    let constructed = FilePacket::construct_packet(field_bytes).expect("Failed to construct FilePacket packet");
    println!("Construct: {} {} {:?}", constructed.chunk_id, constructed.payload_size, constructed.file_bytes);
    return;
}
fn check_file_info_packet() {
    let file_info = FileInfoPacket::new(313, "eefegeg.dą".into());
    println!("ORIGINAL: {} {} ", file_info.file_size, file_info.file_name);
    let parcel = file_info.parcel();
    println!("Parcel {:?}", parcel);
    let field_bytes = &parcel[FIELD_OFFSET..parcel.len()];
    let received = FileInfoPacket::construct_packet(field_bytes).expect("Failed to construct FileInfoPacket packet");
    println!("RECEIVED: {} {} ", received.file_size, received.file_name);
    return;
}