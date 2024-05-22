package main

import (
    "fmt"
    "os"
    "path/filepath"
    "strconv"
)

// Function to implement the 'ls' command
func lsCommand(arguments []string) {
    // Check if --help or --version option is provided
    for _, arg := range arguments {
        switch arg {
        case "--help":
            lsPrintUsage()
            return
        case "--version":
            lsPrintVersion()
            return
        }
    }

    // Get the directory path (if provided) or use the current directory
    directoryPath := "."
    if len(arguments) > 1 {
        directoryPath = arguments[1]
    }

    // Set default block size
    blockSize := 1024

    // Determine block size if provided in arguments
    for _, arg := range arguments {
        if len(arg) > len("--block-size=") && arg[:len("--block-size=")] == "--block-size=" {
            sizeStr := arg[len("--block-size="):]
            size, err := strconv.Atoi(sizeStr)
            if err == nil {
                blockSize = size
            }
        }
    }

    // Walk through the directory
    err := filepath.Walk(directoryPath, func(path string, info os.FileInfo, err error) error {
        if err != nil {
            return err
        }

        // Get file name
        fileName := info.Name()

        // Get file creation time
        created := info.ModTime()
        formattedDate := created.Format("Jan 02 15:04")

        // Format file size
        formattedSize := formatSize(int(info.Size()), blockSize)

        // Print file details
        fmt.Printf("%s %s %s\n", formattedSize, formattedDate, fileName)

        return nil
    })

    if err != nil {
        fmt.Fprintf(os.Stderr, "Error listing directory contents: %v\n", err)
    }
}

// Function to format file size with block size
func formatSize(fileSize, blockSize int) string {
    if fileSize < blockSize {
        return fmt.Sprintf("%dB", fileSize)
    } else if fileSize < blockSize*blockSize {
        return fmt.Sprintf("%dKB", fileSize/blockSize)
    }
    return fmt.Sprintf("%dMB", fileSize/(blockSize*blockSize))
}

// Function to print usage instructions
func lsPrintUsage() {
    fmt.Println("Usage: ls [OPTION]... [FILE]...")
    fmt.Println("List information about the FILEs (the current directory by default).")
    fmt.Println("\nOptions:")
    fmt.Println("  --help            display this help and exit")
    fmt.Println("  --version         output version information and exit")
}

// Function to print version information
func lsPrintVersion() {
    fmt.Println("ls (cacutils) v1.0")
    fmt.Println("There is NO WARRANTY, to the extent permitted by law.")
    fmt.Println("Written by Cyril John Magayaga.")
}
