use std::fs;
use std::path::Path;
use chrono::DateTime;
use chrono::Local;

// Function to implement the 'ls' command
pub fn ls_command(arguments: Vec<String>) {
    // Check if --help or --version option is provided
    if arguments.contains(&String::from("--help")) {
        ls_print_usage();
        return;
    }
    
    else if arguments.contains(&String::from("--version")) {
        ls_print_version();
        return;
    }
    
    // Get the directory path (if provided) or use the current directory
    let directory_path = match arguments.get(1) {
        Some(path) => Path::new(path),
        None => Path::new("."),
    };
    
    // Set default block size
    let mut block_size = 1024;
    
    // Determine block size if provided in arguments
    if let Some(block_size_arg) = arguments.iter().find(|arg| arg.starts_with("--block-size=")) {
        if let Some(size_str) = block_size_arg.split('=').last() {
            if let Ok(size) = size_str.parse::<usize>() {
                block_size = size;
            }
        }
    }
    
    match fs::read_dir(directory_path) {
        Ok(entries) => {
            for entry in entries {
                if let Ok(entry) = entry {
                    let metadata = entry.metadata().unwrap();
                    let file_name = entry.file_name().into_string().unwrap();
                    if let Ok(created) = metadata.created() {
                        let created_datetime: DateTime<Local> = created.into();
                        let formatted_date = created_datetime.format("%b %d %H:%M");
                        
                        let formatted_size = format_size(metadata.len() as usize, block_size);
                        
                        let mut formatted_details = format!("{} {} {}", formatted_size, formatted_date, file_name);
                        
                        // Include author if --author option is provided
                        if arguments.contains(&String::from("--author")) {
                            if let Some(uid) = metadata.uid() {
                                if let Some(username) = users::get_user_by_uid(uid) {
                                    formatted_details += &format!(" {}", username.name().to_string_lossy());
                                }
                            }
                        }
                        
                        println!("{}", formatted_details);
                    }
                }
            }
        }
        Err(err) => {
            eprintln!("Error listing directory contents: {}", err);
        }
    }
}

// Function to format file size with block size
fn format_size(file_size: usize, block_size: usize) -> String {
    if file_size < block_size {
        format!("{}B", file_size)
    }
    
    else if file_size < block_size * block_size {
        format!("{}KB", file_size / block_size)
    }
    
    else {
        format!("{}MB", file_size / (block_size * block_size))
    }
}

// Function to print usage instructions
fn ls_print_usage() {
    println!("Usage: ls [OPTION]... [FILE]...");
    println!("List information about the FILEs (the current directory by default).");
    println!("\nOptions:");
    println!("  -a, --all         do not ignore entries starting with .");
    println!("  -l                use a long listing format");
    println!("  -la               list all files in long format");
    println!("      --author      with -l, print the author of each file");
    println!("  -b, --escape      with -b, print octal escapes for nongraphic characters");
    println!("      --block-size=SIZE  with -l, scale sizes by SIZE when printing them");
    println!("  -d, --directory   list directory entries instead of contents");
    println!("      --color       colorize the output");
    println!("  --help            display this help and exit");
    println!("  --version         output version information and exit");
}

// Function to print version information
fn ls_print_version() {
    println!("ls (cacutils) v1.0");
    println!("There is NO WARRANTY, to the extent permitted by law.");
    println!("Written by Cyril John Magayaga.");
}
