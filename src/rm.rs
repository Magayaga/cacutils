use std::fs;
use std::path::Path;
use std::io;

// Function to implement the 'rm' command
pub fn rm_command(arguments: Vec<String>) {
    if arguments.contains(&String::from("--help")) {
        rm_print_usage();
        return;
    }

    if arguments.contains(&String::from("--version")) {
        rm_print_version();
        return;
    }

    if arguments.len() < 1 {
        eprintln!("Usage: rm [OPTION]... FILE...");
        return;
    }

    let (options, files) = parse_arguments(arguments);

    for file in files {
        if options.recursive {
            if let Err(e) = remove_recursive(&file, options.verbose) {
                eprintln!("Error removing directory {}: {}", file, e);
            }
        } else {
            if let Err(e) = remove_file(&file, options.verbose) {
                eprintln!("Error removing file {}: {}", file, e);
            }
        }
    }
}

// Structure to hold command options
struct Options {
    recursive: bool,
    verbose: bool,
}

// Function to parse arguments
fn parse_arguments(arguments: Vec<String>) -> (Options, Vec<String>) {
    let mut options = Options { recursive: false, verbose: false };
    let mut files = Vec::new();

    let mut args_iter = arguments.iter();
    while let Some(arg) = args_iter.next() {
        match arg.as_str() {
            "-r" | "--recursive" => options.recursive = true,
            "-v" | "--verbose" => options.verbose = true,
            _ => {
                files.push(arg.clone());
            }
        }
    }

    (options, files)
}

// Function to remove a single file
fn remove_file(file: &str, verbose: bool) -> io::Result<()> {
    let file_path = Path::new(file);

    if file_path.is_file() {
        fs::remove_file(file_path)?;
    } else if file_path.is_dir() {
        eprintln!("{} is a directory. Use -r to remove directories.", file);
        return Err(io::Error::new(io::ErrorKind::Other, "Is a directory"));
    }

    if verbose {
        println!("Removed {}", file);
    }

    Ok(())
}

// Function to recursively remove directories
fn remove_recursive(file: &str, verbose: bool) -> io::Result<()> {
    let file_path = Path::new(file);

    if file_path.is_dir() {
        fs::remove_dir_all(file_path)?;
    } else {
        return remove_file(file, verbose);
    }

    if verbose {
        println!("Removed {}", file);
    }

    Ok(())
}

// Function to print usage instructions
fn rm_print_usage() {
    println!("Usage: rm [OPTION]... FILE...");
    println!("Remove the FILE(s).");
    println!("\nOptions:");
    println!("  --help        display this help and exit");
    println!("  --version     output version information and exit");
    println!("  -r, --recursive  remove directories and their contents recursively");
    println!("  -v, --verbose  explain what is being done");
}

// Function to print version information
fn rm_print_version() {
    println!("rm (cacutils) v1.0");
    println!("There is NO WARRANTY, to the extent permitted by law.");
    println!("Written by Cyril John Magayaga.");
}
