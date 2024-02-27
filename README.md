## CLI File Server
TCP-based file exchange program

### Config
Config file is read down line by line <br>
Absence of a config file or a missing key will cause default values to be applied <br>
Every line that doesn't start with a recognized config `key` is skipped

### Running
1. Rename `config-example.txt` file to `config.txt`
2. Configure values for either a `host` or a `client` setup
3. Alternatively pass them as command line arguments
> Additional arguments: <br>
   -a, --address=10.0.0.3 <br>
   -p, --port=5313
4. Run `fileserver connect` or `fileserver host`
