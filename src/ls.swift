import Foundation

// Function to implement the 'ls' command
func lsCommand(arguments: [String]) {
    // Check if --help or --version option is provided
    if arguments.contains("--help") {
        printUsage()
        return
    } else if arguments.contains("--version") {
        printVersion()
        return
    }
    
    // Get the directory path (if provided) or use the current directory
    let directoryPath = arguments.first ?? FileManager.default.currentDirectoryPath
    
    do {
        let contents = try FileManager.default.contentsOfDirectory(atPath: directoryPath)
        for item in contents {
            print(item)
        }
    } catch {
        print("Error listing directory contents: \(error)")
    }
}

// Function to print usage instructions
func printUsage() {
    print("Usage: ls [OPTION]... [FILE]...")
    print("List information about the FILEs (the current directory by default).")
    print("\nOptions:")
    print("  --help            display this help and exit")
    print("  --version         output version information and exit")
}

// Function to print version information
func printVersion() {
    print("ls (cacutils) v1.0")
    print("There is NO WARRANTY, to the extent permitted by law.")
    print("\nWritten by Cyril John Magayaga.")
}
