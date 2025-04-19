import Foundation

func time_command(arguments: [String]) {
    if arguments.isEmpty {
        fputs("Usage: time [COMMAND] [ARGS]...\n", stderr)
        return
    }

    let commandName = arguments[0]
    let args = Array(arguments.dropFirst())

    let startTime = Date()

    let process = Process()
    process.executableURL = URL(fileURLWithPath: "/usr/bin/env") // Use env to find the command
    process.arguments = [commandName] + args
    process.standardOutput = FileHandle.standardOutput
    process.standardError = FileHandle.standardError

    do {
        try process.run()
        process.waitUntilExit()

        let duration = Date().timeIntervalSince(startTime)

        print("\nreal: \(String(format: "%.2f", duration))s")
        print("user: n/a")
        print("sys: n/a")

        if process.terminationStatus != 0 {
            fputs("Command \(commandName) failed with status \(process.terminationStatus)\n", stderr)
        }
    } catch {
        fputs("Failed to execute command \(commandName): \(error)\n", stderr)
    }
}
