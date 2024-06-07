package main

import (
	"flag"
	"fmt"
	"io"
	"os"
	"path/filepath"
)

var (
	help            bool
	version         bool
	archive         bool
	attributesOnly  bool
	backup          string
	copyContents    bool
	d               bool
	force           bool
	interactive     bool
	h               bool
	link            bool
	dereference     bool
	noClobber       bool
	noDereference   bool
	preserve        string
	context         string
)

func init() {
	flag.BoolVar(&help, "help", false, "display this help and exit")
	flag.BoolVar(&version, "version", false, "output version information and exit")
	flag.BoolVar(&archive, "archive", false, "copy all files and directories recursively")
	flag.BoolVar(&attributesOnly, "attributes-only", false, "copy only the attributes of the source files to the destination")
	flag.StringVar(&backup, "backup", "", "make a backup of each existing destination file")
	flag.BoolVar(&copyContents, "copy-contents", false, "copy the contents of special files when recursive")
	flag.BoolVar(&d, "d", false, "same as --no-dereference --preserve=link")
	flag.BoolVar(&force, "force", false, "if an existing destination file cannot be opened, remove it and try again")
	flag.BoolVar(&interactive, "interactive", false, "prompt before overwrite")
	flag.BoolVar(&h, "H", false, "follow command-line symbolic links in SOURCE")
	flag.BoolVar(&link, "link", false, "make hard links instead of copying")
	flag.BoolVar(&dereference, "dereference", false, "dereference symbolic links in SOURCE")
	flag.BoolVar(&noClobber, "no-clobber", false, "do not overwrite an existing file")
	flag.BoolVar(&noDereference, "no-dereference", false, "never follow symbolic links in SOURCE")
	flag.StringVar(&preserve, "preserve", "", "preserve the specified attributes (default: mode,ownership,timestamps)")
	flag.StringVar(&context, "context", "", "set SELinux security context of copy to CTX")
}

// Function to implement the 'cp' command
func cpCommand(arguments []string) {
	flag.Parse()

	if help {
		cpPrintUsage()
		return
	}
	
	else if version {
		cpPrintVersion()
		return
	}

	// Check if two operands are provided
	if len(flag.Args()) != 2 {
		fmt.Println("cp: missing file operand")
		fmt.Println("Try 'cp --help' for more information.")
		return
	}

	sourceFile := flag.Arg(0)
	destinationFile := flag.Arg(1)

	if archive {
		err := copyDir(sourceFile, destinationFile)
		if err != nil {
			fmt.Printf("cp: %v\n", err)
		}
	}
	
	else {
		err := copyFile(sourceFile, destinationFile)
		if err != nil {
			fmt.Printf("cp: %v\n", err)
		}
	}
}

// Function to copy a file
func copyFile(sourceFile, destinationFile string) error {
	source, err := os.Open(sourceFile)
	if err != nil {
		return err
	}
	defer source.Close()

	destination, err := os.Create(destinationFile)
	if err != nil {
		return err
	}
	defer destination.Close()

	_, err = io.Copy(destination, source)
	if err != nil {
		return err
	}

	return nil
}

// Function to copy a directory
func copyDir(sourceDir, destinationDir string) error {
	err := filepath.Walk(sourceDir, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}

		// Construct the destination path by joining the destination directory and relative path
		destinationPath := filepath.Join(destinationDir, path[len(sourceDir):])

		if info.IsDir() {
			err := os.MkdirAll(destinationPath, info.Mode())
			if err != nil {
				return err
			}
		}
		
		else {
			err := copyFile(path, destinationPath)
			if err != nil {
				return err
			}
		}

		return nil
	})
	return err
}

// Function to print usage instructions
func cpPrintUsage() {
	fmt.Println("Usage: cp [OPTION]... SOURCE DEST")
	fmt.Println("Copy SOURCE to DEST.")
	fmt.Println("\nOptions:")
	flag.PrintDefaults()
}

// Function to print version information
func cpPrintVersion() {
	fmt.Println("cp (cacutils) v1.0")
	fmt.Println("There is NO WARRANTY, to the extent permitted by law.")
	fmt.Println("Written by Cyril John Magayaga.")
}
