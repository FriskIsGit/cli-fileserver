
pub const SERVER: &str = "host";
pub const CONNECT: &str = "connect";

// parse program specific arguments like flags
// args: [program.exe, 0, 1, 2, ...]
pub struct ProgramArgs {
    // We can use 'exe' path for determining the relative location of config.txt
    pub exe: String,
    pub args: Vec<String>,
}

impl ProgramArgs {
    pub fn retrieve() -> Self {
        let mut args: Vec<String> = std::env::args().collect();
        if args.len() == 0 {
            panic!("Is this possible?")
        }
        let exe_path = std::mem::take(&mut args[0]);
        args.rotate_left(1);
        unsafe {
            args.set_len(args.len() - 1)
        }
        Self { exe: exe_path, args }
    }

    pub fn has_args(&self) -> bool {
        self.args.len() > 0
    }
    pub fn print_info() {
        println!("Specify server or client:");
        println!("{SERVER}");
        println!("{CONNECT}")
    }
}