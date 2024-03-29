use std::io::Result;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(20);

pub fn connect_to_localhost(port: u16) -> Result<TcpStream> {
    let address = &format!("localhost:{port}");
    // How to convert localhost to IP for a connection with a timeout?
    TcpStream::connect(address)
}
pub fn connect_to_address(port: u16) -> Result<TcpStream> {
    let address = &format!("localhost:{port}");
    // How to convert localhost to IP for a connection with a timeout?
    TcpStream::connect(address)
}

pub(crate) fn connect_ipv4(server: &str, port: u16) -> Result<TcpStream> {
    let socket = create_ipv4_socket(server, port);
    TcpStream::connect_timeout(&socket, TIMEOUT)
}

pub fn receive_connection(address: &str) -> Result<TcpStream> {
    // This will request the OS to assign a port that's available
    receive_connection_at_port(address, 0)
}

// Return available and valid connections at the time of the call
pub fn receive_connection_at_port(address: &str, port: u16) -> Result<TcpStream> {
    let full_address = &format!("{address}:{port}");
    // Binding with timeout?
    let listener = TcpListener::bind(full_address).expect("Couldn't connect");
    listener.incoming().next().unwrap()
}

pub fn create_server(address: &str, port: u16) -> TcpListener {
    let full_address = &format!("{address}:{port}");
    // Binding with timeout?
    TcpListener::bind(full_address).expect("Failed to create server - address invalid")
}

fn create_ipv4_socket(address: &str, port: u16) -> SocketAddr {
    let octets = ipv4_address_to_byte_vec(address);
    let ipv4 = Ipv4Addr::from(octets);
    SocketAddr::new(IpAddr::V4(ipv4), port)
}

pub fn ipv4_address_to_byte_vec(address: &str) -> [u8; 4] {
    let mut octets = [0u8; 4];
    let components = address.split('.');
    for (i, comp) in components.enumerate() {
        let Ok(byte) = comp.parse::<u8>() else {
            panic!("Error: The address {address} is invalid")
        };
        octets[i] = byte;
    }
    octets
}