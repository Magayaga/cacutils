use std::fs;
use std::path::Path;
use std::io::{self};
// Function to implement the 'cp' command
pub fn cp_command(arguments: Vec<String>) {
    if arguments.contains(&String::from("--help")) {
        cp_print_usage();
        return;
    }

    if arguments.contains(&String::from("--version")) {
        cp_print_version();
        return;
    }

    if arguments.len() < 2 {
        eprintln!("Usage: cp [OPTION]... SOURCE... DEST");
        return;
    }

    let (options, sources, destination) = parse_arguments(arguments);

    if sources.len() > 1 && !Path::new(&destination).is_dir() {
        eprintln!("When copying multiple files, the destination must be a directory.");
        return;
    }

    for source in sources {
        if options.recursive {
            if let Err(e) = copy_recursive(&source, &destination, options.verbose) {
                eprintln!("Error copying directory {}: {}", source, e);
            }
        } else {
            if let Err(e) = copy_file(&source, &destination, options.verbose) {
                eprintln!("Error copying file {}: {}", source, e);
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
fn parse_arguments(arguments: Vec<String>) -> (Options, Vec<String>, String) {
    let mut options = Options { recursive: false, verbose: false };
    let mut sources = Vec::new();
    let mut destination = String::new();

    let mut args_iter = arguments.iter();
    while let Some(arg) = args_iter.next() {
        match arg.as_str() {
            "-r" | "--recursive" => options.recursive = true,
            "-v" | "--verbose" => options.verbose = true,
            _ => {
                if destination.is_empty() {
                    sources.push(arg.clone());
                } else {
                    destination = arg.clone();
                }
            }
        }
    }

    if !sources.is_empty() && destination.is_empty() {
        destination = sources.pop().unwrap();
    }

    (options, sources, destination)
}

// Function to copy a single file
fn copy_file(source: &str, destination: &str, verbose: bool) -> io::Result<()> {
    let src_path = Path::new(source);
    let dest_path = Path::new(destination);

    let dest_path = if dest_path.is_dir() {
        dest_path.join(src_path.file_name().unwrap())
    } else {
        dest_path.to_path_buf()
    };

    fs::copy(&src_path, &dest_path)?;

    if verbose {
        println!("{} -> {}", source, dest_path.display());
    }

    Ok(())
}

// Function to recursively copy directories
fn copy_recursive(source: &str, destination: &str, verbose: bool) -> io::Result<()> {
    let src_path = Path::new(source);
    let dest_path = Path::new(destination);

    if !src_path.is_dir() {
        return copy_file(source, destination, verbose);
    }

    let new_dest_path = if dest_path.is_dir() {
        dest_path.join(src_path.file_name().unwrap())
    } else {
        dest_path.to_path_buf()
    };

    fs::create_dir_all(&new_dest_path)?;

    for entry in fs::read_dir(src_path)? {
        let entry = entry?;
        let entry_path = entry.path();
        let new_dest_path = new_dest_path.join(entry.file_name());

        if entry_path.is_dir() {
            copy_recursive(entry_path.to_str().unwrap(), new_dest_path.to_str().unwrap(), verbose)?;
        } else {
            fs::copy(&entry_path, &new_dest_path)?;
            if verbose {
                println!("{} -> {}", entry_path.display(), new_dest_path.display());
            }
        }
    }

    Ok(())
}

// Function to print usage instructions
fn cp_print_usage() {
    println!("Usage: cp [OPTION]... SOURCE... DEST");
    println!("Copy SOURCE to DEST, or multiple SOURCE(s) to DIRECTORY.");
    println!("\nOptions:");
    println!("  --help        display this help and exit");
    println!("  --version     output version information and exit");
    println!("  -r, --recursive  copy directories recursively");
    println!("  -v, --verbose  explain what is being done");
}

// Function to print version information
fn cp_print_version() {
    println!("cp (cputils) v1.0");
    println!("There is NO WARRANTY, to the extent permitted by law.");
    println!("Written by Cyril John Magayaga.");
}
