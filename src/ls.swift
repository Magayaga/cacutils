import Foundation

// Function to implement the 'ls' command
func ls_command(arguments: [String]) {
    // Check if --help or --version option is provided
    if arguments.contains("--help") {
        ls_print_usage()
        return
    } else if arguments.contains("--version") {
        ls_print_version()
        return
    }
    
    // Get the directory path (if provided) or use the current directory
    let directoryPath = arguments.first ?? FileManager.default.currentDirectoryPath
    
    // Set default block size
    var blockSize: Int = 1024
    
    // Determine block size if provided in arguments
    if let blockSizeArg = arguments.first(where: { $0.hasPrefix("--block-size=") }) {
        if let sizeString = blockSizeArg.split(separator: "=").last, let size = Int(sizeString) {
            blockSize = size
        }
    }
    
    do {
        let contents = try FileManager.default.contentsOfDirectory(atPath: directoryPath)
        
        // Apply options
        if arguments.contains("-l") || arguments.contains("-la") {
            for item in contents {
                let itemPath = "\(directoryPath)/\(item)"
                let attributes = try FileManager.default.attributesOfItem(atPath: itemPath)
                if let fileSize = attributes[.size] as? NSNumber,
                   let creationDate = attributes[.creationDate] as? Date {
                    
                    let formattedDate = DateFormatter.localizedString(from: creationDate, dateStyle: .medium, timeStyle: .medium)
                    let formattedSize = formatSize(fileSize, blockSize: blockSize)
                    
                    var formattedDetails = "\(formattedSize) \(formattedDate) \(item)"
                    
                    // Include author if --author option is provided
                    if arguments.contains("--author") {
                        if let fileOwner = attributes[.ownerAccountName] as? String {
                            formattedDetails += " \(fileOwner)"
                        }
                    }
                    
                    print(formattedDetails)
                }
            }
        } else if arguments.contains("-a") || arguments.contains("--all") {
            for item in contents {
                print(item)
            }
        } else if arguments.contains("-d") || arguments.contains("--directory") {
            // List directory entries instead of contents
            print(directoryPath)
        } else {
            for item in contents where !item.hasPrefix(".") {
                print(item)
            }
        }
    } catch {
        print("Error listing directory contents: \(error)")
    }
}

// Function to format file size with block size
func formatSize(_ fileSize: NSNumber, blockSize: Int) -> String {
    let size = fileSize.intValue
    let formattedSize: String
    if size < blockSize {
        formattedSize = "\(size)B"
    } else if size < blockSize * blockSize {
        formattedSize = "\(size / blockSize)KB"
    } else {
        formattedSize = "\(size / (blockSize * blockSize))MB"
    }
    return formattedSize
}

// Function to print usage instructions
func ls_print_usage() {
    print("Usage: ls [OPTION]... [FILE]...")
    print("List information about the FILEs (the current directory by default).")
    print("\nOptions:")
    print("  -a, --all         do not ignore entries starting with .")
    print("  -l                use a long listing format")
    print("  -la               list all files in long format")
    print("      --author      with -l, print the author of each file")
    print("  -b, --escape      with -b, print octal escapes for nongraphic characters")
    print("      --block-size=SIZE  with -l, scale sizes by SIZE when printing them")
    print("  -d, --directory   list directory entries instead of contents")
    print("      --color       colorize the output")
    print("  --help            display this help and exit")
    print("  --version         output version information and exit")
}

// Function to print version information
func ls_print_version() {
    print("cat (cacutils) v1.0")
    print("There is NO WARRANTY, to the extent permitted by law.")
    print("Written by Cyril John Magayaga.")
}
