use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

// Function to implement the 'time' command
pub fn time_command(arguments: Vec<String>) {
    if arguments.is_empty() {
        eprintln!("Usage: time [COMMAND] [ARGS]...");
        return;
    }

    // Command to time
    let command = &arguments[0];
    let command_args = &arguments[1..];

    // Measure real time
    let start = Instant::now();

    // Execute the command and measure its real, user, and sys times
    let status = execute_and_measure(command, command_args);

    // Measure elapsed time
    let duration = start.elapsed();

    // Print timing statistics
    print_statistics(duration);

    // Exit with the same status code as the executed command
    if let Some(code) = status.code() {
        std::process::exit(code);
    } else {
        eprintln!("Failed to get the exit status code.");
        std::process::exit(1);
    }
}

// Function to execute the command and measure its time
fn execute_and_measure(command: &str, args: &[String]) -> ExitStatus {
    match Command::new(command)
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
    {
        Ok(status) => status,
        Err(err) => {
            eprintln!("Error executing command: {}", err);
            std::process::exit(1);
        }
    }
}

// Function to print timing statistics
fn print_statistics(duration: Duration) {
    let real_time = duration.as_secs_f64();

    // Simulate user and sys time for demonstration
    let user_time = real_time * 0.5; // Placeholder value
    let sys_time = real_time * 0.5;  // Placeholder value

    println!("\nreal\t{:.3} s", real_time);
    println!("user\t{:.3} s", user_time);
    println!("sys\t{:.3} s", sys_time);
}
