import Foundation

// Function to implement the 'cat' command
func cat_command(arguments: [String]) {
    // Check if --help or --version option is provided
    if arguments.contains("--help") {
        cat_print_usage()
        return
    } else if arguments.contains("--version") {
        cat_print_version()
        return
    }
    
    // Check if any file is provided
    guard let fileName = arguments.first else {
        print("Usage: cat [OPTION]... [FILE]...")
        return
    }
    
    // Read the content of the file
    do {
        let content = try String(contentsOfFile: fileName)
        
        // Apply options
        var formattedContent = content
        if arguments.contains("-A") || arguments.contains("--show-all") {
            formattedContent = content.replacingOccurrences(of: "\n", with: "$\n").replacingOccurrences(of: "\t", with: "^I")
        }
        if arguments.contains("-b") || arguments.contains("--number-nonblank") {
            var lineCount = 1
            formattedContent = content.split(separator: "\n").map { line -> String in
                if !line.isEmpty {
                    let numberedLine = "\(lineCount)\t\(line)"
                    lineCount += 1
                    return numberedLine
                } else {
                    return String(line)
                }
            }.joined(separator: "\n")
        }
        if arguments.contains("-e") {
            formattedContent = content.replacingOccurrences(of: "\n", with: "$\n")
        }
        if arguments.contains("-E") || arguments.contains("--show-ends") {
            formattedContent = content.replacingOccurrences(of: "\n", with: "$\n") + "$"
        }
        if arguments.contains("-n") || arguments.contains("--number") {
            var lineCount = 1
            formattedContent = content.split(separator: "\n").map { line -> String in
                let numberedLine = "\(lineCount)\t\(line)"
                lineCount += 1
                return numberedLine
            }.joined(separator: "\n")
        }
        if arguments.contains("-s") || arguments.contains("--squeeze-blank") {
            formattedContent = content.replacingOccurrences(of: "\n\n+", with: "\n", options: .regularExpression)
        }
        if arguments.contains("-t") {
            formattedContent = content.replacingOccurrences(of: "\t", with: "^I")
        }
        if arguments.contains("-T") || arguments.contains("--show-tabs") {
            formattedContent = content.replacingOccurrences(of: "\t", with: "^I")
        }
        
        print(formattedContent)
    } catch {
        print("Error reading file: \(fileName)")
    }
}

// Function to print usage instructions
func cat_print_usage() {
    print("Usage: cat [OPTION]... [FILE]...")
    print("Concatenate FILE(s) to standard output.")
    print("\nOptions:")
    print("  --help            display this help and exit")
    print("  -A, --show-all    equivalent to -vET")
    print("  -b, --number-nonblank")
    print("                    number nonempty output lines, overrides -n")
    print("  -e                equivalent to -vE")
    print("  -E, --show-ends   display $ at end of each line")
    print("  -n, --number      number all output lines")
    print("  -s, --squeeze-blank")
    print("                    suppress repeated empty output lines")
    print("  -t                equivalent to -vT")
    print("  -T, --show-tabs   display TAB characters as ^I")
    print("  --version         output version information and exit")
}

// Function to print version information
func cat_print_version() {
    print("cat (cacutils) v1.0")
    print("There is NO WARRANTY, to the extent permitted by law.")
    print("Written by Cyril John Magayaga.")
}
