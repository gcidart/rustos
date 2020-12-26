use shim::io;
use shim::path::{Path, PathBuf};

use stack_vec::StackVec;
use alloc::vec::Vec;
use alloc::string::String;
use pi::atags::Atags;


use fat32::traits::FileSystem;
use fat32::traits::{Dir, Entry};

use crate::console::{kprint, kprintln, CONSOLE};
use crate::ALLOCATOR;
use crate::FILESYSTEM;

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

/// Starts a shell using `prefix` as the prefix for each line.
pub fn shell(prefix: &str)  {
    let mut path = PathBuf::from("/");
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
                } else if cmd.path()=="panic" {
                    panic!();
                } else if cmd.path()=="atag" {
                    let mut atag = Atags::get(); 
                    loop {
                        match atag.next()  {
                            Some(a) => kprintln!("{:#?}", a),
                            None => break,
                        }
                    }
                } else if cmd.path()=="ls" {
                    ls_function(&cmd, &path);
                } else if cmd.path()=="pwd" {
                    cwd_function(&path);
                } else if cmd.path()=="cd" {
                    cd_function(&cmd, &mut path);
                } else if cmd.path()=="cat" {
                    cat_function(&cmd, &path);
                } else if cmd.path()=="sleep" {
                    sleep_function(&cmd);
                } else if cmd.path()=="exit" {
                    return;
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

fn ls_function(cmd: &Command, cwd_path: &PathBuf) {
    if cmd.args.len()==1 {
        let entries: Vec<_> = FILESYSTEM.open_dir(cwd_path).unwrap().entries().expect("entries interator").collect();
        for entry in entries.iter() {
            kprintln!("{}", entry.name());
        }
    } else if cmd.args.len()==2 {
        let mut cwd_path_clone = cwd_path.clone();
        let path = PathBuf::from(cmd.args[1]);
        merge_paths(&mut cwd_path_clone, &path);
        match FILESYSTEM.open_dir(cwd_path_clone) {
            Ok(dir) => match dir.entries() {
                Ok(itr) => {
                    let entries : Vec<_> = itr.collect();
                    for entry in entries.iter() {
                        kprintln!("{}", entry.name());
                    }
                },
                Err(_) => kprintln!("Error in getting the entries for the directory")
            }
            Err(_) => kprintln!("Invalid input")
        };
    } else {
        kprintln!("Incorrect command\n ls [directory path]");
    }
}

fn cwd_function(cwd_path: &PathBuf) {
    kprintln!("{}", cwd_path.to_str().unwrap());
}

fn cd_function(cmd: &Command, cwd_path: &mut PathBuf) {
    if cmd.args.len()==2 {
        let mut cwd_path_clone = cwd_path.clone();
        let path = PathBuf::from(cmd.args[1]);
        merge_paths(&mut cwd_path_clone, &path);
        match FILESYSTEM.open_dir(cwd_path_clone) {
            Ok(_) => merge_paths(cwd_path, &path),
            Err(_) => kprintln!("Directory does not exist"),
        };
    } else {
        kprintln!("Incorrect command\n cd <directory path>");
    }
}

fn cat_function(cmd: &Command, cwd_path: &PathBuf) {
    use io::Read;
    if cmd.args.len()>=2 {
        for i in 1..cmd.args.len() {
            let mut cwd_path_clone = cwd_path.clone();
            let path = PathBuf::from(cmd.args[i]);
            merge_paths(&mut cwd_path_clone, &path);
            match FILESYSTEM.open_file(cwd_path_clone.as_path()) {
                Ok(mut file) => loop{
                    let mut buffer = Vec::new();
                    buffer.resize(4096, 0);
                    match file.read(&mut buffer) {
                        Ok(0) => break,
                        //Ok(read_size) => kprint!("{:?}", String::from_utf8(buffer[..read_size].to_vec()).unwrap()),
                        Ok(read_size) => match String::from_utf8(buffer[..read_size].to_vec()){
                            Ok(s) => kprint!("{}", s),
                            Err(e) => {
                                kprintln!("{:?}", e);
                                break;
                            },
                        },
                        _ => kprintln!("\n error reading file"),
                    };
                }   
                Err(e) => kprintln!("Invalid input {:?}", e)
            };
        }
    } else {
        kprintln!("Incorrect command\n cat <file path....>");
    }
}

fn merge_paths(path: &mut PathBuf, rel_path: &PathBuf) {
    let components: Vec<_> = rel_path.components().map(|comp| comp.as_os_str()).collect();
    for component in components {
        let chk1 = component.to_str().unwrap()=="..";
        let chk2 = component.to_str().unwrap()==".";
        let tpath = PathBuf::from(component.to_str().unwrap());
        if chk1 {
            path.pop();
        } else if !chk2{
            path.push(tpath);
        }
    }
}

fn sleep_function(cmd: &Command) {
    if cmd.args.len()!= 2 {
        kprintln!("Incorrect command\n sleep <duration in ms>");
    }
    let delay ;
    match cmd.args[1].parse::<u32>() {
        Ok(d) => delay = core::time::Duration::from_millis(d as u64),
        _   => {
            kprintln!("Incorrect command\n sleep <duration in ms>");
            return;
        }
    }
    kprintln!("sleep {:?}", delay);
    kprintln!("slept for {:?}", kernel_api::syscall::sleep(delay).unwrap());
}

