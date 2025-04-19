package main

import (
	"fmt"
	"os"
	"os/exec"
	"time"
)

// Function to measure and print the execution time of a command
func timeCommand(arguments []string) {
	if len(arguments) == 0 {
		fmt.Fprintln(os.Stderr, "Usage: time [COMMAND] [ARGS]...")
		return
	}

	commandName := arguments[0]
	args := arguments[1:]

	startTime := time.Now()

	cmd := exec.Command(commandName, args...)
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr

	err := cmd.Run()

	duration := time.Since(startTime)

	if err == nil {
		fmt.Printf("\nreal: %.2fs\n", duration.Seconds())
		fmt.Println("user: n/a")
		fmt.Println("sys: n/a")
	} else {
		fmt.Printf("\nreal: %.2fs\n", duration.Seconds())
		fmt.Println("user: n/a")
		fmt.Println("sys: n/a")
		fmt.Fprintf(os.Stderr, "Failed to execute command %s: %v\n", commandName, err)
	}
}
