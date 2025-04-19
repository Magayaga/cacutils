import Foundation

func mkdir_command(arguments: [String]) {
    if arguments.contains("--help") {
        mkdir_print_usage()
        return
    }

    if arguments.contains("--version") {
        mkdir_print_version()
        return
    }

    guard !arguments.isEmpty else {
        fputs("Usage: mkdir [OPTION]... DIRECTORY...\n", stderr)
        return
    }

    var paths = [String]()
    var createParents = false
    var verbose = false

    for arg in arguments {
        switch arg {
        case "-p", "--parents":
            createParents = true
        case "-v", "--verbose":
            verbose = true
        default:
            paths.append(arg)
        }
    }

    for path in paths {
        let url = URL(fileURLWithPath: path)
        let fileManager = FileManager.default

        do {
            if createParents {
                try fileManager.createDirectory(at: url, withIntermediateDirectories: true)
            } else {
                try fileManager.createDirectory(at: url, withIntermediateDirectories: false)
            }

            if verbose {
                print("created directory: \(path)")
            }
        } catch {
            fputs("mkdir: cannot create directory '\(path)': \(error.localizedDescription)\n", stderr)
        }
    }
}

func mkdir_print_usage() {
    print("""
    Usage: mkdir [OPTION]... DIRECTORY...
    Create the DIRECTORY(ies), if they do not already exist.

    Options:
      -p, --parents   no error if existing, make parent directories as needed
      -v, --verbose   print a message for each created directory
      --help          display this help and exit
      --version       output version information and exit
    """)
}

func mkdir_print_version() {
    print("""
    mkdir (mkdirutils) v1.0
    There is NO WARRANTY, to the extent permitted by law.
    Written by Cyril John Magayaga.
    """)
}
