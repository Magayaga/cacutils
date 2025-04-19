import Foundation

struct Options {
    var recursive: Bool
    var verbose: Bool
}

func cp_command(arguments: [String]) {
    if arguments.contains("--help") {
        cp_print_usage()
        return
    }

    if arguments.contains("--version") {
        cp_print_version()
        return
    }

    if arguments.count < 2 {
        fputs("Usage: cp [OPTION]... SOURCE... DEST\n", stderr)
        return
    }

    let (options, sources, destination) = parse_arguments(arguments: arguments)

    var isDirectory: ObjCBool = false
    if sources.count > 1 && !(FileManager.default.fileExists(atPath: destination, isDirectory: &isDirectory) && isDirectory.boolValue) {
        fputs("When copying multiple files, the destination must be a directory.\n", stderr)
        return
    }

    for source in sources {
        if options.recursive {
            do {
                try copy_recursive(source: source, destination: destination, verbose: options.verbose)
            } catch {
                fputs("Error copying directory \(source): \(error.localizedDescription)\n", stderr)
            }
        } else {
            do {
                try copy_file(source: source, destination: destination, verbose: options.verbose)
            } catch {
                fputs("Error copying file \(source): \(error.localizedDescription)\n", stderr)
            }
        }
    }
}

func parse_arguments(arguments: [String]) -> (Options, [String], String) {
    var options = Options(recursive: false, verbose: false)
    var sources: [String] = []
    var destination = ""

    var args_iter = arguments.makeIterator()
    while let arg = args_iter.next() {
        switch arg {
        case "-r", "--recursive":
            options.recursive = true
        case "-v", "--verbose":
            options.verbose = true
        default:
            if destination.isEmpty {
                sources.append(arg)
            } else {
                destination = arg
            }
        }
    }

    if !sources.isEmpty && destination.isEmpty {
        destination = sources.removeLast()
    }

    return (options, sources, destination)
}

func copy_file(source: String, destination: String, verbose: Bool) throws {
    let srcURL = URL(fileURLWithPath: source)
    var destURL = URL(fileURLWithPath: destination)

    var isDir: ObjCBool = false
    if FileManager.default.fileExists(atPath: destination, isDirectory: &isDir), isDir.boolValue {
        destURL.appendPathComponent(srcURL.lastPathComponent)
    }

    try FileManager.default.copyItem(at: srcURL, to: destURL)

    if verbose {
        print("\(source) -> \(destURL.path)")
    }
}

func copy_recursive(source: String, destination: String, verbose: Bool) throws {
    let srcURL = URL(fileURLWithPath: source)
    var destURL = URL(fileURLWithPath: destination)

    var isDir: ObjCBool = false
    if !FileManager.default.fileExists(atPath: source, isDirectory: &isDir) || !isDir.boolValue {
        try copy_file(source: source, destination: destination, verbose: verbose)
        return
    }

    if FileManager.default.fileExists(atPath: destURL.path, isDirectory: &isDir), isDir.boolValue {
        destURL.appendPathComponent(srcURL.lastPathComponent)
    }

    try FileManager.default.createDirectory(at: destURL, withIntermediateDirectories: true, attributes: nil)

    let contents = try FileManager.default.contentsOfDirectory(atPath: srcURL.path)

    for item in contents {
        let srcItem = srcURL.appendingPathComponent(item).path
        let destItem = destURL.appendingPathComponent(item).path

        var isSubDir: ObjCBool = false
        if FileManager.default.fileExists(atPath: srcItem, isDirectory: &isSubDir), isSubDir.boolValue {
            try copy_recursive(source: srcItem, destination: destItem, verbose: verbose)
        } else {
            try FileManager.default.copyItem(atPath: srcItem, toPath: destItem)
            if verbose {
                print("\(srcItem) -> \(destItem)")
            }
        }
    }
}

func cp_print_usage() {
    print("Usage: cp [OPTION]... SOURCE... DEST")
    print("Copy SOURCE to DEST, or multiple SOURCE(s) to DIRECTORY.\n")
    print("Options:")
    print("  --help        display this help and exit")
    print("  --version     output version information and exit")
    print("  -r, --recursive  copy directories recursively")
    print("  -v, --verbose  explain what is being done")
}

func cp_print_version() {
    print("cp (cputils) v1.0")
    print("There is NO WARRANTY, to the extent permitted by law.")
    print("Written by Cyril John Magayaga.")
}
