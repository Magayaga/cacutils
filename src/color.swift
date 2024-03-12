import Foundation

// ANSI color codes
struct ANSIColors {
    static let green = "\u{001B}[32m"
    static let blue = "\u{001B}[34m"
    static let yellow = "\u{001B}[33m"
    static let magenta = "\u{001B}[35m"
    static let cyan = "\u{001B}[36m"
    static let reset = "\u{001B}[0m"
}

// Function to apply color to text
func colorize(_ text: String, color: String) -> String {
    return color + text + ANSIColors.reset
}
