use std::process::{Stdio};
use std::time::{Instant};
use crate::ProcessCommand;

// Function to measure and print the execution time of a command
pub fn time_command(arguments: Vec<String>) {
    if arguments.is_empty() {
        eprintln!("Usage: time [COMMAND] [ARGS]...");
        return;
    }

    let command_name = &arguments[0];
    let args = &arguments[1..];

    let start_time = Instant::now();

    let status = ProcessCommand::new(command_name)
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();

    let duration = start_time.elapsed();

    match status {
        Ok(status) => {
            println!("\nreal: {:.2?}", duration);
            // Note: `user` and `sys` times are typically platform-specific and not available directly in Rust's standard library.
            println!("user: n/a");
            println!("sys: n/a");

            if !status.success() {
                eprintln!("Command {} failed with status {}", command_name, status);
            }
        }
        Err(err) => {
            eprintln!("Failed to execute command {}: {}", command_name, err);
        }
    }
}
