import Foundation

// Define a struct to represent a command
struct Command {
    let name: String
    let handler: ([String]) -> Void // Change the handler to accept arguments
}

// Define a struct to represent the shell
struct Shell {
    var commands: [String: Command]
    
    // Function to register commands
    mutating func register(command: Command) {
        commands[command.name] = command
    }
    
    // Function to execute a command
    func execute(commandName: String, arguments: [String]) {
        if let command = commands[commandName] {
            command.handler(arguments)
        } else {
            print("Command not found: \(commandName)")
        }
    }
    
    // Function to start the shell
    func start() {
        print("Welcome to Swift Shell!")
        while true {
            print("Enter a command:")
            if let input = readLine() {
                let components = input.split(separator: " ")
                if let commandName = components.first {
                    let arguments = components.dropFirst().map(String.init)
                    execute(commandName: String(commandName), arguments: arguments)
                }
            }
        }
    }
}

// Example commands
func help_command(arguments: [String]) {
    print("Available commands:")
    print("help - Display available commands")
    print("hello - Print Hello World!")
    print("cat <file> [OPTION]... - Display content of a file")
    print("ls [OPTION]... [FILE]... - List directory contents")
}

func hello_command(arguments: [String]) {
    print("Hello, World!")
}

// Create a shell instance
var shell = Shell(commands: [:])

// Register commands
shell.register(command: Command(name: "help", handler: help_command))
shell.register(command: Command(name: "hello", handler: hello_command))

// Register the cat command
shell.register(command: Command(name: "cat", handler: cat_command))

// Register the ls command
shell.register(command: Command(name: "ls", handler: ls_command))

// Start the shell
shell.start()
