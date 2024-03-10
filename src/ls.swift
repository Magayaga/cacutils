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
    
    // Parse options
    var showHiddenFiles = false
    var showDetails = false
    var showHumanReadableSize = false
    
    for option in arguments {
        switch option {
        case "-a":
            showHiddenFiles = true
        case "-l":
            showDetails = true
        case "-h":
            showHumanReadableSize = true
        default:
            break
        }
    }
    
    // Get the directory path (if provided) or use the current directory
    let directoryPath = arguments.last ?? FileManager.default.currentDirectoryPath
    
    do {
        let contents = try FileManager.default.contentsOfDirectory(atPath: directoryPath)
        
        for item in contents {
            // Skip hidden files if -a option is not provided
            if !showHiddenFiles && item.hasPrefix(".") {
                continue
            }
            
            // Get detailed information if -l option is provided
            if showDetails {
                let itemPath = (directoryPath as NSString).appendingPathComponent(item)
                let attributes = try FileManager.default.attributesOfItem(atPath: itemPath)
                
                let fileSize = attributes[.size] as? Int ?? 0
                let fileModificationDate = attributes[.modificationDate] as? Date ?? Date()
                
                var details = "\(item)"
                if showHumanReadableSize {
                    details += " - \(fileSizeToHumanReadable(fileSize))"
                } else {
                    details += " - \(fileSize) bytes"
                }
                details += " - \(fileModificationDate)"
                
                print(details)
            } else {
                print(item)
            }
        }
    } catch {
        print("Error listing directory contents: \(error)")
    }
}

// Function to convert file size to human-readable format
func fileSizeToHumanReadable(_ size: Int) -> String {
    let units = ["B", "KB", "MB", "GB", "TB"]
    var fileSize = Double(size)
    var unitIndex = 0
    
    while fileSize > 1024 {
        fileSize /= 1024
        unitIndex += 1
    }
    
    return String(format: "%.1f %@", fileSize, units[unitIndex])
}

// Function to print usage instructions
func printUsage() {
    print("Usage: ls [OPTION]... [FILE]...")
    print("List information about the FILEs (the current directory by default).")
    print("\nOptions:")
    print("  -a                do not ignore entries starting with .")
    print("  -l                use a long listing format")
    print("  -h                with -l, print sizes in human readable format (e.g., 1K 234M 2G)")
    print("  --help            display this help and exit")
    print("  --version         output version information and exit")
}

// Function to print version information
func printVersion() {
    print("ls (cacutils) v1.0")
    print("There is NO WARRANTY, to the extent permitted by law.")
    print("\nWritten by Cyril John Magayaga.")
}
