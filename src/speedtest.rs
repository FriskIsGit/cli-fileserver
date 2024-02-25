use std::net::TcpStream;
use std::thread::sleep;
use std::time::{Duration, Instant};
use crate::packet::{Packet, PingPacket, SpeedPacket, SpeedtestInfoPacket};
use crate::{packet, read_and_handle_packet};

// Now all parameters can be changed
const SPEEDTEST_TRANSFERS: usize = 100;
const SPEED_PACKET_SIZE: usize = MB_1;

const KB_125: usize = 128000;
const KB_512: usize = 524288; // diez are the most efficient?
const MB_1: usize = 1048576;
const MB_2: usize = 2097152;

pub fn speedtest_out(mut stream: &mut TcpStream) {
    let mut payload = vec![0u8; SPEED_PACKET_SIZE];
    for i in 0..SPEED_PACKET_SIZE {
        payload[i] = i as u8;
    }
    let packet = SpeedPacket::wrap(&payload).unwrap();
    let megabytes_in_packet = SPEED_PACKET_SIZE as f64 / MB_1 as f64;

    println!("Pinging peer..");
    write_ping(stream);
    let ping = read_ping(stream);
    println!("Ping: {ping}ms");
    write_ping(stream);

    let future = read_test_start(stream);
    let until_start = Duration::from_millis(future - packet::epoch_time_now());
    sleep(until_start);
    println!("Woke up at: {}", packet::epoch_time_now());
    let start = Instant::now();
    for i in 1..=SPEEDTEST_TRANSFERS {
        packet.write_header(&mut stream);
        packet.write(&mut stream);

        let elapsed = start.elapsed();
        let seconds = elapsed.as_millis() as f64 / 1000f64;

        let megabytes_transferred = i as f64 * megabytes_in_packet;

        let speed = megabytes_transferred / seconds;
        println!("Written {}/{SPEEDTEST_TRANSFERS} packets ({:.2} MB/s)", i, speed);
    };
    let seconds_elapsed = start.elapsed().as_secs_f64();

    let megabytes_transferred = SPEEDTEST_TRANSFERS as f64 * megabytes_in_packet;
    let speed = megabytes_transferred / seconds_elapsed;
    println!("Upload speed = {speed:.2} MB/s");
}


pub fn speedtest_in(mut stream: &mut TcpStream) {
    let megabytes_in_packet = SPEED_PACKET_SIZE as f64 / MB_1 as f64;

    println!("Awaiting ping..");
    let _ = read_ping(stream);
    write_ping(stream);
    let ping = read_ping(stream);
    println!("Ping: {ping}ms");
    let future = packet::epoch_time_now() + ping + 500;
    write_test_start(stream, future);

    let until_start = Duration::from_millis(future - packet::epoch_time_now());
    sleep(until_start);
    println!("Woke up at: {}", packet::epoch_time_now());
    let start = Instant::now();
    for i in 1..=SPEEDTEST_TRANSFERS {
        read_and_handle_packet(&mut stream);
        let elapsed = start.elapsed();
        let seconds = elapsed.as_millis() as f64 / 1000f64;

        let megabytes_transferred = i as f64 * megabytes_in_packet;
        let speed = megabytes_transferred / seconds;
        println!("Received {}/{SPEEDTEST_TRANSFERS} packets ({:.2} MB/s)", i, speed);
    }
    let seconds_elapsed = start.elapsed().as_secs_f64();

    let megabytes_transferred = SPEEDTEST_TRANSFERS as f64 * megabytes_in_packet;
    let speed = megabytes_transferred / seconds_elapsed;
    println!("Download speed = {speed:.2} MB/s");
}

pub fn write_ping(stream: &mut TcpStream) {
    let ping = PingPacket::new_ping();
    ping.write_header(stream);
    ping.write(stream);
}

fn read_ping(stream: &mut TcpStream) -> u64 {
    let id = packet::read_id(stream);
    let packet_size = packet::read_content_size(stream);

    if id != PingPacket::ID {
        eprintln!("ID {id} wasn't expected at this time");
    }
    let mut field_buffer = vec![0u8; packet_size as usize];
    packet::tcp_read_safe(&mut field_buffer, stream);

    PingPacket::millis_taken(&field_buffer)
}

fn write_test_start(stream: &mut TcpStream, start: u64) {
    let mut start_packet = SpeedtestInfoPacket::new_with_start(start);
    start_packet.write_header(stream);
    start_packet.write(stream);
}

fn read_test_start(stream: &mut TcpStream) -> u64 {
    let id = packet::read_id(stream);
    let packet_size = packet::read_content_size(stream);

    if id != SpeedtestInfoPacket::ID {
        eprintln!("ID {id} wasn't expected at this time");
    }
    let mut field_buffer = vec![0u8; packet_size as usize];
    packet::tcp_read_safe(&mut field_buffer, stream);

    SpeedtestInfoPacket::get_start_time(&field_buffer)
}
