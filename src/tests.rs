use std::fs::File;
use std::io::{Read};
use std::net::{IpAddr, Ipv4Addr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::thread;
use std::time::{Duration, Instant};
use crate::file_operator::FileFeeder;
use crate::packet;
use crate::packet::{FileOfferPacket, FilePacket, MB_1, MB_2, Packet, PingPacket, SpeedPacket};

fn new_tcp_connection(port: u16) -> (TcpStream, TcpStream) {
    let addr = format!("127.0.0.1:{port}");
    let thread_handle = thread::spawn(move || {
        TcpListener::bind(addr).unwrap()
    });
    let ip = IpAddr::from(Ipv4Addr::new(127, 0, 0, 1));
    let socket = SocketAddr::new(ip, port);
    let timeout = Duration::from_secs(5);
    let connect_st = Instant::now();
    let client_tcp = TcpStream::connect_timeout(&socket, timeout).unwrap();
    println!("Time taken to connect: {:?}", connect_st.elapsed());
    let server = thread_handle.join().unwrap();
    (client_tcp, server.accept().unwrap().0)
}

#[test]
fn file_packet_test() {
    let (mut writer, mut reader) = new_tcp_connection(39993);
    let start = Instant::now();
    let content = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
    let original_packet = FilePacket::new(3, content.len() as u64, &content);
    original_packet.write(&mut writer);

    let original_size = original_packet.size() as usize;
    let mut buffer = vec![0u8; original_size];
    packet::tcp_read_safe(&mut buffer, &mut reader);
    println!("Buffer {:?}", buffer);
    let wrapped_packet = FilePacket::wrap(&buffer)
        .expect("Failed to construct FilePacket packet");

    println!("Logic time: {:?}", start.elapsed());
    assert_eq!(original_packet.transaction_id, wrapped_packet.transaction_id);
    assert_eq!(original_packet.chunk_id, wrapped_packet.chunk_id);
    assert_eq!(original_packet.file_bytes, wrapped_packet.file_bytes);
    close_sockets(writer, reader);
}

fn close_sockets(stream1: TcpStream, stream2: TcpStream) {
    let _ = stream1.shutdown(Shutdown::Both);
    let _ = stream2.shutdown(Shutdown::Both);
}

#[test]
fn transfer_offer_test() {
    let (mut writer, mut reader) = new_tcp_connection(39994);
    let start = Instant::now();
    let original_packet = FileOfferPacket::new(133, 313, "àáąâãäå.zip".into());
    original_packet.write(&mut writer);
    let declared_size = original_packet.size();

    let mut buffer = vec![0u8; declared_size as usize];
    packet::tcp_read_safe(&mut buffer, &mut reader);

    let constructed = FileOfferPacket::construct(&buffer)
        .expect("Failed to construct FileInfoPacket packet");
    println!("Logic time: {:?}", start.elapsed());
    assert_eq!(original_packet.file_size, constructed.file_size);
    assert_eq!(original_packet.file_name, constructed.file_name);
    close_sockets(writer, reader);
}

#[test]
fn speed_packet_test() {
    let (mut writer, mut reader) = new_tcp_connection(39995);
    let start = Instant::now();
    let data = vec![1,2,3,4,5,6];
    let original = SpeedPacket::new(&data);
    original.write(&mut writer);

    let mut buffer = vec![0u8; 6];
    packet::tcp_read_safe(&mut buffer, &mut reader);

    let constructed = SpeedPacket::wrap(&buffer).expect("Failed to construct SpeedPacket packet");
    println!("Logic time: {:?}", start.elapsed());
    assert_eq!(original.random_bytes, constructed.random_bytes);
    close_sockets(writer, reader);
}

#[test]
fn ping_packet_test() {
    let (mut writer, mut reader) = new_tcp_connection(39995);
    let ping = PingPacket::new_ping();
    ping.write_header(&mut writer);
    ping.write(&mut writer);

    let id = packet::read_id(&mut reader);
    if id != PingPacket::ID {
        assert!(false)
    }
    let content_size = packet::read_content_size(&mut reader);

    let field_bytes = vec![0u8; content_size as usize];

    let ping_received = PingPacket::millis_taken(&field_bytes);
    assert!(ping_received >= 0);
    close_sockets(writer, reader);
}

#[test]
fn file_test() {
    let path = "Cargo.toml";
    let mut orig_file = File::open(path).unwrap();
    let length = orig_file.metadata().unwrap().len();
    let mut orig_buffer = vec![0u8; length as usize];
    orig_file.read_exact(&mut orig_buffer).unwrap();
    drop(orig_file);

    let mut feeder = FileFeeder::new(path, MB_1).expect("Where is file?");
    let mut feeder_buffer = vec![];
    while feeder.has_next_chunk() {
        match feeder.read_next_chunk() {
            Ok(chunk) => feeder_buffer.extend_from_slice(chunk),
            Err(err) => eprintln!("{err}")
        }
    }
    assert_eq!(feeder_buffer, orig_buffer)
}

#[test]
fn time() {
    println!("{}", packet::epoch_time_now())
}


