
pub const HOST: &str = "host";
pub const CONNECT: &str = "connect";

// parse program specific arguments like flags
// args: [program.exe, 0, 1, 2, ...]
pub struct ProgramArgs {
    // We can use 'exe' path for determining the relative location of config.txt
    pub exe: String,
    pub args: Vec<String>,
    pub ip: Option<String>,
    pub port: Option<u16>,
    pub host_auto_accept: Option<bool>,
}

impl ProgramArgs {
    pub fn parse(mut args: Vec<String>) -> Self {
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
        let mut ip_arg = None;
        let mut host_auto_accept = None;
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
                match argument[7..].parse::<u16>() {
                    Ok(port) => port_arg = Some(port),
                    Err(_) => panic!("Failed to parse port argument!"),
                }
            } else if argument.starts_with("-ip") && i+1 < length {
                ip_arg = Some(args[i+1].to_string());
                i += 1;
            } else if argument.starts_with("--ip=")  {
                ip_arg = Some(argument[5..].to_string())
            }
            else if argument == "-aa" || argument == "--auto-accept" {
                host_auto_accept = Some(true);
            }
            i += 1;
        }
        Self { exe: exe_path, args, ip: ip_arg, port: port_arg, host_auto_accept}
    }

    pub fn str_to_bool(flag: &str) -> bool {
        match std::str::FromStr::from_str(flag) {
            Ok(boolean) => boolean,
            Err(_) => panic!("Not a boolean <{flag}>"),
        }
    }

    pub fn retrieve() -> Self {
        Self::parse(std::env::args().collect())
    }

    pub fn print_info() {
        println!("fileserver {HOST} - listen for a connection");
        println!("fileserver {CONNECT} - initiate a connection");
        println!("Additional arguments:");
        println!("-ip, --ip=<string>");
        println!("-p, --port=<u16>");
        println!("-aa, --auto-accept=<bool>");
    }
}