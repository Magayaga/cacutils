package main

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

// Function to implement the 'cd' command
func cdCommand(arguments []string) {
	// Check if --help or --version option is provided
	if cdcontains(arguments, "--help") {
		cdPrintUsage()
		return
	} else if cdcontains(arguments, "--version") {
		cdPrintVersion()
		return
	}

	// Get the directory path from arguments
	var directoryPath string
	if len(arguments) > 0 {
		directoryPath = arguments[0]
	} else {
		fmt.Println("cd: missing directory")
		return
	}

	// Resolve and change directory
	resolvedPath, err := resolveDirectoryPath(directoryPath, arguments)
	if err != nil {
		fmt.Printf("cd: %s\n", err)
		return
	}

	err = os.Chdir(resolvedPath)
	if err != nil {
		fmt.Printf("cd: %s\n", err)
	}
}

// Function to resolve the directory path based on options
func resolveDirectoryPath(directoryPath string, arguments []string) (string, error) {
	resolvedPath := directoryPath

	// Apply options
	if cdcontains(arguments, "-L") {
		// Follow symbolic links
		targetPath, err := os.Readlink(resolvedPath)
		if err != nil {
			return "", fmt.Errorf("%s: %s", directoryPath, err)
		}
		resolvedPath = targetPath
	}

	if cdcontains(arguments, "-P") {
		// Use physical directory structure
		canonicalPath, err := filepath.EvalSymlinks(resolvedPath)
		if err != nil {
			return "", fmt.Errorf("%s: No such file or directory", directoryPath)
		}
		resolvedPath = canonicalPath
	}

	if strings.Contains(directoryPath, "~") {
		// Use home directory
		homeDir, err := os.UserHomeDir()
		if err != nil {
			return "", fmt.Errorf("%s: No such file or directory", directoryPath)
		}
		resolvedPath = homeDir
	}

	// Check if the path exists
	if cdcontains(arguments, "-e") {
		if _, err := os.Stat(resolvedPath); os.IsNotExist(err) {
			return "", fmt.Errorf("%s: No such file or directory", directoryPath)
		}
	}

	return resolvedPath, nil
}

// Function to check if a string slice contains a specific string
func cdcontains(slice []string, str string) bool {
	for _, s := range slice {
		if s == str {
			return true
		}
	}
	return false
}

// Function to print usage instructions
func cdPrintUsage() {
	fmt.Println("Usage: cd [OPTION]... DIRECTORY")
	fmt.Println("Change the shell working directory to DIRECTORY.")
	fmt.Println("\nOptions:")
	fmt.Println("  -L             force symbolic links to be followed")
	fmt.Println("  -P             use the physical directory structure")
	fmt.Println("  -e             check if the directory exists before changing")
	fmt.Println("  -@             print symbolic links resolved name")
	fmt.Println("  --help         display this help and exit")
	fmt.Println("  --version      output version information and exit")
}

// Function to print version information
func cdPrintVersion() {
	fmt.Println("cd (cacutils) v1.0")
	fmt.Println("There is NO WARRANTY, to the extent permitted by law.")
	fmt.Println("Written by Cyril John Magayaga.")
}
