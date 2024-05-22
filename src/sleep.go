package main

import (
	"fmt"
	"strconv"
	"strings"
	"time"
)

// Function to implement the 'sleep' command
func sleepCommand(arguments []string) {
	// Check if --help or --version option is provided
	if sleepContains(arguments, "--help") {
		sleepPrintUsage()
		return
	} else if sleepContains(arguments, "--version") {
		sleepPrintVersion()
		return
	}

	// Check if any operand is provided
	if len(arguments) == 0 {
		fmt.Println("sleep: missing operand")
		fmt.Println("Try 'sleep --help' for more information.")
		return
	}

	// Get the duration from arguments
	duration := parseDuration(arguments)

	// Sleep for the specified duration
	time.Sleep(time.Duration(duration) * time.Second)
}

// Function to check if a slice contains a specific element
func sleepContains(slice []string, item string) bool {
	for _, element := range slice {
		if element == item {
			return true
		}
	}
	return false
}

// Function to parse the duration from arguments
func parseDuration(arguments []string) uint64 {
	durationString := arguments[0]
	var duration uint64

	// Parse the duration string
	index := strings.IndexFunc(durationString, func(c rune) bool {
		return !('0' <= c && c <= '9')
	})
	if index != -1 {
		numberString := durationString[:index]
		unitString := durationString[index:]

		number, err := strconv.ParseUint(numberString, 10, 64)
		if err != nil {
			fmt.Println("Invalid duration format.")
			return 0
		}

		unit := map[string]uint64{
			"s": 1,
			"m": 60,
			"h": 3600,
			"d": 86400,
		}[unitString]

		duration = number * unit
	} else {
		number, err := strconv.ParseUint(durationString, 10, 64)
		if err != nil {
			fmt.Println("Invalid duration format.")
			return 0
		}
		duration = number
	}

	return duration
}

// Function to print usage instructions
func sleepPrintUsage() {
	fmt.Println("Usage: sleep [NUMBER][s|m|h|d]")
	fmt.Println("Pause execution for NUMBER seconds, minutes, hours, or days.")
	fmt.Println("\nOptions:")
	fmt.Println("  --help     display this help and exit")
	fmt.Println("  --version  output version information and exit")
}

// Function to print version information
func sleepPrintVersion() {
	fmt.Println("sleep (cacutils) v1.0")
	fmt.Println("There is NO WARRANTY, to the extent permitted by law.")
	fmt.Println("Written by Cyril John Magayaga.")
}
