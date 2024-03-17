use std::env;
use std::fs;
use std::path::{PathBuf};

// Function to implement the 'cd' command
pub fn cd_command(arguments: Vec<String>) {
    // Check if --help or --version option is provided
    if arguments.contains(&String::from("--help")) {
        cd_print_usage();
        return;
    }
    
    else if arguments.contains(&String::from("--version")) {
        cd_print_version();
        return;
    }

    // Get the directory path from arguments
    let directory_path = match arguments.get(0) {
        Some(path) => path,
        None => {
            println!("cd: missing directory");
            return;
        }
    };

    // Resolve and change directory
    if let Some(resolved_path) = resolve_directory_path(directory_path, &arguments) {
        if let Err(err) = env::set_current_dir(&resolved_path) {
            println!("cd: {}", err);
        }
    }
}

// Function to resolve the directory path based on options
fn resolve_directory_path(directory_path: &str, arguments: &Vec<String>) -> Option<PathBuf> {
    let mut resolved_path = PathBuf::from(directory_path);

    // Apply options
    if arguments.contains(&String::from("-L")) {
        // Follow symbolic links
        match fs::read_link(&resolved_path) {
            Ok(target_path) => resolved_path = target_path,
            Err(err) => {
                println!("cd: {}: {}", directory_path, err);
                return None;
            }
        }
    }
    
    else if arguments.contains(&String::from("-P")) {
        // Use physical directory structure
        if let Ok(canonical_path) = resolved_path.canonicalize() {
            resolved_path = canonical_path;
        }
        
        else {
            println!("cd: {}: No such file or directory", directory_path);
            return None;
        }
    }
    
    else if arguments.contains(&String::from("~")) {
        // Use home directory
        if let Some(home_dir) = dirs::home_dir() {
            resolved_path = home_dir;
        }
        
        else {
            println!("cd: {}: No such file or directory", directory_path);
            return None;
        }
    }

    // Check if the path exists
    if arguments.contains(&String::from("-e")) && !resolved_path.exists() {
        println!("cd: {}: No such file or directory", directory_path);
        return None;
    }

    Some(resolved_path)
}

// Function to print usage instructions
fn cd_print_usage() {
    println!("Usage: cd [OPTION]... DIRECTORY");
    println!("Change the shell working directory to DIRECTORY.");
    println!("\nOptions:");
    println!("  -L             force symbolic links to be followed");
    println!("  -P             use the physical directory structure");
    println!("  -e             check if the directory exists before changing");
    println!("  -@             print symbolic links resolved name");
    println!("  --help         display this help and exit");
    println!("  --version      output version information and exit");
}

// Function to print version information
fn cd_print_version() {
    println!("cd (cacutils) v1.0");
    println!("There is NO WARRANTY, to the extent permitted by law.");
    println!("Written by Cyril John Magayaga.");
}
