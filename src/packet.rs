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

use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::net::TcpStream;
use std::time::{SystemTime};
use crate::util;

pub const KB_125: usize = 128000;
pub const KB_512: usize = 524288;
pub const MB_1: usize = 1048576;
pub const MB_2: usize = 2097152;
pub const MB_100: usize = 20971520;

pub trait Packet {
    // Every packet must identify itself
    fn id(&self) -> u32;

    // Every packet must provide its content's size in bytes
    fn size(&self) -> u32;

    // Every packet must serialize itself
    fn write(&self, stream: &mut TcpStream) -> Result<(), std::io::Error>;

    // The default header impl, don't override
    fn write_header(&self, stream: &mut TcpStream) -> Result<(), std::io::Error> {
        tcp_write_safe(&self.id().to_be_bytes(), stream)
            .and(tcp_write_safe(&self.size().to_be_bytes(), stream))
    }
}


pub fn tcp_write_safe(mut data: &[u8], stream: &mut TcpStream) -> Result<(), std::io::Error> {
    loop {
        match stream.write(data) {
            Ok(written) => {
                if written == data.len() {
                    return Ok(());
                }
                data = &data[written..];
            }
            Err(err) => {
                let kind = err.kind();
                eprintln!("Error \"{kind}\" occurred when writing to socket - {err}");
                if kind != ErrorKind::Interrupted {
                    // anything other than Interrupted is not salvageable
                    return Err(err);
                }
            }
        }
    }
}

pub fn tcp_read_safe(mut buffer: &mut [u8], stream: &mut TcpStream) -> std::io::Result<()> {
    loop {
        match stream.read(buffer) {
            Ok(read) => {
                if read == buffer.len() {
                    return Ok(());
                }
                buffer = &mut buffer[read..];
            }
            Err(err) => {
                let kind = err.kind();
                eprintln!("Error \"{kind}\" occurred when reading from socket - {err}");
                if kind != ErrorKind::Interrupted {
                    // anything other than Interrupted is not salvageable
                    return Err(err);
                }
            }
        }
    }
}

const DISCARD_BUF_SIZE: usize = 4096;
pub fn tcp_discard_bytes(stream: &mut TcpStream) -> usize {
    let mut buffer = [0u8; DISCARD_BUF_SIZE];
    let mut cleared = 0;
    loop {
        match stream.peek(&mut buffer) {
            Ok(available) => {
                if available == 0 {
                    return cleared;
                }
                let min = std::cmp::min(DISCARD_BUF_SIZE, available);
                if stream.read_exact(&mut buffer[0..min]).is_err() {
                    return cleared;
                }
                cleared += min;
            }
            Err(_) => return cleared,
        };
    }
}

pub fn read_id(stream: &mut TcpStream) -> u32 {
    let mut id_bytes = [0u8; 4];
    let _ = tcp_read_safe(&mut id_bytes, stream);
    u32::from_be_bytes(id_bytes)
}

pub fn read_content_size(stream: &mut TcpStream) -> u32 {
    let mut size_bytes = [0u8; 4];
    let _ = tcp_read_safe(&mut size_bytes, stream);
    u32::from_be_bytes(size_bytes)
}

pub fn read_into_new_buffer(stream: &mut TcpStream, content_size: u32) -> Vec<u8> {
    let mut buffer = vec![0u8; content_size as usize];
    let _ = tcp_read_safe(&mut buffer, stream);
    return buffer;
}

// PACKET STRUCT IMPLEMENTATIONS
pub struct FileOfferPacket {
    pub transaction_id: u64,
    pub file_size: u64,
    // in bytes
    pub file_name: String,
}

impl FileOfferPacket {
    pub const ID: u32 = 100_000;
    pub fn new(transaction_id: u64, file_size: u64, file_name: String) -> Self {
        Self { transaction_id, file_size, file_name }
    }
    pub fn construct(field_bytes: &[u8]) -> Result<Self, String> {
        if field_bytes.len() < 17 {
            return Err(format!("Packet has {} bytes but at least 17 were expected", field_bytes.len()));
        }
        let bytes: [u8; 8] = field_bytes[0..8].try_into().unwrap();
        let transaction_id = u64::from_be_bytes(bytes);

        let bytes: [u8; 8] = field_bytes[8..16].try_into().unwrap();
        let file_size = u64::from_be_bytes(bytes);

        return match String::from_utf8(field_bytes[16..].to_vec()) {
            Ok(file_name) => Ok(Self::new(transaction_id, file_size, file_name)),
            Err(e) => Err(e.to_string()),
        };
    }
}

impl Packet for FileOfferPacket {
    fn id(&self) -> u32 {
        FileOfferPacket::ID
    }

    fn size(&self) -> u32 {
        (8 + 8 + self.file_name.len()) as u32
    }

    fn write(&self, stream: &mut TcpStream) -> Result<(), std::io::Error> {
        tcp_write_safe(&self.transaction_id.to_be_bytes(), stream)
            .and(tcp_write_safe(&self.file_size.to_be_bytes(), stream))
            .and(tcp_write_safe(self.file_name.as_bytes(), stream))
    }
}

// FILE PACKET
pub struct FilePacket<'r> {
    pub transaction_id: u64,
    // the first chunk id should always be 0 when a transfer begins or is resumed
    pub chunk_id: u64,
    pub file_bytes: &'r [u8],
}

impl<'r> FilePacket<'r> {
    pub const ID: u32 = 200_000;
    pub fn new(transaction_id: u64, chunk_id: u64, content: &'r [u8]) -> Self {
        Self { transaction_id, chunk_id, file_bytes: content }
    }
    pub fn wrap(field_bytes: &'r [u8]) -> Result<Self, String> {
        let length = field_bytes.len();
        if length < 16 {
            return Err(format!("Packet has {length} bytes but 16 were expected"));
        }
        let transaction_bytes: [u8; 8] = field_bytes[0..8].try_into().unwrap();
        let transaction_id = u64::from_be_bytes(transaction_bytes);

        let chunk_id_bytes: [u8; 8] = field_bytes[8..16].try_into().unwrap();
        let chunk_id = u64::from_be_bytes(chunk_id_bytes);

        let file_bytes = &field_bytes[16..length];

        Ok(Self::new(transaction_id, chunk_id, file_bytes))
    }
}

impl<'r> Packet for FilePacket<'r> {
    fn id(&self) -> u32 {
        FilePacket::ID
    }

    fn size(&self) -> u32 {
        (8 + 8 + self.file_bytes.len()) as u32
    }

    fn write(&self, stream: &mut TcpStream) -> Result<(), std::io::Error> {
        tcp_write_safe(&self.transaction_id.to_be_bytes(), stream)
            .and(tcp_write_safe(&self.chunk_id.to_be_bytes(), stream))
            .and(tcp_write_safe(self.file_bytes, stream))
    }
}


// Used for testing purposes
pub struct SpeedPacket<'r> {
    pub random_bytes: &'r [u8],
}

impl<'r> SpeedPacket<'r> {
    pub const ID: u32 = 300_000;
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

    fn write(&self, stream: &mut TcpStream) -> Result<(), std::io::Error>{
        tcp_write_safe(self.random_bytes, stream)
    }
}

// Used for testing purposes
pub struct SpeedtestInfoPacket {
    pub start_time: u64, // future unix time - the moment reading and writing should commence
}

impl SpeedtestInfoPacket {
    pub const ID: u32 = 400_000;
    pub fn new_with_start(start: u64) -> Self {
        Self { start_time: start }
    }
    pub fn get_start_time(field_bytes: &[u8]) -> u64 {
        let unix_bytes: [u8; 8] = field_bytes[0..8].try_into().unwrap();
        u64::from_be_bytes(unix_bytes)
    }
}

impl Packet for SpeedtestInfoPacket{
    fn id(&self) -> u32 {
        SpeedtestInfoPacket::ID
    }
    fn size(&self) -> u32 {
        8u32
    }
    fn write(&self, stream: &mut TcpStream) -> Result<(), std::io::Error> {
        tcp_write_safe(&self.start_time.to_be_bytes(), stream)
    }
}

pub fn epoch_time_now() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH).unwrap()
        .as_millis() as u64
}

pub struct PingPacket {
    pub creation_time: u64, // unix time the moment packet is made
}
impl PingPacket {
    pub const ID: u32 = 500_000;
    pub fn new_ping() -> Self {
        Self { creation_time: epoch_time_now() }
    }

    pub fn millis_taken(field_bytes: &[u8]) -> i64 {
        let now = epoch_time_now() as i64;
        let unix_bytes: [u8; 8] = field_bytes[0..8].try_into().unwrap();
        let time_sent = u64::from_be_bytes(unix_bytes) as i64;
        now - time_sent
    }
}
impl Packet for PingPacket {
    fn id(&self) -> u32 {
        PingPacket::ID
    }

    fn size(&self) -> u32 {
        8u32
    }

    fn write(&self, stream: &mut TcpStream) -> std::io::Result<()>  {
        tcp_write_safe(&self.creation_time.to_be_bytes(), stream)
    }
}

pub struct BeginUploadPacket {
    pub transaction_id: u64,
    pub files_accepted: u32,
    // files_accepted == file_indexes.len() == cursors.len()
    pub file_indexes: Vec<u32>, // 0-indexed
    pub cursors: Vec<u64>,
}

impl BeginUploadPacket {
    pub const ID: u32 = 800_000;

    pub fn single_file(transaction_id: u64, cursor: u64) -> Self {
        let file_indexes = vec![0u32];
        let cursors = vec![cursor];
        Self { transaction_id, files_accepted: 1, file_indexes, cursors }
    }
    pub fn has_any_files(&self) -> bool {
        self.files_accepted > 0 && self.file_indexes.len() > 0 && self.cursors.len() > 0
    }

    pub fn accept_all(transaction_id: u64, count: u64) -> Self {
        let mut file_indexes = Vec::with_capacity(count as usize);
        let mut cursors = Vec::with_capacity(count as usize);
        for i in 0..count {
            file_indexes.push(i as u32);
            cursors.push(0);
        }
        Self { transaction_id, files_accepted: count as u32, file_indexes, cursors }
    }

    pub fn new(transaction_id: u64, file_indexes: Vec<u32>, cursors: Vec<u64>) -> Self {
        if file_indexes.len() != cursors.len() {
            panic!("ERROR: To ensure data integrity file_indexes & cursors must be the same length");
        }
        let files_accepted = file_indexes.len() as u32;
        if files_accepted == 0 {
            return Self::new_empty();
        }
        Self { transaction_id, files_accepted, file_indexes, cursors }
    }

    pub fn new_empty() -> Self {
        Self { transaction_id: 0, files_accepted: 0, file_indexes: vec![], cursors: vec![] }
    }

    pub fn from_bytes(field_bytes: &[u8]) -> Self {
        if field_bytes.len() < 12 {
            eprintln!("Packet was too small");
            return Self::new_empty();
        }
        let id_bytes: [u8; 8] = field_bytes[0..8].try_into().unwrap();
        let transaction_id = u64::from_be_bytes(id_bytes);

        let files_accepted_bytes: [u8; 4] = field_bytes[8..12].try_into().unwrap();
        let files_accepted = u32::from_be_bytes(files_accepted_bytes);

        let mut file_indexes = Vec::with_capacity(files_accepted as usize);
        let mut offset = 12;
        for _ in 0..files_accepted {
            let index_bytes: [u8; 4] = field_bytes[offset..offset + 4].try_into().unwrap();
            let index = u32::from_be_bytes(index_bytes);
            file_indexes.push(index);
            offset += 4;
        }

        let mut cursors = Vec::with_capacity(files_accepted as usize);
        for _ in 0..files_accepted {
            let cursor_bytes: [u8; 8] = field_bytes[offset..offset + 8].try_into().unwrap();
            let cursor = u64::from_be_bytes(cursor_bytes);
            cursors.push(cursor);
            offset += 8;
        }

        Self { transaction_id, files_accepted, file_indexes, cursors }
    }
}
impl Packet for BeginUploadPacket {
    fn id(&self) -> u32 {
        BeginUploadPacket::ID
    }

    fn size(&self) -> u32 {
        (8 + 4 + self.file_indexes.len() * 4 + self.cursors.len() * 8) as u32
    }

    fn write(&self, stream: &mut TcpStream) -> std::io::Result<()> {
        let mut write_result = tcp_write_safe(&self.transaction_id.to_be_bytes(), stream)
            .and(tcp_write_safe(&self.files_accepted.to_be_bytes(), stream));

        for i in 0..self.files_accepted {
            let index: u32 = self.file_indexes[i as usize];
            write_result = write_result.and(tcp_write_safe(&index.to_be_bytes(), stream))
        }
        for i in 0..self.files_accepted {
            let cursor: u64 = self.cursors[i as usize];
            write_result = write_result.and(tcp_write_safe(&cursor.to_be_bytes(), stream))
        }
        return write_result
    }
}

pub struct FileInfo {
    pub size: u64,
    name_size: u64,
    pub name: String,
}
impl FileInfo {
    pub fn new(name: String, size: u64) -> Self {
        Self { size, name_size: name.len() as u64, name, }
    }
}
pub struct DirectoryOfferPacket {
    pub total_size: u64,
    pub file_count: u64,
    pub name_size: u64,
    pub directory_name: String,
    pub files: Vec<FileInfo>
}
impl DirectoryOfferPacket {
    pub const ID: u32 = 900_000;

    pub fn new(directory_path: &str) -> Self {
        let dir_name = util::get_path_name(directory_path).to_string();
        let Ok (entries) = fs::read_dir(directory_path) else {
            eprintln!("Failed to create DirectoryOfferPacket");
            return Self::empty();
        };

        let mut total_size: u64 = 0;
        let mut files = vec![];
        for entry in entries {
            let path = entry.unwrap().path();
            if !path.is_file() {
                continue
            }
            let Ok(metadata) = path.metadata() else {
                eprintln!("Skipping entry, unable to retrieve metadata");
                continue
            };
            let size = metadata.len();
            total_size += size;
            let entry_name = path.file_name().unwrap().to_str().unwrap().to_string();
            let file_info = FileInfo::new(entry_name, size);
            files.push(file_info);
        }

        let file_count: u64 = files.len() as u64;
        let name_size = dir_name.len() as u64;
        Self { total_size, file_count, name_size, directory_name: dir_name, files }
    }

    pub fn from_bytes(field_bytes: &[u8]) -> Self {
        if field_bytes.len() < 8*3 + 1 {
            eprintln!("Packet is too small to be deserialized");
            return Self::empty();
        }
        let size_bytes: [u8; 8] = field_bytes[0..8].try_into().unwrap();
        let total_size = u64::from_be_bytes(size_bytes);

        let count_bytes: [u8; 8] = field_bytes[8..16].try_into().unwrap();
        let file_count = u64::from_be_bytes(count_bytes);

        let name_size_bytes: [u8; 8] = field_bytes[16..24].try_into().unwrap();
        let name_size = u64::from_be_bytes(name_size_bytes);

        let after_name = (24 + name_size) as usize;
        let name_bytes = field_bytes[24..after_name].to_vec();
        let dir_name = String::from_utf8(name_bytes).expect("Failed to decode file name");

        let mut files_bytes = &field_bytes[after_name..];
        let mut files = Vec::with_capacity(file_count as usize);
        for _ in 0..file_count {
            let size_bytes: [u8; 8] = files_bytes[0..8].try_into().unwrap();
            let size = u64::from_be_bytes(size_bytes);

            let name_size_bytes: [u8; 8] = files_bytes[8..16].try_into().unwrap();
            let name_size = u64::from_be_bytes(name_size_bytes);

            let packet_end = (16 + name_size) as usize;
            let name_bytes = files_bytes[16..packet_end].to_vec();
            let name = String::from_utf8(name_bytes).expect("Failed to decode file name");
            let file_info = FileInfo { size, name_size, name };
            files.push(file_info);
            files_bytes = &files_bytes[packet_end..]
        }

        Self { total_size, file_count, name_size, directory_name: dir_name, files }
    }

    pub fn empty() -> Self {
        Self { total_size: 0, file_count: 0, name_size: 0, directory_name: "".into(), files: vec![]}
    }
}
impl Packet for DirectoryOfferPacket {
    fn id(&self) -> u32 {
        DirectoryOfferPacket::ID
    }

    fn size(&self) -> u32 {
        let mut size = 24 + self.name_size;
        for file in &self.files {
            size += 8 * 2;
            size += file.name_size;
        }
        size as u32
    }

    fn write(&self, stream: &mut TcpStream) -> std::io::Result<()> {
        let mut write_result = tcp_write_safe(&self.total_size.to_be_bytes(), stream)
            .and(tcp_write_safe(&self.file_count.to_be_bytes(), stream))
            .and(tcp_write_safe(&self.name_size.to_be_bytes(), stream))
            .and(tcp_write_safe(self.directory_name.as_bytes(), stream));

        for file in &self.files {
            write_result = write_result
                .and(tcp_write_safe(&file.size.to_be_bytes(), stream))
                .and(tcp_write_safe(&file.name_size.to_be_bytes(), stream))
                .and(tcp_write_safe(file.name.as_bytes(), stream))
        }
        write_result
    }
}