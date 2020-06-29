use stack_vec::StackVec;
use crate::console::{kprint, kprintln, CONSOLE};
/// Error type for `Command` parse failures.
#[derive(Debug)]
enum Error {
    Empty,
    TooManyArgs,
}

/// A structure representing a single shell command.
struct Command<'a> {
    args: StackVec<'a, &'a str>,
}

impl<'a> Command<'a> {
    /// Parse a command from a string `s` using `buf` as storage for the
    /// arguments.
    ///
    /// # Errors
    ///
    /// If `s` contains no arguments, returns `Error::Empty`. If there are more
    /// arguments than `buf` can hold, returns `Error::TooManyArgs`.
    fn parse(s: &'a str, buf: &'a mut [&'a str]) -> Result<Command<'a>, Error> {
        let mut args = StackVec::new(buf);
        for arg in s.split(' ').filter(|a| !a.is_empty()) {
            args.push(arg).map_err(|_| Error::TooManyArgs)?;
        }

        if args.is_empty() {
            return Err(Error::Empty);
        }

        Ok(Command { args })
    }

    /// Returns this command's path. This is equivalent to the first argument.
    fn path(&self) -> &str {
        self.args[0]
    }
}

/// Starts a shell using `prefix` as the prefix for each line. This function
/// returns if the `exit` command is called.
pub fn shell(prefix: &str) -> ! {
    loop {
        kprint!("\n\r");
        kprint!("{}",prefix);
        let mut buf = [0u8; 512];
        let mut read_size = 0;
        {
            let mut console = CONSOLE.lock();
            buf[read_size] = console.read_byte();
        }
        while read_size <511 && buf[read_size] != b'\n' && buf[read_size] != b'\r' {
            if buf[read_size] >=32 && buf[read_size] <=126 {
                let mut console = CONSOLE.lock();
                console.write_byte(buf[read_size]);
            } else if buf[read_size] == 8 {
                if read_size > 0 {          // Not to backspace into prefix
                    kprint!("\u{8} \u{8}");
                }
                read_size -= 1;
            } else 
            {
                kprint!("\u{7}");
                read_size -= 1;
            }
            read_size += 1;
            {
                let mut console = CONSOLE.lock();
                buf[read_size] = console.read_byte();
            }
        }
        buf[read_size] = 0u8;
        let cstr = core::str::from_utf8(&buf[0..read_size]).unwrap();
        let mut bufstr = [""; 64]; 
        match Command::parse(cstr, &mut bufstr){
            Ok(cmd) => {
                kprint!("\n\r");
                if cmd.path()=="echo" {
                    for i in 1..cmd.args.len() {
                        kprint!("{} ",cmd.args[i]);
                    }
                } else {
                    kprint!("unknown command: {}", cmd.path());
                }
            },
            Err(Error::Empty) => {
                kprint!(" ")
            }, 
            Err(Error::TooManyArgs) => {
                kprint!("\n\r");
                kprint!("error: too many arguments");
            }
        }
    }
}

