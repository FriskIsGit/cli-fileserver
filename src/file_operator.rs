use std::fs::File;
use std::io::{Read, Seek, Result, SeekFrom};

pub struct FileFeeder {
    file: File,
    length: u64,
    chunk_size: usize,
    buffer: Vec<u8>,
}
impl FileFeeder {
    pub fn new(path: &str, chunk_size: usize) -> Result<Self> {
        let buffer = vec![0u8; chunk_size];
        let file_result = File::open(path);
        let Ok(file) = file_result else {
            return Err(file_result.unwrap_err());
        };
        let length = file.metadata().unwrap().len();
        Ok(Self {
            file,
            length,
            chunk_size,
            buffer,
        })
    }
    pub fn has_next_chunk(&mut self) -> bool {
        self.cursor_pos() < self.length
    }

    // file.read_exact method moves cursor position which isn't specified in the docs
    pub fn read_next_chunk(&mut self) -> Result<&[u8]> {
        let cursor = self.cursor_pos();

        // or we can quit on EOF error without this logic?
        if cursor + self.buffer.len() as u64 > self.length {
            let desired_size = (self.length - cursor) as usize;
            let buffer = &mut self.buffer[0..desired_size];
            return match self.file.read_exact(buffer) {
                Ok(_) => Ok(buffer),
                Err(err) => Err(err)
            }
        }
        match self.file.read_exact(&mut self.buffer) {
            Ok(_) => Ok(&self.buffer),
            Err(err) => Err(err)
        }
    }

    pub fn file_size(&self) -> u64 {
        self.length
    }

    // it doesn't modify anything
    pub fn set_cursor_pos(&mut self, cursor: u64) {
        let offset = SeekFrom::Start(cursor);
        self.file.seek(offset).expect("When does it fail?");
    }
    // it doesn't modify anything
    fn cursor_pos(&mut self) -> u64 {
        self.file.stream_position()
            .expect("When does it fail?")
    }
}

// TODO struct BufferedFileWriter
