use std::env;
use std::thread;
use std::time::Duration;

// Function to implement the 'sleep' command
pub fn sleep_command(arguments: Vec<String>) {
    // Check if --help or --version option is provided
    if arguments.contains(&String::from("--help")) {
        sleep_print_usage();
        return;
    }
    
    else if arguments.contains(&String::from("--version")) {
        sleep_print_version();
        return;
    }

    // Check if any operand is provided
    if arguments.is_empty() {
        println!("sleep: missing operand");
        println!("Try 'sleep --help' for more information.");
        return;
    }

    // Get the duration from arguments
    let duration: u64 = parse_duration(&arguments);

    // Sleep for the specified duration
    thread::sleep(Duration::from_secs(duration));
}

// Function to parse the duration from arguments
fn parse_duration(arguments: &[String]) -> u64 {
    let duration_string = &arguments[0];
    let mut duration = 0;

    // Parse the duration string
    if let Some(index) = duration_string.chars().position(|c| c.is_alphabetic()) {
        let (number_string, unit_string) = duration_string.split_at(index);
        if let Ok(number) = number_string.parse::<u64>() {
            let unit = match unit_string {
                "s" => 1,
                "m" => 60,
                "h" => 3600,
                "d" => 86400,
                _ => 1, // Default to seconds
            };
            duration = number * unit;
        } else {
            println!("Invalid duration format.");
        }
    }
    
    else if let Ok(number) = duration_string.parse::<u64>() {
        // Default to seconds if no unit is specified
        duration = number;
    }
    
    else {
        println!("Invalid duration format.");
    }

    duration
}

// Function to print usage instructions
fn sleep_print_usage() {
    println!("Usage: sleep [NUMBER][s|m|h|d]");
    println!("Pause execution for NUMBER seconds, minutes, hours, or days.");
    println!("\nOptions:");
    println!("  --help     display this help and exit");
    println!("  --version  output version information and exit");
}

// Function to print version information
fn sleep_print_version() {
    println!("sleep (cacutils) v1.0");
    println!("There is NO WARRANTY, to the extent permitted by law.");
    println!("Written by Cyril John Magayaga.");
}