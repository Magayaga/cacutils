use std::collections::HashMap;
use std::env;
use std::io::{self, Write};

mod cat;
mod cd;
mod cp;
mod color;
mod ls;
mod sleep;
// mod time;

use crate::cat::cat_command;
use crate::cd::cd_command;
use crate::cp::cp_command;
use crate::color::{colorize, ANSIColors};
use crate::ls::ls_command;
use crate::sleep::sleep_command;
// use crate::time::time_command;

// Define a struct to represent a command
struct Command {
    name: String,
    handler: fn(Vec<String>),
}

// Define a struct to represent the shell
struct Shell {
    commands: HashMap<String, Command>,
}

impl Shell {
    // Function to register commands
    fn register(&mut self, command: Command) {
        self.commands.insert(command.name.clone(), command);
    }
    
    // Function to execute a command
    fn execute(&self, command_name: &str, arguments: Vec<String>) {
        if let Some(command) = self.commands.get(command_name) {
            (command.handler)(arguments);
        }
        
        else {
            println!("Command not found: {}", command_name);
        }
    }
    
    // Function to get the current directory path
    fn get_current_directory() -> String {
        env::current_dir().unwrap().display().to_string()
    }
    
    // Function to start the shell
    fn start(&self) {
        println!("Welcome to Cacutils Shell!");
        loop {
            let username = whoami::username();
            let os_name = sys_info::os_type().unwrap();
            let directory_path = colorize(&Shell::get_current_directory(), ANSIColors::BLUE);
            let username_colored = colorize(&username, ANSIColors::GREEN);
            print!("{}@{}: {} $ ", username_colored, os_name, directory_path);
            io::stdout().flush().unwrap();
            
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            let components: Vec<&str> = input.trim().split_whitespace().collect();
            if let Some(command_name) = components.first() {
                let arguments = components.iter().skip(1).map(|&s| s.to_string()).collect();
                self.execute(command_name, arguments);
            }
        }
    }    
}

// Example commands
fn help_command(_arguments: Vec<String>) {
    println!("Available commands:");
    println!("help - Display available commands");
    println!("hello - Print Hello World!");
    println!("cat <file> [OPTION]... - Display content of a file");
    println!("ls [OPTION]... [FILE]... - List directory contents");
}

fn hello_command(_arguments: Vec<String>) {
    println!("Hello, World!");
}

fn main() {
    // Create a shell instance
    let mut shell = Shell {
        commands: HashMap::new(),
    };

    // Register commands
    shell.register(Command {
        name: "help".to_string(),
        handler: help_command,
    });

    shell.register(Command {
        name: "hello".to_string(),
        handler: hello_command,
    });

    shell.register(Command {
        name: "cat".to_string(),
        handler: cat_command,
    });

    shell.register(Command {
        name: "cd".to_string(),
        handler: cd_command,
    });

    shell.register(Command {
        name: "cp".to_string(),
        handler: cp_command,
    });
    
    shell.register(Command {
        name: "ls".to_string(),
        handler: ls_command,
    });

    shell.register(Command {
        name: "sleep".to_string(),
        handler: sleep_command,
    });

    /*
    shell.register(Command {
        name: "time".to_string(),
        handler: time_command,
    });
    */

    // Start the shell
    shell.start();
}
