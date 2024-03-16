use std::fs;

// Function to implement the 'cat' command
pub fn cat_command(arguments: Vec<String>) {
    // Check if --help or --version option is provided
    if arguments.contains(&String::from("--help")) {
        cat_print_usage();
        return;
    }
    
    else if arguments.contains(&String::from("--version")) {
        cat_print_version();
        return;
    }
    
    // Check if any file is provided
    let file_name = match arguments.get(0) {
        Some(name) => name,
        None => {
            println!("Usage: cat [OPTION]... [FILE]...");
            return;
        }
    };
    
    // Read the content of the file
    match fs::read_to_string(file_name) {
        Ok(content) => {
            // Apply options
            let formatted_content = apply_options(content, &arguments);
            println!("{}", formatted_content);
        },
        
        Err(_) => {
            println!("Error reading file: {}", file_name);
        }
    }
}

// Function to apply options to the content
fn apply_options(mut content: String, arguments: &Vec<String>) -> String {
    if arguments.contains(&String::from("-A")) || arguments.contains(&String::from("--show-all")) {
        content = content.replace("\n", "$\n").replace("\t", "^I");
    }
    
    if arguments.contains(&String::from("-b")) || arguments.contains(&String::from("--number-nonblank")) {
        let mut line_count = 1;
        content = content.split('\n').map(|line| {
            if !line.is_empty() {
                let numbered_line = format!("{}\t{}", line_count, line);
                line_count += 1;
                numbered_line
            } else {
                line.to_string()
            }
        }).collect::<Vec<String>>().join("\n");
    }
    
    if arguments.contains(&String::from("-e")) {
        content = content.replace("\n", "$\n");
    }
    
    if arguments.contains(&String::from("-E")) || arguments.contains(&String::from("--show-ends")) {
        content = content.replace("\n", "$\n") + "$";
    }
    
    if arguments.contains(&String::from("-n")) || arguments.contains(&String::from("--number")) {
        let mut line_count = 1;
        content = content.split('\n').map(|line| {
            let numbered_line = format!("{}\t{}", line_count, line);
            line_count += 1;
            numbered_line
        }).collect::<Vec<String>>().join("\n");
    }
    
    if arguments.contains(&String::from("-s")) || arguments.contains(&String::from("--squeeze-blank")) {
        content = content.replace("\n\n+", "\n");
    }
    
    if arguments.contains(&String::from("-t")) {
        content = content.replace("\t", "^I");
    }
    
    if arguments.contains(&String::from("-T")) || arguments.contains(&String::from("--show-tabs")) {
        content = content.replace("\t", "^I");
    }
    content
}

// Function to print usage instructions
fn cat_print_usage() {
    println!("Usage: cat [OPTION]... [FILE]...");
    println!("Concatenate FILE(s) to standard output.");
    println!("\nOptions:");
    println!("  --help            display this help and exit");
    println!("  -A, --show-all    equivalent to -vET");
    println!("  -b, --number-nonblank");
    println!("                    number nonempty output lines, overrides -n");
    println!("  -e                equivalent to -vE");
    println!("  -E, --show-ends   display $ at end of each line");
    println!("  -n, --number      number all output lines");
    println!("  -s, --squeeze-blank");
    println!("                    suppress repeated empty output lines");
    println!("  -t                equivalent to -vT");
    println!("  -T, --show-tabs   display TAB characters as ^I");
    println!("  --version         output version information and exit");
}

// Function to print version information
fn cat_print_version() {
    println!("cat (cacutils) v1.0");
    println!("There is NO WARRANTY, to the extent permitted by law.");
    println!("Written by Cyril John Magayaga.");
}
