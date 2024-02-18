use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(20);

fn connect_local(port: u16) {
    let address = &format!("localhost:{port}");
    let connection_result = TcpStream::connect(address);
    if let Err(err) = connection_result {
        println!("Failed to connect: {err}");
        return;
    }
    successfully_connect(connection_result.unwrap())
}

fn connect_ipv4(server: &str, port: u16) {
    let address = &format!("{server}:{port}");
    let socket = create_ipv4_socket(address, port);
    let connection_result = TcpStream::connect_timeout(&socket, TIMEOUT);
    successfully_connect(connection_result.unwrap())
}

fn successfully_connect(mut stream: TcpStream) {
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
        },
        Err(e) => {
            println!("Failed to receive data: {}", e);
        }
    }
}

fn create_ipv4_socket(address: &str, port: u16) -> SocketAddr {
    let octets = ipv4_address_to_byte_vec(address);
    let ipv4 = Ipv4Addr::from(octets);
    let socket = SocketAddr::new(IpAddr::V4(ipv4), port);;
    socket
}


pub fn ipv4_address_to_byte_vec(address: &str) -> [u8; 4] {
    let mut octets = [0u8; 4];
    let components = address.split('.');
    let mut i = 0;
    for comp in components {
        let Ok(byte) = comp.parse::<u8>() else {
            panic!("Error: The address {address} is invalid")
        };
        octets[i] = byte;
        i += 1;
    }
    octets
}