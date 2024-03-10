## CLI File Server
TCP-based file exchange program

### Features
- Downloads can be resumed at any time as long as the file is present in the filesystem
- Each packet is validated (if sent out of order or corrupt - the download will terminate)
- Current `progress`, `speed` and `ETA` are updated every packet and displayed

### Config
- having a config file is `not required` as long as command line parameters are provided <br>
- config file is read down line by line so if a key is redefined again it'll be overwritten <br>
- lines that don't start with any recognized config `key` are not parsed (can be used for comments) <br>
- when running as `host` without an ip argument the host ip will be automatically assigned

Run `cargo r host` or `cargo r connect`

### Running
1. Rename `config-ex.txt` file to `config.txt`
2. Configure values for either a `host` or a `client` setup
3. Alternatively pass them as command line arguments
> Additional arguments: <br>
   -ip, --ip=10.0.0.3 <br>
   -p, --port=5313 <br>
   -aa, --auto-accept


### Usage
- file sharing: `share <path>`, the other end should perform a `read`
- speedtest: `si` - downloading peer, `so` - uploading peer
- RTT (round trip time): `rtt 1` - any peer, `rtt 2` - other peer
- close connection (stream close): `shutdown`

### Code snippet
```rust
pub struct FilePacket<'r> {
    pub transaction_id: u64,
    pub chunk_id: u64,
    pub file_bytes: &'r [u8],
}
```