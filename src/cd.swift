import Foundation

// Function to implement the 'cd' command
func cd_command(arguments: [String]) {
    // Check if --help or --version option is provided
    if arguments.contains("--help") {
        cd_print_usage()
        return
    }
    
    else if arguments.contains("--version") {
        cd_print_version()
        return
    }

    // Get the directory path from arguments
    guard let directoryPath = arguments.first else {
        print("cd: missing directory")
        return
    }

    // Validate and resolve the directory path
    guard var resolvedPath = resolveDirectoryPath(directoryPath: directoryPath, arguments: arguments) else {
        return
    }

    // Change directory
    do {
        try FileManager.default.changeCurrentDirectoryPath(resolvedPath)
    }
    
    catch {
        print("cd: \(error.localizedDescription)")
    }
}

// Function to resolve the directory path based on options
func resolveDirectoryPath(directoryPath: String, arguments: [String]) -> String? {
    var resolvedPath = directoryPath

    // Apply options
    if arguments.contains("-L") {
        // Follow symbolic links
        do {
            resolvedPath = try FileManager.default.destinationOfSymbolicLink(atPath: directoryPath)
        }
        
        catch {
            print("cd: \(directoryPath): No such file or directory")
            return nil
        }
    }
    
    else if arguments.contains("-P") {
        // Use physical directory structure
        resolvedPath = URL(fileURLWithPath: directoryPath).standardizedFileURL.path
    }
    
    else if arguments.contains("~") {
        resolvedPath = NSHomeDirectory()
    }

    // Check if the path exists
    if arguments.contains("-e") && !FileManager.default.fileExists(atPath: resolvedPath) {
        print("cd: \(directoryPath): No such file or directory")
        return nil
    }

    return resolvedPath
}

// Function to print usage instructions
func cd_print_usage() {
    print("Usage: cd [OPTION]... DIRECTORY")
    print("Change the shell working directory to DIRECTORY.")
    print("\nOptions:")
    print("  -L             force symbolic links to be followed")
    print("  -P             use the physical directory structure")
    print("  -e             check if the directory exists before changing")
    print("  -@             print symbolic links resolved name")
    print("  --help         display this help and exit")
    print("  --version      output version information and exit")
}

// Function to print version information
func cd_print_version() {
    print("cat (cacutils) v1.0")
    print("There is NO WARRANTY, to the extent permitted by law.")
    print("Written by Cyril John Magayaga.")
}
