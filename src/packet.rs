/*
PACKET STRUCTURE FORMAT:
    id = packet id
    content size = field bytes length
    content = field bytes streamlined (raw)
     --------------------------------
     |  id | content size | content |
     | u32 |      u32     | Vec<u8> |
     --------------------------------
    packets can not be bigger than ~ 4.29 GB
*/
use std::string::FromUtf8Error;

pub const FIELD_OFFSET: usize = 8;
pub trait Packet {

    // Every packet must identify itself
    fn id(&self) -> u32;

    // Every packet must provide its content's size in bytes
    fn size(&self) -> u32;

    // Every packet has a function which constructs a packet object from field bytes or yields error
    fn construct_packet(field_bytes: &[u8]) -> Result<Self, String> where Self: Sized;

    // Every packet must serialize its fields individually in the provided buffer
    // The buffer is always big enough to contain declared length
    fn parcel_fields(&self, field_bytes: &mut [u8]);

    // This parcels the entire packet according to the format
    fn parcel(&self) -> Vec<u8> {
        let content_size = self.size();
        let mut all_data = vec![0u8; FIELD_OFFSET + content_size as usize];
        // 1. ID
        let id_bytes = self.id().to_be_bytes();
        for i in 0..4 {
            all_data[i] = id_bytes[i];
        }
        let mut data_i = 4;
        // 2. Field bytes length
        let content_size_bytes = content_size.to_be_bytes();
        for i in 0..4 {
            all_data[data_i] = content_size_bytes[i];
            data_i += 1;
        }
        // 3. Field bytes
        self.parcel_fields(&mut all_data[FIELD_OFFSET..]);
        all_data
    }

    fn read_id(byte_stream: &Vec<u8>) -> Option<u32> {
        if byte_stream.len() < 4 {
            return None;
        }
        let id_bytes: [u8; 4] = byte_stream[0..4].try_into().unwrap();
        Some(u32::from_be_bytes(id_bytes))
    }

    fn read_content_size(byte_stream: &Vec<u8>) -> Option<u32> {
        if byte_stream.len() < 8 {
            return None;
        }
        let content_size_bytes: [u8; 4] = byte_stream[4..8].try_into().unwrap();
        Some(u32::from_be_bytes(content_size_bytes))
    }
}

// PACKET STRUCT IMPLEMENTATIONS
pub struct FileInfoPacket {
    pub file_size: u64, // in bytes
    pub file_name: String,
}

impl FileInfoPacket {
    pub const ID: u32 = 100_000;
    pub fn new(file_size: u64, file_name: String) -> Self {
        Self {
            file_size,
            file_name,
        }
    }
}
impl Packet for FileInfoPacket {
    fn id(&self) -> u32 {
        FileInfoPacket::ID
    }

    fn size(&self) -> u32 {
        (8 + self.file_name.len()) as u32
    }

    fn construct_packet(field_bytes: &[u8]) -> Result<Self, String> {
        let length = field_bytes.len();
        if length < 9 {
            return Err(format!("Packet has {length} bytes but at least 9 were expected"));
        }
        let file_size_bytes: [u8; 8] = field_bytes[0..8].try_into().unwrap();
        let file_size = u64::from_be_bytes(file_size_bytes);

        return match String::from_utf8(field_bytes[8..length].to_vec()) {
            Ok(file_name) => Ok(Self::new(file_size, file_name)),
            Err(e) => Err(e.to_string()),
        }
    }

    fn parcel_fields(&self, field_bytes: &mut [u8]) {
        let file_size_bytes = self.file_size.to_be_bytes();
        for i in 0..8 {
            field_bytes[i] = file_size_bytes[i];
        }
        let mut f = 8;
        let name_bytes = self.file_name.as_bytes();
        for i in 0..name_bytes.len() {
            field_bytes[f] = name_bytes[i];
            f += 1;
        }
    }
}

// FILE PACKET
pub struct FilePacket {
    pub chunk_id: u64,
    pub payload_size: u64,
    pub file_bytes: Vec<u8>,
}

impl FilePacket {
    pub const ID: u32 = 200_000;
    pub fn new(chunk_id: u64, payload_size: u64, content: Vec<u8>) -> Self {
        Self { chunk_id, payload_size, file_bytes: content }
    }
}

impl Packet for FilePacket {
    fn id(&self) -> u32 {
        FilePacket::ID
    }

    fn size(&self) -> u32 {
        (8 + 8 + self.file_bytes.len()) as u32
    }

    fn construct_packet(field_bytes: &[u8]) -> Result<Self, String> {
        let length = field_bytes.len();
        if length < 16 {
            return Err(format!("Packet has {length} bytes but 16 were expected"));
        }
        let chunk_id_bytes: [u8; 8] = field_bytes[0..8].try_into().unwrap();
        let chunk_id = u64::from_be_bytes(chunk_id_bytes);

        let payload_id_bytes: [u8; 8] = field_bytes[8..16].try_into().unwrap();
        let payload_size = u64::from_be_bytes(payload_id_bytes);

        let file_bytes = field_bytes[16..length].to_vec();

        Ok(Self::new(chunk_id, payload_size, file_bytes))
    }

    fn parcel_fields(&self, field_bytes: &mut [u8]) {
        let chunk_bytes = self.chunk_id.to_be_bytes();
        for i in 0..8 {
            field_bytes[i] = chunk_bytes[i];
        }
        let payload_bytes = self.payload_size.to_be_bytes();
        let mut f = 8;
        for i in 0..8 {
            field_bytes[f] = payload_bytes[i];
            f += 1;
        }
        for i in 0..self.file_bytes.len() {
            field_bytes[f] = self.file_bytes[i];
            f += 1;
        }
    }
}