package main

import (
	"bufio"
	"fmt"
	"os"
	"os/user"
	"runtime"
	"strings"
)

// Command represents a shell command
type Command struct {
	name    string
	handler func([]string)
}

// Shell represents a shell instance
type Shell struct {
	commands map[string]Command
}

// Register registers a command with the shell
func (s *Shell) register(command Command) {
	s.commands[command.name] = command
}

// Execute executes a command
func (s Shell) execute(commandName string, arguments []string) {
	if command, ok := s.commands[commandName]; ok {
		command.handler(arguments)
	} else {
		fmt.Printf("Command not found: %s\n", commandName)
	}
}

// GetCurrentDirectory returns the current directory path
func getCurrentDirectory() string {
	dir, err := os.Getwd()
	if err != nil {
		fmt.Println(err)
	}
	return dir
}

// Start starts the shell
func (s *Shell) start() {
	user, err := user.Current()
	if err != nil {
		fmt.Println(err)
		return
	}
	fmt.Println("Welcome to Cacutils Shell!")

	for {
		directoryPath := getCurrentDirectory()
		fmt.Printf("%s@%s:%s $ ", user.Username, runtime.GOOS, directoryPath)

		reader := bufio.NewReader(os.Stdin)
		input, err := reader.ReadString('\n')
		if err != nil {
			fmt.Println(err)
			continue
		}

		input = strings.TrimSpace(input)
		components := strings.Fields(input)
		if len(components) == 0 {
			continue
		}

		commandName := components[0]
		arguments := components[1:]

		s.execute(commandName, arguments)
	}
}

// Example commands
func helpCommand(arguments []string) {
	fmt.Println("Available commands:")
	fmt.Println("help - Display available commands")
	fmt.Println("hello - Print Hello World!")
	fmt.Println("cat <file> [OPTION]... - Display content of a file")
}

func helloCommand(arguments []string) {
	fmt.Println("Hello, World!")
}

func main() {
	shell := Shell{commands: make(map[string]Command)}

	// Register the help command
	shell.register(Command{name: "help", handler: helpCommand})

	// Register the hello command
	shell.register(Command{name: "hello", handler: helloCommand})

	// Register the cat command
	shell.register(Command{name: "cat", handler: catCommand})

	// Register the cd command
	shell.register(Command{name: "cd", handler: cdCommand})

	// Register the ls command
	shell.register(Command{name: "ls", handler: lsCommand})

	// Register the sleep command
	shell.register(Command{name: "sleep", handler: sleepCommand})

	// Register the cp command
	shell.register(Command{name: "cp", handler: cpCommand})

	// Start the shell
	shell.start()
}
