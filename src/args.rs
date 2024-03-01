
pub const HOST: &str = "host";
pub const CONNECT: &str = "connect";

// parse program specific arguments like flags
// args: [program.exe, 0, 1, 2, ...]
pub struct ProgramArgs {
    // We can use 'exe' path for determining the relative location of config.txt
    pub exe: String,
    pub args: Vec<String>,
    pub address: Option<String>,
    pub port: Option<u16>,
}

impl ProgramArgs {
    pub fn retrieve() -> Self {
        let mut args: Vec<String> = std::env::args().collect();
        if args.is_empty() {
            panic!("Is this possible?")
        }
        let exe_path = std::mem::take(&mut args[0]);
        args.rotate_left(1);
        unsafe {
            args.set_len(args.len() - 1)
        }

        let length = args.len();
        let mut port_arg = None;
        let mut address_arg = None;
        let mut i = 0;
        while i < length {
            let argument = &args[i];
            if argument == "-p" && i+1 < length {
                match args[i+1].parse::<u16>() {
                    Ok(port) => port_arg = Some(port),
                    Err(_) => panic!("Failed to parse port argument!"),
                }
                i += 1;
            } else if argument.starts_with("--port=")  {
                let Some(equal) = argument.find('=') else {
                    println!("Unrecognized argument: {argument}");
                    continue
                };
                match argument[equal+1..].parse::<u16>() {
                    Ok(port) => port_arg = Some(port),
                    Err(_) => panic!("Failed to parse port argument!"),
                }
            } else if argument.starts_with("-a") && i+1 < length {
                address_arg = Some(args[i+1].to_string());
                i += 1;
            } else if argument.starts_with("--address=")  {
                let Some(equal) = argument.find('=') else {
                    println!("Unrecognized argument: {argument}");
                    continue
                };
                address_arg = Some(argument[equal+1..].to_string())
            }
            i += 1;
        }
        Self { exe: exe_path, args, address: address_arg, port: port_arg}
    }

    pub fn print_info() {
        println!("fileserver {HOST} - listen for a connection");
        println!("fileserver {CONNECT} - initiate a connection");
        println!("Additional arguments:");
        println!("-a, --address=<string>");
        println!("-p, --port=<u16>");
    }
}