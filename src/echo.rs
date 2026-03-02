// Function to implement the 'echo' command
pub fn echo_command(arguments: Vec<String>) {
    // Check if --help or --version option is provided
    if arguments.contains(&String::from("--help")) {
        echo_print_usage();
        return;
    }

    else if arguments.contains(&String::from("--version")) {
        echo_print_version();
        return;
    }

    let mut no_newline = false;
    let mut interpret_escapes = false;
    let mut tokens: Vec<String> = Vec::new();

    for arg in &arguments {
        match arg.as_str() {
            "-n" => no_newline = true,
            "-e" => interpret_escapes = true,
            "-E" => interpret_escapes = false,
            _ => tokens.push(arg.clone()),
        }
    }

    let mut output = tokens.join(" ");

    if interpret_escapes {
        output = output
            .replace("\\\\", "\x00BACKSLASH\x00")
            .replace("\\n", "\n")
            .replace("\\t", "\t")
            .replace("\\r", "\r")
            .replace("\\a", "\x07")
            .replace("\\b", "\x08")
            .replace("\\v", "\x0B")
            .replace("\x00BACKSLASH\x00", "\\");
    }

    if no_newline {
        print!("{}", output);
    } else {
        println!("{}", output);
    }
}

// Function to print usage instructions
fn echo_print_usage() {
    println!("Usage: echo [OPTION]... [STRING]...");
    println!("Echo the STRING(s) to standard output.");
    println!("\nOptions:");
    println!("  -n          do not output the trailing newline");
    println!("  -e          enable interpretation of backslash escapes");
    println!("  -E          disable interpretation of backslash escapes (default)");
    println!("  --help      display this help and exit");
    println!("  --version   output version information and exit");
    println!("\nEscape sequences (with -e):");
    println!("  \\\\   backslash");
    println!("  \\a   alert (BEL)");
    println!("  \\b   backspace");
    println!("  \\n   new line");
    println!("  \\r   carriage return");
    println!("  \\t   horizontal tab");
    println!("  \\v   vertical tab");
}

// Function to print version information
fn echo_print_version() {
    println!("echo (cacutils) v1.0");
    println!("There is NO WARRANTY, to the extent permitted by law.");
    println!("Written by Cyril John Magayaga.");
}
