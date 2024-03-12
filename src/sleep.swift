import Foundation

// Function to implement the 'sleep' command
func sleep_command(arguments: [String]) {
    // Check if --help or --version option is provided
    if arguments.contains("--help") {
        sleep_print_usage()
        return
    } else if arguments.contains("--version") {
        sleep_print_version()
        return
    }
    
    // Check if any operand is provided
    guard arguments.count > 0 else {
        print("sleep: missing operand")
        print("Try 'sleep --help' for more information.")
        return
    }

    // Get the duration from arguments
    let duration: TimeInterval = parseDuration(from: arguments)
    
    // Sleep for the specified duration
    Thread.sleep(forTimeInterval: duration)
}

// Function to parse the duration from arguments
func parseDuration(from arguments: [String]) -> TimeInterval {
    guard let durationString = arguments.first else {
        return 1.0 // Default duration of 1 second
    }

    let unitMap: [Character: TimeInterval] = ["s": 1.0, "m": 60.0, "h": 3600.0, "d": 86400.0]
    
    var duration = 0.0
    
    // Parse the duration string
    var numberString = ""
    var unitChar: Character?
    for char in durationString {
        if let _ = unitMap[char] {
            unitChar = char
            break
        } else {
            numberString.append(char)
        }
    }
    
    if let number = Double(numberString), let unit = unitChar {
        duration = number * unitMap[unit]!
    } else {
        if let number = Double(durationString) {
            duration = number
        } else {
            print("Invalid duration format.")
        }
    }

    return duration
}

// Function to print usage instructions
func sleep_print_usage() {
    print("Usage: sleep [NUMBER][s|m|h|d]")
    print("Pause execution for NUMBER seconds, minutes, hours, or days.")
    print("\nOptions:")
    print("  --help     display this help and exit")
    print("  --version  output version information and exit")
}

// Function to print version information
func sleep_print_version() {
    print("cd (cacutils) v1.0")
    print("There is NO WARRANTY, to the extent permitted by law.")
    print("Written by Cyril John Magayaga.")
}
