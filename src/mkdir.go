package main

import (
	"fmt"
	"os"
)

// mkdirCommand implements the 'mkdir' command
func mkdirCommand(arguments []string) {
	if mkdir_contains(arguments, "--help") {
		mkdirPrintUsage()
		return
	}

	if mkdir_contains(arguments, "--version") {
		mkdirPrintVersion()
		return
	}

	if len(arguments) == 0 {
		fmt.Fprintln(os.Stderr, "Usage: mkdir [OPTION]... DIRECTORY...")
		return
	}

	paths := []string{}
	createParents := false
	verbose := false

	for _, arg := range arguments {
		switch arg {
		case "-p", "--parents":
			createParents = true
		case "-v", "--verbose":
			verbose = true
		default:
			paths = append(paths, arg)
		}
	}

	for _, path := range paths {
		var err error
		if createParents {
			err = os.MkdirAll(path, 0755)
		} else {
			err = os.Mkdir(path, 0755)
		}

		if err != nil {
			fmt.Fprintf(os.Stderr, "mkdir: cannot create directory '%s': %v\n", path, err)
		} else {
			if verbose {
				fmt.Printf("created directory: %s\n", path)
			}
		}
	}
}

// mkdirPrintUsage prints usage instructions
func mkdirPrintUsage() {
	fmt.Println(`Usage: mkdir [OPTION]... DIRECTORY...
Create the DIRECTORY(ies), if they do not already exist.

Options:
  -p, --parents   no error if existing, make parent directories as needed
  -v, --verbose   print a message for each created directory
  --help          display this help and exit
  --version       output version information and exit`)
}

// mkdirPrintVersion prints version information
func mkdirPrintVersion() {
	fmt.Println("mkdir (cacutils) v1.0")
	fmt.Println("There is NO WARRANTY, to the extent permitted by law.")
	fmt.Println("Written by Cyril John Magayaga.")
}

// contains checks if a slice contains a string
func mkdir_contains(slice []string, item string) bool {
	for _, s := range slice {
		if s == item {
			return true
		}
	}
	return false
}