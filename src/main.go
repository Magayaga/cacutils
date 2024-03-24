package main

import (
	"bufio"
	"fmt"
	"os"
	"os/user"
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
		fmt.Printf("%s@%s:%s $ ", user.Username, "your_os_name", directoryPath)

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
func help_command(arguments []string) {
	fmt.Println("Available commands:")
	fmt.Println("help - Display available commands")
	fmt.Println("hello - Print Hello World!")
}

func hello_command(arguments []string) {
	fmt.Println("Hello, World!")
}

func main() {
	shell := Shell{commands: make(map[string]Command)}

	// Register the help command
	shell.register(Command{name: "help", handler: help_command})

	// Register the hello command
	shell.register(Command{name: "hello", handler: hello_command})

	// Start the shell
	shell.start()
}
