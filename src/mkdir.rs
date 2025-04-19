use std::env;
use std::fs::{self, DirBuilder};
use std::path::Path;

pub fn mkdir_command(arguments: Vec<String>) {
    if arguments.contains(&String::from("--help")) {
        mkdir_print_usage();
        return;
    }

    if arguments.contains(&String::from("--version")) {
        mkdir_print_version();
        return;
    }

    if arguments.is_empty() {
        eprintln!("Usage: mkdir [OPTION]... DIRECTORY...");
        return;
    }

    let mut paths = Vec::new();
    let mut create_parents = false;
    let mut verbose = false;

    for arg in arguments {
        match arg.as_str() {
            "-p" | "--parents" => create_parents = true,
            "-v" | "--verbose" => verbose = true,
            _ => paths.push(arg),
        }
    }

    for path in paths {
        let path = Path::new(&path);
        let result = if create_parents {
            DirBuilder::new().recursive(true).create(path)
        } else {
            fs::create_dir(path)
        };

        match result {
            Ok(_) => {
                if verbose {
                    println!("created directory: {}", path.display());
                }
            }
            Err(e) => {
                eprintln!("mkdir: cannot create directory '{}': {}", path.display(), e);
            }
        }
    }
}

fn mkdir_print_usage() {
    println!("Usage: mkdir [OPTION]... DIRECTORY...");
    println!("Create the DIRECTORY(ies), if they do not already exist.\n");
    println!("Options:");
    println!("  -p, --parents   no error if existing, make parent directories as needed");
    println!("  -v, --verbose   print a message for each created directory");
    println!("  --help          display this help and exit");
    println!("  --version       output version information and exit");
}

fn mkdir_print_version() {
    println!("mkdir (cacutils) v1.0");
    println!("There is NO WARRANTY, to the extent permitted by law.");
    println!("Written by Cyril John Magayaga.");
}
