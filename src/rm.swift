import Foundation

func rm_command(arguments: [String]) {
    if arguments.contains("--help") {
        rm_print_usage()
        return
    }

    if arguments.contains("--version") {
        rm_print_version()
        return
    }

    guard !arguments.isEmpty else {
        fputs("Usage: rm [OPTION]... FILE...\n", stderr)
        return
    }

    var paths = [String]()
    var recursive = false
    var force = false
    var verbose = false

    for arg in arguments {
        switch arg {
        case "-r", "-R", "--recursive":
            recursive = true
        case "-f", "--force":
            force = true
        case "-v", "--verbose":
            verbose = true
        default:
            paths.append(arg)
        }
    }

    let fileManager = FileManager.default

    for path in paths {
        let url = URL(fileURLWithPath: path)

        do {
            var isDir: ObjCBool = false
            if fileManager.fileExists(atPath: url.path, isDirectory: &isDir) {
                if isDir.boolValue {
                    if recursive {
                        try fileManager.removeItem(at: url)
                        if verbose {
                            print("removed directory: \(path)")
                        }
                    } else {
                        fputs("rm: cannot remove '\(path)': Is a directory\n", stderr)
                        if !force { continue }
                    }
                } else {
                    try fileManager.removeItem(at: url)
                    if verbose {
                        print("removed file: \(path)")
                    }
                }
            } else {
                if !force {
                    fputs("rm: cannot remove '\(path)': No such file or directory\n", stderr)
                }
            }
        } catch {
            if !force {
                fputs("rm: cannot remove '\(path)': \(error.localizedDescription)\n", stderr)
            }
        }
    }
}

func rm_print_usage() {
    print("""
    Usage: rm [OPTION]... FILE...
    Remove (unlink) the FILE(s).

    Options:
      -f, --force     ignore nonexistent files and arguments, never prompt
      -r, -R, --recursive   remove directories and their contents recursively
      -v, --verbose   explain what is being done
      --help          display this help and exit
      --version       output version information and exit
    """)
}

func rm_print_version() {
    print("""
    rm (rmutills) v1.0
    There is NO WARRANTY, to the extent permitted by law.
    Written by Cyril John Magayaga.
    """)
}
