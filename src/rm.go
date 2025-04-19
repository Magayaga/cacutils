package main

import (
	"fmt"
	"os"
)

// Options holds command options
type Options struct {
	recursive bool
	verbose   bool
}

// rmCommand implements the 'rm' command
func rmCommand(arguments []string) {
	if rm_contains(arguments, "--help") {
		rmPrintUsage()
		return
	}

	if rm_contains(arguments, "--version") {
		rmPrintVersion()
		return
	}

	if len(arguments) < 1 {
		fmt.Fprintln(os.Stderr, "Usage: rm [OPTION]... FILE...")
		return
	}

	options, files := parseArguments(arguments)

	for _, file := range files {
		if options.recursive {
			if err := removeRecursive(file, options.verbose); err != nil {
				fmt.Fprintf(os.Stderr, "Error removing directory %s: %v\n", file, err)
			}
		} else {
			if err := removeFile(file, options.verbose); err != nil {
				fmt.Fprintf(os.Stderr, "Error removing file %s: %v\n", file, err)
			}
		}
	}
}

// contains checks if a slice contains a string
func rm_contains(slice []string, item string) bool {
	for _, s := range slice {
		if s == item {
			return true
		}
	}
	return false
}

// parseArguments parses options and files
func parseArguments(arguments []string) (Options, []string) {
	options := Options{}
	files := []string{}

	for _, arg := range arguments {
		switch arg {
		case "-r", "--recursive":
			options.recursive = true
		case "-v", "--verbose":
			options.verbose = true
		default:
			files = append(files, arg)
		}
	}

	return options, files
}

// removeFile removes a single file
func removeFile(file string, verbose bool) error {
	info, err := os.Stat(file)
	if err != nil {
		return err
	}

	if info.IsDir() {
		fmt.Fprintf(os.Stderr, "%s is a directory. Use -r to remove directories.\n", file)
		return fmt.Errorf("is a directory")
	}

	if err := os.Remove(file); err != nil {
		return err
	}

	if verbose {
		fmt.Printf("Removed %s\n", file)
	}

	return nil
}

// removeRecursive recursively removes a directory or file
func removeRecursive(file string, verbose bool) error {
	info, err := os.Stat(file)
	if err != nil {
		return err
	}

	if info.IsDir() {
		if err := os.RemoveAll(file); err != nil {
			return err
		}
	} else {
		return removeFile(file, verbose)
	}

	if verbose {
		fmt.Printf("Removed %s\n", file)
	}

	return nil
}

// rmPrintUsage prints usage instructions
func rmPrintUsage() {
	fmt.Println("Usage: rm [OPTION]... FILE...")
	fmt.Println("Remove the FILE(s).")
	fmt.Println("\nOptions:")
	fmt.Println("  --help           display this help and exit")
	fmt.Println("  --version        output version information and exit")
	fmt.Println("  -r, --recursive  remove directories and their contents recursively")
	fmt.Println("  -v, --verbose    explain what is being done")
}

// rmPrintVersion prints version information
func rmPrintVersion() {
	fmt.Println("rm (cacutils) v1.0")
	fmt.Println("There is NO WARRANTY, to the extent permitted by law.")
	fmt.Println("Written by Cyril John Magayaga.")
}
