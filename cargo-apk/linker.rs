//! This file contains the source code of a dummy linker whose path is going to get passed to
//! rustc. Rustc will think that this program is gcc and pass all the arguments to it. Then this
//! program will tweak the arguments as needed.

use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::io::{Write, BufReader, BufRead};
use std::path::{Path, PathBuf};
use std::process;
use std::process::{Command, Stdio};

fn main() {
    let (args, passthrough) = parse_arguments();

    // Write the arguments for the subcommand to pick up.
    {
        let mut lib_paths = File::create(Path::new(&args.cargo_apk_libs_path_output)).unwrap();
        for lib_path in args.library_path.iter() {
            writeln!(lib_paths, "{}", lib_path.to_string_lossy()).unwrap();
        }

        let mut libs = File::create(Path::new(&args.cargo_apk_libs_output)).unwrap();
        for lib in args.shared_libraries.iter() {
            writeln!(libs, "{}", lib).unwrap();
        }
    }

    // Execute the real linker.
    if Command::new(Path::new(&args.cargo_apk_gcc))
        .args(&*passthrough)
        .arg(args.cargo_apk_native_app_glue)
        .arg("-llog").arg("-landroid")      // these two libraries are used by the injected-glue
        .arg("--sysroot").arg(args.cargo_apk_gcc_sysroot)
        .arg("-o").arg(args.cargo_apk_linker_output)
        .arg("-shared")
        .arg("-Wl,-E")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status().unwrap().code().unwrap() != 0
    {
        println!("Error while executing gcc");
        process::exit(1);
    }
}

struct Args {
    // Paths where to search for libraries as passed with the `-L` options.
    library_path: Vec<PathBuf>,

    // List of libraries to link to as passed with the `-l` option.
    shared_libraries: HashSet<String>,

    cargo_apk_gcc: String,
    cargo_apk_gcc_sysroot: String,
    cargo_apk_native_app_glue: String,
    cargo_apk_linker_output: String,
    cargo_apk_libs_path_output: String,
    cargo_apk_libs_output: String,
}

/// Parses the arguments passed by the CLI and returns two things: the interpretation of some
/// arguments, and a list of other arguments that must be passed through to the real linker.
fn parse_arguments() -> (Args, Vec<String>) {
    let mut result_library_path = Vec::new();
    let mut result_shared_libraries = HashSet::new();
    let mut result_passthrough = Vec::new();

    let mut cargo_apk_gcc: Option<String> = None;
    let mut cargo_apk_gcc_sysroot: Option<String> = None;
    let mut cargo_apk_native_app_glue: Option<String> = None;
    let mut cargo_apk_linker_output: Option<String> = None;
    let mut cargo_apk_libs_path_output: Option<String> = None;
    let mut cargo_apk_libs_output: Option<String> = None;

    let mut args = RustCArgsReader::new();

    loop {
        let arg = match args.next() {
            Some(arg) => arg,
            None => {
                let args = Args {
                    library_path: result_library_path,
                    shared_libraries: result_shared_libraries,
                    cargo_apk_gcc: cargo_apk_gcc
                        .expect("Missing cargo_apk_gcc option in linker"),
                    cargo_apk_gcc_sysroot: cargo_apk_gcc_sysroot
                        .expect("Missing cargo_apk_gcc_sysroot option in linker"),
                    cargo_apk_native_app_glue: cargo_apk_native_app_glue
                        .expect("Missing cargo_apk_native_app_glue option in linker"),
                    cargo_apk_linker_output: cargo_apk_linker_output
                        .expect("Missing cargo_apk_linker_output option in linker"),
                    cargo_apk_libs_path_output: cargo_apk_libs_path_output
                        .expect("Missing cargo_apk_libs_path_output option in linker"),
                    cargo_apk_libs_output: cargo_apk_libs_output
                        .expect("Missing cargo_apk_libs_output option in linker"),
                };

                return (args, result_passthrough);
            }
        };

        match &*arg {
            "--cargo-apk-gcc" => {
                cargo_apk_gcc = Some(args.next().unwrap());
            },
            "--cargo-apk-gcc-sysroot" => {
                cargo_apk_gcc_sysroot = Some(args.next().unwrap());
            },
            "--cargo-apk-native-app-glue" => {
                cargo_apk_native_app_glue = Some(args.next().unwrap());
            },
            "--cargo-apk-linker-output" => {
                cargo_apk_linker_output = Some(args.next().unwrap());
            },
            "--cargo-apk-libs-path-output" => {
                cargo_apk_libs_path_output = Some(args.next().unwrap());
            },
            "--cargo-apk-libs-output" => {
                cargo_apk_libs_output = Some(args.next().unwrap());
            },

            "-o" => {
                // Ignore `-o` and the following argument
                args.next();
            },
            "-L" => {
                let path = args.next().expect("-L must be followed by a path");
                result_library_path.push(PathBuf::from(path.clone()));

                // Also pass these through.
                result_passthrough.push(arg);
                result_passthrough.push(path);
            },
            "-l" => {
                let name = args.next().expect("-l must be followed by a library name");
                result_shared_libraries.insert(vec!["lib", &name, ".so"].concat());

                // Also pass these through.
                result_passthrough.push(arg);
                result_passthrough.push(name);
            }
            _ => {
                if arg.starts_with("-l") {
                    result_shared_libraries.insert(vec!["lib", &arg[2..], ".so"].concat());
                }

                // Also pass these through.
                result_passthrough.push(arg);
            }
        };
    }
}

struct RustCArgsReader {
    reader: Option<BufReader<File>>,
    args: env::Args
}

impl RustCArgsReader {
    pub fn new() -> RustCArgsReader {
        let mut args = env::args();
        args.next();

        RustCArgsReader {
            reader: None,
            args
        }
    }

    pub fn next(&mut self) -> Option<String> {

        let mut replace = false;
        let mut next_reader : Option<BufReader<File>> = None;

        loop {
            if replace {
                self.reader = next_reader.take();
            }

            if let Some(ref mut r) = &mut self.reader {
                let mut line = String::new();
                match r.read_line(&mut line) {
                    Ok(size) => {
                        if size == 0 {
                            replace = true;
                            next_reader = None;
                            continue;
                        }

                        if line.ends_with('\n') {
                            line.pop();
                        }

                        if line.ends_with('\r') {
                            line.pop();
                        }

                        return Some(line)
                    },
                    Err(e) => {
                        println!("Error on file read: {}", e);
                        replace = true;
                        next_reader = None;
                        continue;
                    }
                }
            } else {
                match self.args.next() {
                    Some(arg) => {
                        if arg.starts_with("@") {
                            let file = match File::open(&arg[1..]) {
                                Ok(f) => f,
                                Err(e) => {
                                    println!("Error on file open: {}", e);
                                    continue;
                                }
                            };

                            replace = true;
                            next_reader = Some(BufReader::new(file));
                            continue;
                        } else {
                            return Some(arg);
                        }
                    }
                    None => return None
                }
            }
        }
    }
}
