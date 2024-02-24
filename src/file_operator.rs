use std::fs::File;
use std::io::{Read, Seek, Result};

const MB_1: usize = 1048576;

#[derive(Debug)]
pub struct FileFeeder {
    file: File,
    length: u64,
    chunk_size: usize,
    buffer: Vec<u8>,
}
impl FileFeeder {
    pub fn new(path: &str) -> Result<Self> {
        let chunk_size = MB_1;
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
            // we need to do this again here as opposed to using a variable
            // or the borrow checker will think this is an attempt at returning a local variable
            return match self.file.read_exact(&mut self.buffer[0..desired_size]) {
                Ok(_) => Ok(&self.buffer[0..desired_size]),
                Err(err) => Err(err)
            }
        }
        match self.file.read_exact(&mut self.buffer) {
            Ok(_) => Ok(&self.buffer),
            Err(err) => Err(err)
        }
    }

    // it doesn't modify anything
    fn cursor_pos(&mut self) -> u64 {
        self.file.stream_position()
            .expect("When does it fail?")
    }
}
