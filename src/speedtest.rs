use std::io::{Error, ErrorKind};
use std::net::TcpStream;
use rand::Rng;
use std::thread::sleep;
use std::time::{Duration, Instant};
use crate::packet::{MB_1, Packet, PingPacket, SpeedPacket, SpeedtestInfoPacket};
use crate::{packet};

// Now all parameters can be changed
const SPEEDTEST_TRANSFERS: usize = 100;
const SPEED_PACKET_SIZE: usize = MB_1;

// KB_512 are the most efficient?

pub fn speedtest_out(stream: &mut TcpStream) {
    let mut payload = vec![0u8; SPEED_PACKET_SIZE];
    let mut rng = rand::thread_rng();
    for i in 0..SPEED_PACKET_SIZE {
        payload[i] = rng.gen();
    }
    let packet = SpeedPacket::wrap(&payload).unwrap();
    let megabytes_in_packet = SPEED_PACKET_SIZE as f64 / MB_1 as f64;

    println!("Pinging peer..");
    round_trip_time(stream);
    let elapsed = round_trip_time(stream);
    let ping = elapsed.checked_div(2).unwrap();
    println!("Ping: {:?}", ping);

    let _ = read_test_start(stream);
    // begin instantly, peer will sleep for the ping duration

    let start = Instant::now();
    for i in 1..=SPEEDTEST_TRANSFERS {
        if packet.write_header(stream).and(packet.write(stream)).is_err() {
            eprintln!("Speedtest wasn't completed");
            break;
        }

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
    println!("Transferred in {seconds_elapsed:.2}s");
}


pub fn speedtest_in(stream: &mut TcpStream) {
    let megabytes_in_packet = SPEED_PACKET_SIZE as f64 / MB_1 as f64;

    println!("Awaiting ping..");
    read_ping(stream);
    let rtt_elapsed = round_trip_time(stream);
    write_ping(stream);
    let ping = rtt_elapsed.checked_div(2).unwrap();
    println!("Ping: {:?}", ping);

    let future = packet::epoch_time_now() + 300;
    write_test_start(stream, future);
    sleep(ping);

    let start = Instant::now();
    for i in 1..=SPEEDTEST_TRANSFERS {
        if read_speed_packet(stream).is_err() {
            eprintln!("Speedtest wasn't completed");
            break;
        }

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
    println!("Transferred in {seconds_elapsed:.2}s");
}

pub fn write_ping(stream: &mut TcpStream) -> Instant {
    let ping = PingPacket::new_ping();
    let ping_start = Instant::now();
    let _ = ping.write_header(stream);
    let _ = ping.write(stream);
    ping_start
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

type Elapsed = Duration;
fn read_ping_and_measure(stream: &mut TcpStream, ping_start: Instant) -> Elapsed {
    let id = packet::read_id(stream);
    let elapsed = ping_start.elapsed();
    let packet_size = packet::read_content_size(stream);

    if id != PingPacket::ID {
        eprintln!("ID {id} wasn't expected at this time");
    }
    let mut field_buffer = vec![0u8; packet_size as usize];
    let _ = packet::tcp_read_safe(&mut field_buffer, stream);
    elapsed
}

fn write_test_start(stream: &mut TcpStream, start: u64) {
    let start_packet = SpeedtestInfoPacket::new_with_start(start);
    let _ = start_packet.write_header(stream);
    let _ = start_packet.write(stream);
}

fn read_test_start(stream: &mut TcpStream) -> u64 {
    let id = packet::read_id(stream);
    let packet_size = packet::read_content_size(stream);

    if id != SpeedtestInfoPacket::ID {
        eprintln!("ID {id} wasn't expected at this time");
    }
    let mut field_buffer = vec![0u8; packet_size as usize];
    let _ = packet::tcp_read_safe(&mut field_buffer, stream);

    SpeedtestInfoPacket::get_start_time(&field_buffer)
}

// RTT
pub fn round_trip_time(stream: &mut TcpStream) -> Elapsed {
    let ping_start = write_ping(stream);
    read_ping_and_measure(stream, ping_start)
}

pub fn read_speed_packet(stream: &mut TcpStream) -> std::io::Result<()> {
    let id = packet::read_id(stream);
    if id != SpeedPacket::ID {
        let err = Error::new(ErrorKind::InvalidData, "Unexpected packet ID");
        return Err(err);
    }
    let packet_size = packet::read_content_size(stream);

    let mut field_buffer = vec![0u8; packet_size as usize];
    packet::tcp_read_safe(&mut field_buffer, stream)
}
