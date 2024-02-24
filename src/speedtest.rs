use std::net::TcpStream;
use std::time::Instant;
use crate::packet::{Packet, SpeedPacket};
use crate::read_and_handle_packet;

// Now all parameters can be changed
const SPEEDTEST_TRANSFERS: usize = 100;
const SPEED_PACKET_SIZE: usize = KB_512;
const WARMUP_PACKETS: usize = 3;

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
    println!("megabytes_in_packet: {megabytes_in_packet}");
    let mut start = Instant::now();
    println!("Warming up with {WARMUP_PACKETS} packets");
    for _ in 0..WARMUP_PACKETS {
        packet.write_header(&mut stream);
        packet.write(&mut stream);
    }
    let seconds = start.elapsed().as_secs_f64();
    println!("Warmup time: {seconds:.2}s. Starting..");

    start = Instant::now();
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
    println!("megabytes_in_packet: {megabytes_in_packet}");
    let mut start = Instant::now();
    println!("Warming up with {WARMUP_PACKETS} packets");
    for _ in 0..WARMUP_PACKETS {
        read_and_handle_packet(&mut stream);
    }
    let seconds = start.elapsed().as_secs_f64();
    println!("Warmup time: {seconds:.2}s. Starting..");

    start = Instant::now();
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
