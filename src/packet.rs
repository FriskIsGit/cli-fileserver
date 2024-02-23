/*
PACKET STRUCTURE FORMAT:
    id = packet id
    content size = field bytes length
    content = field bytes streamlined (raw)
     --------------------------------
     |  id | content size | content |
     | u32 |      u32     | Vec<u8> |
     --------------------------------
    packet max size ~ 4.29 GB
*/

use std::io::{Read, Write};
use std::net::TcpStream;

pub const FIELD_OFFSET: usize = 8;

pub trait Packet {
    // Every packet must identify itself
    fn id(&self) -> u32;

    // Every packet must provide its content's size in bytes
    fn size(&self) -> u32;

    // Every packet must serialize itself
    fn write(&self, stream: &mut TcpStream);

    // The default header impl, don't override
    fn write_header(&self, stream: &mut TcpStream) {
        tcp_write_safe(&self.id().to_be_bytes(), stream);
        tcp_write_safe(&self.size().to_be_bytes(), stream);
    }
}


pub fn tcp_write_safe(mut data: &[u8], stream: &mut TcpStream) {
    loop {
        match stream.write(data) {
            Ok(written) => {
                if written == data.len() {
                    return;
                }
                data = &data[written..];
            }
            Err(err) => eprintln!("Encountered error when writing socket: {err}")
        }
    }
}

pub fn tcp_read_safe(mut buffer: &mut [u8], stream: &mut TcpStream) {
    loop {
        match stream.read(buffer) {
            Ok(read) => {
                if read == buffer.len() {
                    return;
                }
                buffer = &mut buffer[read..];
            }
            Err(err) => eprintln!("Encountered error when reading from socket: {err}")
        }
    }
}

pub fn read_id(id_bytes: [u8; 4]) -> u32 {
    u32::from_be_bytes(id_bytes)
}

pub fn read_content_size(packet_size: [u8; 4]) -> u32 {
    u32::from_be_bytes(packet_size)
}

// PACKET STRUCT IMPLEMENTATIONS
pub struct FileOfferPacket {
    pub file_size: u64,
    // in bytes
    pub file_name: String,
}

impl FileOfferPacket {
    pub const ID: u32 = 100_000;
    pub fn new(file_size: u64, file_name: String) -> Self {
        Self { file_size, file_name }
    }
    pub fn construct(field_bytes: &[u8]) -> Result<Self, String> {
        if field_bytes.len() < 9 {
            return Err(format!("Packet has {} bytes but at least 9 were expected", field_bytes.len()));
        }
        let bytes: [u8; 8] = field_bytes[0..8].try_into().unwrap();
        let file_size = u64::from_be_bytes(bytes);

        return match String::from_utf8(field_bytes[8..].to_vec()) {
            Ok(file_name) => Ok(Self::new(file_size, file_name)),
            Err(e) => Err(e.to_string()),
        };
    }
}

impl Packet for FileOfferPacket {
    fn id(&self) -> u32 {
        FileOfferPacket::ID
    }

    fn size(&self) -> u32 {
        (8 + self.file_name.len()) as u32
    }

    fn write(&self, stream: &mut TcpStream) {
        tcp_write_safe(&self.file_size.to_be_bytes(), stream);
        tcp_write_safe(&self.file_name.as_bytes(), stream);
    }
}

// FILE PACKET
pub struct FilePacket<'r> {
    pub chunk_id: u64,
    pub payload_size: u64,
    pub file_bytes: &'r [u8],
}

impl<'r> FilePacket<'r> {
    pub const ID: u32 = 200_000;
    pub fn new(chunk_id: u64, payload_size: u64, content: &'r [u8]) -> Self {
        Self { chunk_id, payload_size, file_bytes: content }
    }
    pub fn wrap(field_bytes: &'r [u8]) -> Result<Self, String> {
        let length = field_bytes.len();
        if length < 16 {
            return Err(format!("Packet has {length} bytes but 16 were expected"));
        }
        let chunk_id_bytes: [u8; 8] = field_bytes[0..8].try_into().unwrap();
        let chunk_id = u64::from_be_bytes(chunk_id_bytes);

        let payload_id_bytes: [u8; 8] = field_bytes[8..16].try_into().unwrap();
        let payload_size = u64::from_be_bytes(payload_id_bytes);

        let file_bytes = &field_bytes[16..length];

        Ok(Self::new(chunk_id, payload_size, file_bytes))
    }
}

impl<'r> Packet for FilePacket<'r> {
    fn id(&self) -> u32 {
        FilePacket::ID
    }

    fn size(&self) -> u32 {
        (8 + 8 + self.file_bytes.len()) as u32
    }

    fn write(&self, stream: &mut TcpStream) {
        tcp_write_safe(&self.chunk_id.to_be_bytes(), stream);
        tcp_write_safe(&self.payload_size.to_be_bytes(), stream);
        tcp_write_safe(&self.file_bytes, stream);
    }
}


// Used for testing purposes
pub struct SpeedPacket<'r> {
    pub random_bytes: &'r [u8],
}

impl<'r> SpeedPacket<'r> {
    pub const ID: u32 = 400_000;
    pub fn new(random_bytes: &'r [u8]) -> Self {
        Self { random_bytes }
    }
    pub fn wrap(field_bytes: &'r [u8]) -> Result<Self, String> where Self: Sized {
        Ok(Self::new(field_bytes))
    }
}

impl<'r> Packet for SpeedPacket<'r> {
    fn id(&self) -> u32 {
        SpeedPacket::ID
    }

    fn size(&self) -> u32 {
        self.random_bytes.len() as u32
    }

    fn write(&self, stream: &mut TcpStream) {
        tcp_write_safe(&self.random_bytes, stream);
    }
}
