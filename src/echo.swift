import Foundation

// Function to implement the 'echo' command
func echo_command(arguments: [String]) {
    // Check if --help or --version option is provided
    if arguments.contains("--help") {
        echo_print_usage()
        return
    }

    else if arguments.contains("--version") {
        echo_print_version()
        return
    }

    var noNewline = false
    var interpretEscapes = false
    var tokens = [String]()

    for arg in arguments {
        switch arg {
        case "-n":
            noNewline = true
        case "-e":
            interpretEscapes = true
        case "-E":
            interpretEscapes = false
        default:
            tokens.append(arg)
        }
    }

    var output = tokens.joined(separator: " ")

    if interpretEscapes {
        output = output
            .replacingOccurrences(of: "\\n", with: "\n")
            .replacingOccurrences(of: "\\t", with: "\t")
            .replacingOccurrences(of: "\\r", with: "\r")
            .replacingOccurrences(of: "\\\\", with: "\\")
            .replacingOccurrences(of: "\\a", with: "\u{0007}")
            .replacingOccurrences(of: "\\b", with: "\u{0008}")
            .replacingOccurrences(of: "\\v", with: "\u{000B}")
    }

    if noNewline {
        print(output, terminator: "")
    } else {
        print(output)
    }
}

// Function to print usage instructions
func echo_print_usage() {
    print("""
    Usage: echo [OPTION]... [STRING]...
    Echo the STRING(s) to standard output.

    Options:
      -n          do not output the trailing newline
      -e          enable interpretation of backslash escapes
      -E          disable interpretation of backslash escapes (default)
      --help      display this help and exit
      --version   output version information and exit

    Escape sequences (with -e):
      \\\\   backslash
      \\a   alert (BEL)
      \\b   backspace
      \\n   new line
      \\r   carriage return
      \\t   horizontal tab
      \\v   vertical tab
    """)
}

// Function to print version information
func echo_print_version() {
    print("""
    echo (cacutils) v1.0
    There is NO WARRANTY, to the extent permitted by law.
    Written by Cyril John Magayaga.
    """)
}
