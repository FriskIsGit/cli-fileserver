## CLI File Server
TCP-based file exchange program

### Config
- having a config file is `not required` as long as command line parameters are provided <br>
- config file is read down line by line so if a key is redefined again it'll be overwritten<br>
- lines that don't start with any recognized config `key` are not parsed (can be for as comments) <br>
- when running as `host` without an ip argument the host ip will be automatically assigned

### Running
1. Rename `config-example.txt` file to `config.txt`
2. Configure values for either a `host` or a `client` setup
3. Alternatively pass them as command line arguments
> Additional arguments: <br>
   -ip, --ip=10.0.0.3 <br>
   -p, --port=5313
>

Run `cargo run connect` or `cargo run host`

### Usage
- file sharing: `share <path>`, the other end should perform a `read`
- speedtest: `si` - uploading peer, `so` - downloading peer
- RTT (round trip time): `rtt 1` - any peer, `rtt 2` - other peer
- close connection (stream close): `shutdown`