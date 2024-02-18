use crate::config::Config;

mod connection;
mod config;
mod file_operator;

fn main() {
    let config = Config::read_config();
    let vec = connection::ipv4_address_to_byte_vec("246.32.43.43");
    println!("Test {:?}", vec);
}
