use std::net::TcpStream;
use std::time::Instant;
use crate::packet::{Packet, SpeedPacket};
use crate::read_and_handle_packet;

const SPEEDTEST_TRANSFERS: usize = 100;
const IGNORE_FIRST_COUNT: usize = 5;
const MB_1: usize = 1048576;

pub fn speedtest_out(mut stream: &mut TcpStream) {
    println!("Preparing to send {SPEEDTEST_TRANSFERS} packets of size = {MB_1}");
    let mut payload = vec![0u8; MB_1];
    for i in 0..MB_1 {
        payload[i] = i as u8;
    }
    let packet = SpeedPacket::wrap(&payload).unwrap();
    println!("Starting..");
    let test_start = Instant::now();
    let mut start = Instant::now();
    for i in 0..SPEEDTEST_TRANSFERS {
        if i == IGNORE_FIRST_COUNT {
            start = Instant::now();
        }

        packet.write_header(&mut stream);
        packet.write(&mut stream);

        if i < IGNORE_FIRST_COUNT {
            continue
        }

        let elapsed = start.elapsed();
        let megabytes = (i + 1 - IGNORE_FIRST_COUNT) as f64;
        let seconds = elapsed.as_millis() as f64 / 1000f64;
        println!("Written {}/{SPEEDTEST_TRANSFERS} packets ({:.2} MB/s)", i + 1, megabytes / seconds);
    };
    display_final_result(test_start)
}

pub fn speedtest_in(mut stream: &mut TcpStream) {
    let test_start = Instant::now();
    let mut start = Instant::now();
    for i in 0..SPEEDTEST_TRANSFERS {
        if i == IGNORE_FIRST_COUNT {
            start = Instant::now();
        }

        read_and_handle_packet(&mut stream);

        if i < IGNORE_FIRST_COUNT {
            continue
        }

        let elapsed = start.elapsed();
        let megabytes = (i + 1 - IGNORE_FIRST_COUNT) as f64;
        let seconds = elapsed.as_millis() as f64 / 1000f64;
        println!("Received {}/{SPEEDTEST_TRANSFERS} packets ({:.2} MB/s)", i + 1, megabytes / seconds);
    }
    display_final_result(test_start)
}

pub fn display_final_result(start: Instant) {
    let elapsed = start.elapsed();
    let megabytes = SPEEDTEST_TRANSFERS as f64;
    let seconds = elapsed.as_millis() as f64 / 1000f64;
    println!("Time taken: {:?}", elapsed);
    println!("Speed: {:.2} MB/s", megabytes / seconds);
}