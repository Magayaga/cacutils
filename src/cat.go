package main

import (
    "fmt"
    "io/ioutil"
    "strings"
)

// Function to implement the 'cat' command
func catCommand(arguments []string) {
    // Check if --help or --version option is provided
    for _, arg := range arguments {
        if arg == "--help" {
            catPrintUsage()
            return
        } else if arg == "--version" {
            catPrintVersion()
            return
        }
    }

    // Check if any file is provided
    if len(arguments) == 0 {
        fmt.Println("Usage: cat [OPTION]... [FILE]...")
        return
    }

    // Read the content of the file
    fileName := arguments[0]
    content, err := ioutil.ReadFile(fileName)
    if err != nil {
        fmt.Printf("Error reading file: %s\n", fileName)
        return
    }

    // Apply options
    formattedContent := applyOptions(string(content), arguments)
    fmt.Println(formattedContent)
}

// Function to apply options to the content
func applyOptions(content string, arguments []string) string {
    options := strings.Join(arguments, " ")

    if strings.Contains(options, "-A") || strings.Contains(options, "--show-all") {
        content = strings.ReplaceAll(content, "\n", "$\n")
        content = strings.ReplaceAll(content, "\t", "^I")
    }

    if strings.Contains(options, "-b") || strings.Contains(options, "--number-nonblank") {
        lines := strings.Split(content, "\n")
        lineCount := 1
        for i, line := range lines {
            if line != "" {
                lines[i] = fmt.Sprintf("%d\t%s", lineCount, line)
                lineCount++
            }
        }
        content = strings.Join(lines, "\n")
    }

    if strings.Contains(options, "-e") {
        content = strings.ReplaceAll(content, "\n", "$\n")
    }

    if strings.Contains(options, "-E") || strings.Contains(options, "--show-ends") {
        content = strings.ReplaceAll(content, "\n", "$\n") + "$"
    }

    if strings.Contains(options, "-n") || strings.Contains(options, "--number") {
        lines := strings.Split(content, "\n")
        for i, line := range lines {
            lines[i] = fmt.Sprintf("%d\t%s", i+1, line)
        }
        content = strings.Join(lines, "\n")
    }

    if strings.Contains(options, "-s") || strings.Contains(options, "--squeeze-blank") {
        content = strings.ReplaceAll(content, "\n\n+", "\n")
    }

    if strings.Contains(options, "-t") {
        content = strings.ReplaceAll(content, "\t", "^I")
    }

    if strings.Contains(options, "-T") || strings.Contains(options, "--show-tabs") {
        content = strings.ReplaceAll(content, "\t", "^I")
    }

    return content
}

// Function to print usage instructions
func catPrintUsage() {
    fmt.Println("Usage: cat [OPTION]... [FILE]...")
    fmt.Println("Concatenate FILE(s) to standard output.")
    fmt.Println("\nOptions:")
    fmt.Println("  --help            display this help and exit")
    fmt.Println("  -A, --show-all    equivalent to -vET")
    fmt.Println("  -b, --number-nonblank")
    fmt.Println("                    number nonempty output lines, overrides -n")
    fmt.Println("  -e                equivalent to -vE")
    fmt.Println("  -E, --show-ends   display $ at end of each line")
    fmt.Println("  -n, --number      number all output lines")
    fmt.Println("  -s, --squeeze-blank")
    fmt.Println("                    suppress repeated empty output lines")
    fmt.Println("  -t                equivalent to -vT")
    fmt.Println("  -T, --show-tabs   display TAB characters as ^I")
    fmt.Println("  --version         output version information and exit")
}

// Function to print version information
func catPrintVersion() {
    fmt.Println("cat (cacutils) v1.0")
    fmt.Println("There is NO WARRANTY, to the extent permitted by law.")
    fmt.Println("Written by Cyril John Magayaga.")
}
