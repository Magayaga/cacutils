import Foundation

// MARK: Entry point

func awk_command(arguments: [String]) {
    if arguments.contains("--help") {
        awk_print_usage()
        return
    }

    if arguments.contains("--version") {
        awk_print_version()
        return
    }

    guard !arguments.isEmpty else {
        fputs("awk: no program given\nTry 'awk --help' for more information.\n", stderr)
        return
    }

    var programSource: String? = nil
    var programFile: String? = nil
    var fieldSeparator: String = " "
    var assignments: [(String, String)] = []
    var inputFiles: [String] = []
    var i = 0

    while i < arguments.count {
        let arg = arguments[i]
        switch arg {
        case "-f":
            i += 1
            guard i < arguments.count else {
                fputs("awk: -f requires a file argument\n", stderr)
                return
            }
            programFile = arguments[i]
        case "-F":
            i += 1
            guard i < arguments.count else {
                fputs("awk: -F requires a separator argument\n", stderr)
                return
            }
            fieldSeparator = arguments[i]
        case "-v":
            i += 1
            guard i < arguments.count else {
                fputs("awk: -v requires a var=value argument\n", stderr)
                return
            }
            let parts = arguments[i].split(separator: "=", maxSplits: 1).map(String.init)
            if parts.count == 2 {
                assignments.append((parts[0], parts[1]))
            } else {
                fputs("awk: invalid -v assignment: \(arguments[i])\n", stderr)
                return
            }
        default:
            if arg.hasPrefix("-F") {
                fieldSeparator = String(arg.dropFirst(2))
            } else if arg.contains("=") && !arg.hasPrefix("-") && programSource != nil {
                let parts = arg.split(separator: "=", maxSplits: 1).map(String.init)
                assignments.append((parts[0], parts[1]))
            } else if programSource == nil && programFile == nil {
                programSource = arg
            } else {
                inputFiles.append(arg)
            }
        }
        i += 1
    }

    // Load program source
    let source: String
    if let file = programFile {
        do {
            source = try String(contentsOfFile: file, encoding: .utf8)
        } catch {
            fputs("awk: cannot open program file '\(file)': \(error.localizedDescription)\n", stderr)
            return
        }
    } else if let prog = programSource {
        source = prog
    } else {
        fputs("awk: no program given\n", stderr)
        return
    }

    // Parse and run
    let interpreter = AWKInterpreter(fieldSeparator: fieldSeparator)
    for (key, value) in assignments {
        interpreter.setVariable(key, value: .string(value))
    }

    do {
        let program = try AWKParser(source: source).parse()
        if inputFiles.isEmpty {
            // Read from stdin
            interpreter.run(program: program, input: AnyIterator {
                readLine(strippingNewline: true)
            })
        } else {
            for filePath in inputFiles {
                guard let handle = FileHandle(forReadingAtPath: filePath) else {
                    fputs("awk: cannot open '\(filePath)': No such file or directory\n", stderr)
                    continue
                }
                let content = String(data: handle.readDataToEndOfFile(), encoding: .utf8) ?? ""
                handle.closeFile()
                let lines = content.components(separatedBy: "\n")
                var lineIndex = 0
                interpreter.run(program: program, input: AnyIterator {
                    guard lineIndex < lines.count else { return nil }
                    let line = lines[lineIndex]
                    lineIndex += 1
                    // Skip phantom trailing empty line from split
                    if lineIndex == lines.count && line.isEmpty { return nil }
                    return line
                })
            }
        }
    } catch let error as AWKError {
        fputs("awk: \(error.message)\n", stderr)
    } catch {
        fputs("awk: \(error.localizedDescription)\n", stderr)
    }
}

// MARK: Error

struct AWKError: Error {
    let message: String
}

// MARK: Value

indirect enum AWKValue {
    case string(String)
    case number(Double)
    case uninitialized

    var stringValue: String {
        switch self {
        case .string(let s): return s
        case .number(let n):
            if n == n.rounded() && !n.isInfinite && abs(n) < 1e15 {
                return String(format: "%.6g", n)
            }
            return String(format: "%.6g", n)
        case .uninitialized: return ""
        }
    }

    var numberValue: Double {
        switch self {
        case .string(let s): return Double(s.trimmingCharacters(in: .whitespaces)) ?? 0
        case .number(let n): return n
        case .uninitialized: return 0
        }
    }

    var boolValue: Bool {
        switch self {
        case .string(let s): return !s.isEmpty
        case .number(let n): return n != 0
        case .uninitialized: return false
        }
    }

    static func add(_ a: AWKValue, _ b: AWKValue) -> AWKValue { .number(a.numberValue + b.numberValue) }
    static func sub(_ a: AWKValue, _ b: AWKValue) -> AWKValue { .number(a.numberValue - b.numberValue) }
    static func mul(_ a: AWKValue, _ b: AWKValue) -> AWKValue { .number(a.numberValue * b.numberValue) }
    static func div(_ a: AWKValue, _ b: AWKValue) -> AWKValue {
        let d = b.numberValue
        guard d != 0 else { fputs("awk: division by zero\n", stderr); return .number(0) }
        return .number(a.numberValue / d)
    }
    static func mod(_ a: AWKValue, _ b: AWKValue) -> AWKValue {
        let d = b.numberValue
        guard d != 0 else { fputs("awk: modulo by zero\n", stderr); return .number(0) }
        return .number(a.numberValue.truncatingRemainder(dividingBy: d))
    }
    static func pow(_ a: AWKValue, _ b: AWKValue) -> AWKValue { .number(Foundation.pow(a.numberValue, b.numberValue)) }
}

// MARK: Lexer

enum TokenKind: Equatable {
    case number(Double), string(String), identifier(String), regex(String)
    case plus, minus, star, slash, percent, caret
    case plusEq, minusEq, starEq, slashEq, percentEq, caretEq
    case plusPlus, minusMinus
    case eq, eqEq, bangEq, lt, ltEq, gt, gtEq
    case and, or, bang
    case pipe, getline
    case tilde, bangTilde
    case comma, semicolon, colon, question
    case lbrace, rbrace, lparen, rparen, lbracket, rbracket
    case dollar, newline, eof
    case kw_BEGIN, kw_END
    case kw_if, kw_else, kw_while, kw_do, kw_for, kw_in
    case kw_break, kw_continue, kw_next, kw_exit
    case kw_print, kw_printf, kw_delete, kw_return
    case kw_function, kw_getline
    case redirect_gt, redirect_append, redirect_pipe
}

struct Token {
    let kind: TokenKind
    let line: Int
}

struct Lexer {
    let source: [Character]
    var pos: Int = 0
    var line: Int = 1

    init(source: String) { self.source = Array(source) }

    var current: Character? { pos < source.count ? source[pos] : nil }
    var next: Character? { pos + 1 < source.count ? source[pos + 1] : nil }

    mutating func advance() { if let c = current { if c == "\n" { line += 1 }; pos += 1 } }

    mutating func skipWhitespaceAndComments() {
        while let c = current {
            if c == "#" {
                while let c2 = current, c2 != "\n" { advance() }
            } else if c == " " || c == "\t" || c == "\r" {
                advance()
            } else { break }
        }
    }

    mutating func nextToken() throws -> Token {
        skipWhitespaceAndComments()
        guard let c = current else { return Token(kind: .eof, line: line) }
        let ln = line

        if c == "\n" { advance(); return Token(kind: .newline, line: ln) }

        // String literal
        if c == "\"" {
            advance()
            var buf = ""
            while let ch = current, ch != "\"" {
                if ch == "\\" {
                    advance()
                    if let esc = current {
                        switch esc {
                        case "n": buf.append("\n")
                        case "t": buf.append("\t")
                        case "r": buf.append("\r")
                        case "\\": buf.append("\\")
                        case "\"": buf.append("\"")
                        case "/": buf.append("/")
                        default: buf.append("\\"); buf.append(esc)
                        }
                        advance()
                    }
                } else { buf.append(ch); advance() }
            }
            advance() // closing "
            return Token(kind: .string(buf), line: ln)
        }

        // Number
        if c.isNumber || (c == "." && next?.isNumber == true) {
            var buf = ""
            while let ch = current, ch.isNumber { buf.append(ch); advance() }
            if current == "." { buf.append("."); advance()
                while let ch = current, ch.isNumber { buf.append(ch); advance() }
            }
            if current == "e" || current == "E" {
                buf.append(current!); advance()
                if current == "+" || current == "-" { buf.append(current!); advance() }
                while let ch = current, ch.isNumber { buf.append(ch); advance() }
            }
            return Token(kind: .number(Double(buf) ?? 0), line: ln)
        }

        // Identifier / keyword
        if c.isLetter || c == "_" {
            var buf = ""
            while let ch = current, ch.isLetter || ch.isNumber || ch == "_" { buf.append(ch); advance() }
            let kind: TokenKind
            switch buf {
            case "BEGIN": kind = .kw_BEGIN
            case "END": kind = .kw_END
            case "if": kind = .kw_if
            case "else": kind = .kw_else
            case "while": kind = .kw_while
            case "do": kind = .kw_do
            case "for": kind = .kw_for
            case "in": kind = .kw_in
            case "break": kind = .kw_break
            case "continue": kind = .kw_continue
            case "next": kind = .kw_next
            case "exit": kind = .kw_exit
            case "print": kind = .kw_print
            case "printf": kind = .kw_printf
            case "delete": kind = .kw_delete
            case "return": kind = .kw_return
            case "function": kind = .kw_function
            case "getline": kind = .kw_getline
            default: kind = .identifier(buf)
            }
            return Token(kind: kind, line: ln)
        }

        // Operators and punctuation
        advance()
        switch c {
        case "+":
            if current == "+" { advance(); return Token(kind: .plusPlus, line: ln) }
            if current == "=" { advance(); return Token(kind: .plusEq, line: ln) }
            return Token(kind: .plus, line: ln)
        case "-":
            if current == "-" { advance(); return Token(kind: .minusMinus, line: ln) }
            if current == "=" { advance(); return Token(kind: .minusEq, line: ln) }
            return Token(kind: .minus, line: ln)
        case "*":
            if current == "=" { advance(); return Token(kind: .starEq, line: ln) }
            return Token(kind: .star, line: ln)
        case "/":
            if current == "=" { advance(); return Token(kind: .slashEq, line: ln) }
            return Token(kind: .slash, line: ln)
        case "%":
            if current == "=" { advance(); return Token(kind: .percentEq, line: ln) }
            return Token(kind: .percent, line: ln)
        case "^":
            if current == "=" { advance(); return Token(kind: .caretEq, line: ln) }
            return Token(kind: .caret, line: ln)
        case "=":
            if current == "=" { advance(); return Token(kind: .eqEq, line: ln) }
            return Token(kind: .eq, line: ln)
        case "!":
            if current == "=" { advance(); return Token(kind: .bangEq, line: ln) }
            if current == "~" { advance(); return Token(kind: .bangTilde, line: ln) }
            return Token(kind: .bang, line: ln)
        case "<":
            if current == "=" { advance(); return Token(kind: .ltEq, line: ln) }
            return Token(kind: .lt, line: ln)
        case ">":
            if current == ">" { advance(); return Token(kind: .redirect_append, line: ln) }
            if current == "=" { advance(); return Token(kind: .gtEq, line: ln) }
            return Token(kind: .gt, line: ln)
        case "&":
            if current == "&" { advance(); return Token(kind: .and, line: ln) }
            throw AWKError(message: "unexpected character '&'")
        case "|":
            if current == "|" { advance(); return Token(kind: .or, line: ln) }
            return Token(kind: .pipe, line: ln)
        case "~": return Token(kind: .tilde, line: ln)
        case ",": return Token(kind: .comma, line: ln)
        case ";": return Token(kind: .semicolon, line: ln)
        case ":": return Token(kind: .colon, line: ln)
        case "?": return Token(kind: .question, line: ln)
        case "{": return Token(kind: .lbrace, line: ln)
        case "}": return Token(kind: .rbrace, line: ln)
        case "(": return Token(kind: .lparen, line: ln)
        case ")": return Token(kind: .rparen, line: ln)
        case "[": return Token(kind: .lbracket, line: ln)
        case "]": return Token(kind: .rbracket, line: ln)
        case "$": return Token(kind: .dollar, line: ln)
        case "\\":
            if current == "\n" { advance(); return try nextToken() } // line continuation
            throw AWKError(message: "unexpected '\\'")
        default:
            throw AWKError(message: "unexpected character '\(c)'")
        }
    }

    // Lex a regex literal (called by parser when '/' is expected as regex, not division)
    mutating func lexRegex() throws -> Token {
        let ln = line
        var buf = ""
        while let ch = current, ch != "/" {
            if ch == "\n" { throw AWKError(message: "unterminated regex") }
            if ch == "\\" { advance(); if let esc = current { buf.append("\\"); buf.append(esc); advance() } }
            else { buf.append(ch); advance() }
        }
        advance() // closing /
        return Token(kind: .regex(buf), line: ln)
    }
}

// MARK: AST

indirect enum AWKExpr {
    case number(Double)
    case string(String)
    case regex(String)
    case fieldAccess(AWKExpr)
    case variable(String)
    case arrayAccess(String, [AWKExpr])
    case assign(AWKExpr, AWKExpr)
    case compoundAssign(String, AWKExpr, AWKExpr)  // op, lhs, rhs
    case preIncDec(String, AWKExpr)
    case postIncDec(AWKExpr, String)
    case unary(String, AWKExpr)
    case binary(String, AWKExpr, AWKExpr)
    case ternary(AWKExpr, AWKExpr, AWKExpr)
    case match(AWKExpr, AWKExpr)       // ~ or !~
    case functionCall(String, [AWKExpr])
    case getline(AWKExpr?)             // optional variable to read into
    case inArray(AWKExpr, String)      // expr in array
    case concat([AWKExpr])
}

indirect enum AWKStmt {
    case block([AWKStmt])
    case expr(AWKExpr)
    case print([AWKExpr], AWKRedirect?)
    case printf([AWKExpr], AWKRedirect?)
    case ifStmt(AWKExpr, AWKStmt, AWKStmt?)
    case whileStmt(AWKExpr, AWKStmt)
    case doWhileStmt(AWKStmt, AWKExpr)
    case forStmt(AWKStmt?, AWKExpr?, AWKStmt?, AWKStmt)
    case forInStmt(String, String, AWKStmt)
    case breakStmt, continueStmt, nextStmt
    case exitStmt(AWKExpr?)
    case returnStmt(AWKExpr?)
    case deleteStmt(String, [AWKExpr]?)
    case empty
}

enum AWKRedirect {
    case file(AWKExpr)
    case append(AWKExpr)
    case pipe(AWKExpr)
}

struct AWKRule {
    let pattern: AWKPattern
    let action: AWKStmt
}

enum AWKPattern {
    case begin, end
    case expr(AWKExpr)
    case range(AWKExpr, AWKExpr)
    case always
}

struct AWKFunction {
    let name: String
    let params: [String]
    let body: AWKStmt
}

struct AWKProgram {
    let rules: [AWKRule]
    let functions: [String: AWKFunction]
}

// MARK: Parser

struct AWKParser {
    var lexer: Lexer
    var current: Token
    var peeked: Token?

    init(source: String) throws {
        lexer = Lexer(source: source)
        current = Token(kind: .eof, line: 1)
        current = try lexer.nextToken()
    }

    mutating func peek() throws -> Token {
        if let p = peeked { return p }
        let t = try lexer.nextToken()
        peeked = t
        return t
    }

    mutating func advance() throws -> Token {
        let t = current
        if let p = peeked { current = p; peeked = nil }
        else { current = try lexer.nextToken() }
        return t
    }

    mutating func skipNewlines() throws {
        while case .newline = current.kind { _ = try advance() }
    }

    mutating func skipTerminators() throws {
        while case .newline = current.kind { _ = try advance() }
        while case .semicolon = current.kind { _ = try advance()
            while case .newline = current.kind { _ = try advance() } }
    }

    mutating func expect(_ kind: TokenKind) throws {
        guard current.kind == kind else {
            throw AWKError(message: "expected \(kind) at line \(current.line), got \(current.kind)")
        }
        _ = try advance()
    }

    mutating func parse() throws -> AWKProgram {
        var rules: [AWKRule] = []
        var functions: [String: AWKFunction] = [:]
        try skipTerminators()
        while case .eof = current.kind {} == false {
            if case .kw_function = current.kind {
                let fn = try parseFunction()
                functions[fn.name] = fn
            } else {
                let rule = try parseRule()
                rules.append(rule)
            }
            try skipTerminators()
        }
        return AWKProgram(rules: rules, functions: functions)
    }

    mutating func parseFunction() throws -> AWKFunction {
        _ = try advance() // consume 'function'
        guard case .identifier(let name) = current.kind else {
            throw AWKError(message: "expected function name at line \(current.line)")
        }
        _ = try advance()
        try expect(.lparen)
        var params: [String] = []
        while case .identifier(let p) = current.kind {
            params.append(p)
            _ = try advance()
            if case .comma = current.kind { _ = try advance() }
        }
        try expect(.rparen)
        try skipNewlines()
        let body = try parseBlock()
        return AWKFunction(name: name, params: params, body: body)
    }

    mutating func parseRule() throws -> AWKRule {
        let pattern: AWKPattern
        switch current.kind {
        case .kw_BEGIN: _ = try advance(); pattern = .begin
        case .kw_END:   _ = try advance(); pattern = .end
        case .lbrace:   pattern = .always
        default:
            let expr1 = try parseExpr()
            if case .comma = current.kind {
                _ = try advance()
                try skipNewlines()
                let expr2 = try parseExpr()
                pattern = .range(expr1, expr2)
            } else {
                pattern = .expr(expr1)
            }
        }
        try skipNewlines()
        let action: AWKStmt
        if case .lbrace = current.kind {
            action = try parseBlock()
        } else if case .always = pattern {
            throw AWKError(message: "pattern without action at line \(current.line)")
        } else {
            // Default action: print $0
            action = .print([], nil)
        }
        return AWKRule(pattern: pattern, action: action)
    }

    mutating func parseBlock() throws -> AWKStmt {
        try expect(.lbrace)
        try skipTerminators()
        var stmts: [AWKStmt] = []
        while !(current.kind == .rbrace) {
            if case .eof = current.kind { throw AWKError(message: "unclosed block") }
            let s = try parseStmt()
            stmts.append(s)
            try skipTerminators()
        }
        _ = try advance() // consume }
        return .block(stmts)
    }

    mutating func parseStmt() throws -> AWKStmt {
        switch current.kind {
        case .kw_if: return try parseIf()
        case .kw_while: return try parseWhile()
        case .kw_do: return try parseDo()
        case .kw_for: return try parseFor()
        case .kw_break:  _ = try advance(); return .breakStmt
        case .kw_continue: _ = try advance(); return .continueStmt
        case .kw_next:   _ = try advance(); return .nextStmt
        case .kw_exit:
            _ = try advance()
            if case .newline = current.kind { return .exitStmt(nil) }
            if case .semicolon = current.kind { return .exitStmt(nil) }
            if case .rbrace = current.kind { return .exitStmt(nil) }
            return .exitStmt(try parseExpr())
        case .kw_return:
            _ = try advance()
            if case .newline = current.kind { return .returnStmt(nil) }
            if case .semicolon = current.kind { return .returnStmt(nil) }
            if case .rbrace = current.kind { return .returnStmt(nil) }
            return .returnStmt(try parseExpr())
        case .kw_delete:
            _ = try advance()
            guard case .identifier(let name) = current.kind else {
                throw AWKError(message: "expected array name after delete")
            }
            _ = try advance()
            if case .lbracket = current.kind {
                _ = try advance()
                var keys: [AWKExpr] = [try parseExpr()]
                while case .comma = current.kind { _ = try advance(); keys.append(try parseExpr()) }
                try expect(.rbracket)
                return .deleteStmt(name, keys)
            }
            return .deleteStmt(name, nil)
        case .kw_print, .kw_printf:
            let isPrintf = { if case .kw_printf = current.kind { return true }; return false }()
            _ = try advance()
            var args: [AWKExpr] = []
            let hasParen = current.kind == .lparen
            if hasParen { _ = try advance() }
            if !(current.kind == .newline || current.kind == .semicolon || current.kind == .rbrace || current.kind == .eof) {
                args.append(try parseExpr())
                while case .comma = current.kind { _ = try advance(); args.append(try parseExpr()) }
            }
            if hasParen { if case .rparen = current.kind { _ = try advance() } }
            let redirect = try parseRedirect()
            return isPrintf ? .printf(args, redirect) : .print(args, redirect)
        case .lbrace:
            return try parseBlock()
        default:
            let expr = try parseExpr()
            return .expr(expr)
        }
    }

    mutating func parseRedirect() throws -> AWKRedirect? {
        switch current.kind {
        case .gt:
            _ = try advance()
            if case .gt = current.kind { _ = try advance(); return .append(try parsePrimary()) }
            return .file(try parsePrimary())
        case .redirect_append:
            _ = try advance()
            return .append(try parsePrimary())
        case .pipe:
            _ = try advance()
            return .pipe(try parsePrimary())
        default: return nil
        }
    }

    mutating func parseIf() throws -> AWKStmt {
        _ = try advance()
        try expect(.lparen)
        let cond = try parseExpr()
        try expect(.rparen)
        try skipNewlines()
        let then = try parseStmt()
        try skipTerminators()
        if case .kw_else = current.kind {
            _ = try advance()
            try skipNewlines()
            let els = try parseStmt()
            return .ifStmt(cond, then, els)
        }
        return .ifStmt(cond, then, nil)
    }

    mutating func parseWhile() throws -> AWKStmt {
        _ = try advance()
        try expect(.lparen)
        let cond = try parseExpr()
        try expect(.rparen)
        try skipNewlines()
        let body = try parseStmt()
        return .whileStmt(cond, body)
    }

    mutating func parseDo() throws -> AWKStmt {
        _ = try advance()
        try skipNewlines()
        let body = try parseStmt()
        try skipTerminators()
        _ = try advance() // while
        try expect(.lparen)
        let cond = try parseExpr()
        try expect(.rparen)
        return .doWhileStmt(body, cond)
    }

    mutating func parseFor() throws -> AWKStmt {
        _ = try advance()
        try expect(.lparen)
        // Detect for-in: (var in array)
        if case .identifier(let varName) = current.kind {
            let saved = current
            _ = try advance()
            if case .kw_in = current.kind {
                _ = try advance()
                guard case .identifier(let arrayName) = current.kind else {
                    throw AWKError(message: "expected array name in for-in")
                }
                _ = try advance()
                try expect(.rparen)
                try skipNewlines()
                let body = try parseStmt()
                return .forInStmt(varName, arrayName, body)
            }
            // Backtrack: re-inject saved token â€” not ideal, but we can parse init expr from remaining
            // We re-lex by treating saved as start of expression; since we can't truly backtrack,
            // we use a workaround: build a variable expr from the saved identifier
            let initExpr: AWKExpr = .variable(varName)
            // Continue parsing potential assignment
            let fullInit: AWKExpr
            switch current.kind {
            case .eq:
                _ = try advance()
                let rhs = try parseExpr()
                fullInit = .assign(initExpr, rhs)
            case .plusEq, .minusEq, .starEq, .slashEq, .percentEq, .caretEq:
                let op = operatorString(current.kind)
                _ = try advance()
                let rhs = try parseExpr()
                fullInit = .compoundAssign(op, initExpr, rhs)
            default:
                fullInit = initExpr
            }
            try expect(.semicolon)
            let cond: AWKExpr? = (current.kind == .semicolon) ? nil : try parseExpr()
            try expect(.semicolon)
            let post: AWKStmt? = (current.kind == .rparen) ? nil : .expr(try parseExpr())
            try expect(.rparen)
            try skipNewlines()
            let body = try parseStmt()
            return .forStmt(.expr(fullInit), cond, post, body)
        }
        // General for
        let init_: AWKStmt?
        if case .semicolon = current.kind { init_ = nil } else { init_ = try parseStmt() }
        try expect(.semicolon)
        let cond: AWKExpr? = (current.kind == .semicolon) ? nil : try parseExpr()
        try expect(.semicolon)
        let post: AWKStmt? = (current.kind == .rparen) ? nil : .expr(try parseExpr())
        try expect(.rparen)
        try skipNewlines()
        let body = try parseStmt()
        return .forStmt(init_, cond, post, body)
    }

    func operatorString(_ kind: TokenKind) -> String {
        switch kind {
        case .plusEq: return "+"; case .minusEq: return "-"
        case .starEq: return "*"; case .slashEq: return "/"
        case .percentEq: return "%"; case .caretEq: return "^"
        default: return ""
        }
    }

    // Expression parsing â€” precedence climbing
    mutating func parseExpr() throws -> AWKExpr { try parseTernary() }

    mutating func parseTernary() throws -> AWKExpr {
        let lhs = try parseOr()
        if case .question = current.kind {
            _ = try advance()
            let t = try parseTernary()
            try expect(.colon)
            let f = try parseTernary()
            return .ternary(lhs, t, f)
        }
        // Assignment
        switch current.kind {
        case .eq:
            _ = try advance()
            return .assign(lhs, try parseTernary())
        case .plusEq, .minusEq, .starEq, .slashEq, .percentEq, .caretEq:
            let op = operatorString(current.kind)
            _ = try advance()
            return .compoundAssign(op, lhs, try parseTernary())
        default: return lhs
        }
    }

    mutating func parseOr() throws -> AWKExpr {
        var lhs = try parseAnd()
        while case .or = current.kind { _ = try advance(); lhs = .binary("||", lhs, try parseAnd()) }
        return lhs
    }

    mutating func parseAnd() throws -> AWKExpr {
        var lhs = try parseMatch()
        while case .and = current.kind { _ = try advance(); lhs = .binary("&&", lhs, try parseMatch()) }
        return lhs
    }

    mutating func parseMatch() throws -> AWKExpr {
        var lhs = try parseIn()
        while true {
            if case .tilde = current.kind { _ = try advance(); lhs = .match(lhs, try parseIn()) }
            else if case .bangTilde = current.kind { _ = try advance(); lhs = .unary("!", .match(lhs, try parseIn())) }
            else { break }
        }
        return lhs
    }

    mutating func parseIn() throws -> AWKExpr {
        var lhs = try parseComparison()
        while case .kw_in = current.kind {
            _ = try advance()
            guard case .identifier(let name) = current.kind else {
                throw AWKError(message: "expected array name after 'in'")
            }
            _ = try advance()
            lhs = .inArray(lhs, name)
        }
        return lhs
    }

    mutating func parseComparison() throws -> AWKExpr {
        var lhs = try parseConcat()
        while true {
            let op: String
            switch current.kind {
            case .lt: op = "<"; case .ltEq: op = "<="
            case .gt: op = ">"; case .gtEq: op = ">="
            case .eqEq: op = "=="; case .bangEq: op = "!="
            default: return lhs
            }
            _ = try advance()
            lhs = .binary(op, lhs, try parseConcat())
        }
    }

    mutating func parseConcat() throws -> AWKExpr {
        var parts = [try parseAdd()]
        // Concatenation: adjacent expressions (no operator)
        while true {
            switch current.kind {
            case .newline, .semicolon, .rbrace, .rparen, .rbracket,
                 .comma, .eof, .pipe, .gt, .redirect_append,
                 .question, .colon, .eq, .plusEq, .minusEq, .starEq,
                 .slashEq, .percentEq, .caretEq, .and, .or,
                 .tilde, .bangTilde, .kw_in,
                 .lt, .ltEq, .gtEq, .eqEq, .bangEq:
                if parts.count == 1 { return parts[0] }
                return .concat(parts)
            default:
                parts.append(try parseAdd())
            }
        }
    }

    mutating func parseAdd() throws -> AWKExpr {
        var lhs = try parseMul()
        while true {
            if case .plus = current.kind { _ = try advance(); lhs = .binary("+", lhs, try parseMul()) }
            else if case .minus = current.kind { _ = try advance(); lhs = .binary("-", lhs, try parseMul()) }
            else { break }
        }
        return lhs
    }

    mutating func parseMul() throws -> AWKExpr {
        var lhs = try parsePow()
        while true {
            if case .star = current.kind { _ = try advance(); lhs = .binary("*", lhs, try parsePow()) }
            else if case .slash = current.kind { _ = try advance(); lhs = .binary("/", lhs, try parsePow()) }
            else if case .percent = current.kind { _ = try advance(); lhs = .binary("%", lhs, try parsePow()) }
            else { break }
        }
        return lhs
    }

    mutating func parsePow() throws -> AWKExpr {
        let lhs = try parseUnary()
        if case .caret = current.kind { _ = try advance(); return .binary("^", lhs, try parsePow()) }
        return lhs
    }

    mutating func parseUnary() throws -> AWKExpr {
        if case .bang = current.kind { _ = try advance(); return .unary("!", try parseUnary()) }
        if case .minus = current.kind { _ = try advance(); return .unary("-", try parseUnary()) }
        if case .plus = current.kind { _ = try advance(); return .unary("+", try parseUnary()) }
        if case .plusPlus = current.kind { _ = try advance(); return .preIncDec("++", try parseUnary()) }
        if case .minusMinus = current.kind { _ = try advance(); return .preIncDec("--", try parseUnary()) }
        return try parsePostfix()
    }

    mutating func parsePostfix() throws -> AWKExpr {
        var expr = try parsePrimary()
        if case .plusPlus = current.kind { _ = try advance(); expr = .postIncDec(expr, "++") }
        else if case .minusMinus = current.kind { _ = try advance(); expr = .postIncDec(expr, "--") }
        return expr
    }

    mutating func parsePrimary() throws -> AWKExpr {
        switch current.kind {
        case .number(let n): _ = try advance(); return .number(n)
        case .string(let s): _ = try advance(); return .string(s)
        case .dollar:
            _ = try advance()
            return .fieldAccess(try parseUnary())
        case .lparen:
            _ = try advance()
            let e = try parseExpr()
            try expect(.rparen)
            return e
        case .bang:
            _ = try advance()
            return .unary("!", try parsePrimary())
        case .slash:
            // Regex literal
            _ = try advance()
            let tok = try lexer.lexRegex()
            current = try lexer.nextToken()
            if case .regex(let pat) = tok.kind { return .regex(pat) }
            throw AWKError(message: "expected regex")
        case .identifier(let name):
            _ = try advance()
            // Array access
            if case .lbracket = current.kind {
                _ = try advance()
                var keys: [AWKExpr] = [try parseExpr()]
                while case .comma = current.kind { _ = try advance(); keys.append(try parseExpr()) }
                try expect(.rbracket)
                return .arrayAccess(name, keys)
            }
            // Function call
            if case .lparen = current.kind {
                _ = try advance()
                var args: [AWKExpr] = []
                if !(current.kind == .rparen) {
                    args.append(try parseExpr())
                    while case .comma = current.kind { _ = try advance(); args.append(try parseExpr()) }
                }
                try expect(.rparen)
                return .functionCall(name, args)
            }
            return .variable(name)
        case .kw_getline:
            _ = try advance()
            if case .identifier(let v) = current.kind {
                _ = try advance()
                return .getline(.variable(v))
            }
            return .getline(nil)
        default:
            throw AWKError(message: "unexpected token \(current.kind) at line \(current.line)")
        }
    }
}

// MARK: Control flow signals

struct AWKBreak: Error {}
struct AWKContinue: Error {}
struct AWKNext: Error {}
struct AWKExit: Error { let code: AWKValue }
struct AWKReturn: Error { let value: AWKValue }

// MARK: Interpreter

class AWKInterpreter {
    var globals: [String: AWKValue] = [:]
    var arrays: [String: [String: AWKValue]] = [:]
    var fields: [String] = []
    var record: String = ""
    var fieldSeparator: String
    var outputSeparator: String = " "
    var outputRecordSeparator: String = "\n"
    var NR: Int = 0
    var NF: Int = 0
    var FILENAME: String = ""
    var openFiles: [String: FileHandle] = [:]
    var openPipes: [String: Pipe] = [:]
    var rangeActive: [Int: Bool] = [:]
    var program: AWKProgram = AWKProgram(rules: [], functions: [:])
    var callStack: [[String: AWKValue]] = []

    init(fieldSeparator: String) {
        self.fieldSeparator = fieldSeparator
    }

    func setVariable(_ name: String, value: AWKValue) {
        globals[name] = value
    }

    // MARK: Split record into fields

    func splitRecord(_ line: String) {
        record = line
        if fieldSeparator == " " {
            fields = line.components(separatedBy: .whitespaces).filter { !$0.isEmpty }
        } else if fieldSeparator.count == 1 {
            fields = line.components(separatedBy: fieldSeparator)
        } else {
            // Regex separator
            do {
                let rx = try NSRegularExpression(pattern: fieldSeparator)
                let ns = line as NSString
                let matches = rx.matches(in: line, range: NSRange(location: 0, length: ns.length))
                var parts: [String] = []
                var last = 0
                for m in matches {
                    parts.append(ns.substring(with: NSRange(location: last, length: m.range.location - last)))
                    last = m.range.location + m.range.length
                }
                parts.append(ns.substring(from: last))
                fields = parts
            } catch {
                fields = [line]
            }
        }
        NF = fields.count
    }

    func getField(_ index: Int) -> AWKValue {
        if index == 0 { return .string(record) }
        if index < 1 || index > fields.count { return .uninitialized }
        return .string(fields[index - 1])
    }

    func setField(_ index: Int, value: AWKValue) {
        if index == 0 {
            record = value.stringValue
            splitRecord(record)
            return
        }
        while fields.count < index { fields.append("") }
        fields[index - 1] = value.stringValue
        NF = fields.count
        record = fields.joined(separator: outputSeparator)
    }

    // MARK: Run

    func run(program: AWKProgram, input: AnyIterator<String?>) {
        self.program = program

        // BEGIN rules
        for rule in program.rules {
            if case .begin = rule.pattern {
                do { try execStmt(rule.action, locals: &globals) }
                catch let e as AWKExit { exit(Int32(e.code.numberValue)) }
                catch {}
            }
        }

        // Per-record rules
        while let maybeLine = input.next(), let line = maybeLine {
            NR += 1
            splitRecord(line)
            for (i, rule) in program.rules.enumerated() {
                switch rule.pattern {
                case .begin, .end: continue
                case .always:
                    do { try execStmt(rule.action, locals: &globals) }
                    catch is AWKNext { break }
                    catch let e as AWKExit { exit(Int32(e.code.numberValue)) }
                    catch {}
                case .expr(let cond):
                    do {
                        let val = try evalExpr(cond, locals: &globals)
                        if val.boolValue {
                            try execStmt(rule.action, locals: &globals)
                        }
                    }
                    catch is AWKNext { break }
                    catch let e as AWKExit { exit(Int32(e.code.numberValue)) }
                    catch {}
                case .range(let start, let end):
                    let active = rangeActive[i] ?? false
                    if !active {
                        do {
                            let v = try evalExpr(start, locals: &globals)
                            if v.boolValue {
                                rangeActive[i] = true
                                do { try execStmt(rule.action, locals: &globals) }
                                catch is AWKNext { break }
                                catch let e as AWKExit { exit(Int32(e.code.numberValue)) }
                                catch {}
                            }
                        } catch {}
                    } else {
                        do { try execStmt(rule.action, locals: &globals) }
                        catch is AWKNext { break }
                        catch let e as AWKExit { exit(Int32(e.code.numberValue)) }
                        catch {}
                        do {
                            let v = try evalExpr(end, locals: &globals)
                            if v.boolValue { rangeActive[i] = false }
                        } catch {}
                    }
                }
            }
        }

        // END rules
        for rule in program.rules {
            if case .end = rule.pattern {
                do { try execStmt(rule.action, locals: &globals) }
                catch let e as AWKExit { exit(Int32(e.code.numberValue)) }
                catch {}
            }
        }

        // Close open files/pipes
        for (_, fh) in openFiles { fh.closeFile() }
    }

    // MARK: Statement execution

    func execStmt(_ stmt: AWKStmt, locals: inout [String: AWKValue]) throws {
        switch stmt {
        case .empty: break
        case .block(let stmts):
            for s in stmts { try execStmt(s, locals: &locals) }
        case .expr(let e):
            _ = try evalExpr(e, locals: &locals)
        case .print(let args, let redirect):
            let output: String
            if args.isEmpty {
                output = record
            } else {
                let parts = try args.map { try evalExpr($0, locals: &locals).stringValue }
                output = parts.joined(separator: outputSeparator)
            }
            writeOutput(output + outputRecordSeparator, redirect: redirect, locals: &locals)
        case .printf(let args, let redirect):
            guard !args.isEmpty else { break }
            let fmt = try evalExpr(args[0], locals: &locals).stringValue
            let rest = try args.dropFirst().map { try evalExpr($0, locals: &locals) }
            let formatted = sprintfFormat(fmt, args: rest)
            writeOutput(formatted, redirect: redirect, locals: &locals)
        case .ifStmt(let cond, let then, let els):
            let v = try evalExpr(cond, locals: &locals)
            if v.boolValue { try execStmt(then, locals: &locals) }
            else if let e = els { try execStmt(e, locals: &locals) }
        case .whileStmt(let cond, let body):
            while (try evalExpr(cond, locals: &locals)).boolValue {
                do { try execStmt(body, locals: &locals) }
                catch is AWKBreak { break }
                catch is AWKContinue { continue }
            }
        case .doWhileStmt(let body, let cond):
            repeat {
                do { try execStmt(body, locals: &locals) }
                catch is AWKBreak { break }
                catch is AWKContinue { continue }
            } while (try evalExpr(cond, locals: &locals)).boolValue
        case .forStmt(let init_, let cond, let post, let body):
            if let i = init_ { try execStmt(i, locals: &locals) }
            while true {
                if let c = cond {
                    let v = try evalExpr(c, locals: &locals)
                    if !v.boolValue { break }
                }
                do { try execStmt(body, locals: &locals) }
                catch is AWKBreak { break }
                catch is AWKContinue { }
                if let p = post { try execStmt(p, locals: &locals) }
            }
        case .forInStmt(let varName, let arrayName, let body):
            let arr = arrays[arrayName] ?? [:]
            for key in arr.keys {
                setVar(varName, value: .string(key), locals: &locals)
                do { try execStmt(body, locals: &locals) }
                catch is AWKBreak { break }
                catch is AWKContinue { continue }
            }
        case .breakStmt: throw AWKBreak()
        case .continueStmt: throw AWKContinue()
        case .nextStmt: throw AWKNext()
        case .exitStmt(let expr):
            let code = expr != nil ? try evalExpr(expr!, locals: &locals) : AWKValue.number(0)
            throw AWKExit(code: code)
        case .returnStmt(let expr):
            let val = expr != nil ? try evalExpr(expr!, locals: &locals) : AWKValue.uninitialized
            throw AWKReturn(value: val)
        case .deleteStmt(let name, let keys):
            if let keys = keys {
                let key = try keys.map { try evalExpr($0, locals: &locals).stringValue }.joined(separator: "\u{001C}")
                arrays[name]?.removeValue(forKey: key)
            } else {
                arrays[name] = [:]
            }
        }
    }

    // MARK: Expression evaluation

    func evalExpr(_ expr: AWKExpr, locals: inout [String: AWKValue]) throws -> AWKValue {
        switch expr {
        case .number(let n): return .number(n)
        case .string(let s): return .string(s)
        case .regex(let pat):
            // bare regex matches against $0
            return matchRegex(record, pattern: pat) ? .number(1) : .number(0)
        case .fieldAccess(let idxExpr):
            let idx = Int(try evalExpr(idxExpr, locals: &locals).numberValue)
            return getField(idx)
        case .variable(let name):
            return getVar(name, locals: &locals)
        case .arrayAccess(let name, let keys):
            let key = try keys.map { try evalExpr($0, locals: &locals).stringValue }.joined(separator: "\u{001C}")
            return arrays[name]?[key] ?? .uninitialized
        case .assign(let lhs, let rhs):
            let val = try evalExpr(rhs, locals: &locals)
            try assignTo(lhs, value: val, locals: &locals)
            return val
        case .compoundAssign(let op, let lhs, let rhs):
            let current = try evalExpr(lhs, locals: &locals)
            let rval = try evalExpr(rhs, locals: &locals)
            let result: AWKValue
            switch op {
            case "+": result = AWKValue.add(current, rval)
            case "-": result = AWKValue.sub(current, rval)
            case "*": result = AWKValue.mul(current, rval)
            case "/": result = AWKValue.div(current, rval)
            case "%": result = AWKValue.mod(current, rval)
            case "^": result = AWKValue.pow(current, rval)
            default: result = rval
            }
            try assignTo(lhs, value: result, locals: &locals)
            return result
        case .preIncDec(let op, let e):
            let cur = try evalExpr(e, locals: &locals)
            let next: AWKValue = op == "++" ? .number(cur.numberValue + 1) : .number(cur.numberValue - 1)
            try assignTo(e, value: next, locals: &locals)
            return next
        case .postIncDec(let e, let op):
            let cur = try evalExpr(e, locals: &locals)
            let next: AWKValue = op == "++" ? .number(cur.numberValue + 1) : .number(cur.numberValue - 1)
            try assignTo(e, value: next, locals: &locals)
            return cur
        case .unary(let op, let e):
            let v = try evalExpr(e, locals: &locals)
            switch op {
            case "-": return .number(-v.numberValue)
            case "+": return .number(v.numberValue)
            case "!": return .number(v.boolValue ? 0 : 1)
            default: return v
            }
        case .binary(let op, let lhsExpr, let rhsExpr):
            switch op {
            case "&&":
                let l = try evalExpr(lhsExpr, locals: &locals)
                if !l.boolValue { return .number(0) }
                return try evalExpr(rhsExpr, locals: &locals).boolValue ? .number(1) : .number(0)
            case "||":
                let l = try evalExpr(lhsExpr, locals: &locals)
                if l.boolValue { return .number(1) }
                return try evalExpr(rhsExpr, locals: &locals).boolValue ? .number(1) : .number(0)
            default: break
            }
            let l = try evalExpr(lhsExpr, locals: &locals)
            let r = try evalExpr(rhsExpr, locals: &locals)
            switch op {
            case "+": return AWKValue.add(l, r)
            case "-": return AWKValue.sub(l, r)
            case "*": return AWKValue.mul(l, r)
            case "/": return AWKValue.div(l, r)
            case "%": return AWKValue.mod(l, r)
            case "^": return AWKValue.pow(l, r)
            case "<":  return .number(compareValues(l, r) < 0 ? 1 : 0)
            case "<=": return .number(compareValues(l, r) <= 0 ? 1 : 0)
            case ">":  return .number(compareValues(l, r) > 0 ? 1 : 0)
            case ">=": return .number(compareValues(l, r) >= 0 ? 1 : 0)
            case "==": return .number(compareValues(l, r) == 0 ? 1 : 0)
            case "!=": return .number(compareValues(l, r) != 0 ? 1 : 0)
            default: return .uninitialized
            }
        case .ternary(let cond, let t, let f):
            return try (try evalExpr(cond, locals: &locals)).boolValue
                ? evalExpr(t, locals: &locals)
                : evalExpr(f, locals: &locals)
        case .match(let lhsExpr, let patExpr):
            let str = try evalExpr(lhsExpr, locals: &locals).stringValue
            let pat: String
            if case .regex(let p) = patExpr { pat = p }
            else { pat = try evalExpr(patExpr, locals: &locals).stringValue }
            return .number(matchRegex(str, pattern: pat) ? 1 : 0)
        case .inArray(let keyExpr, let arrayName):
            let key = try evalExpr(keyExpr, locals: &locals).stringValue
            return .number((arrays[arrayName]?[key] != nil) ? 1 : 0)
        case .concat(let parts):
            let s = try parts.map { try evalExpr($0, locals: &locals).stringValue }.joined()
            return .string(s)
        case .getline(let varExpr):
            guard let line = readLine(strippingNewline: true) else { return .number(-1) }
            if let v = varExpr {
                try assignTo(v, value: .string(line), locals: &locals)
            } else {
                NR += 1
                splitRecord(line)
            }
            return .number(1)
        case .functionCall(let name, let argExprs):
            return try callFunction(name, argExprs: argExprs, locals: &locals)
        }
    }

    // MARK: Variable access

    func getVar(_ name: String, locals: inout [String: AWKValue]) -> AWKValue {
        // Built-in variables
        switch name {
        case "NR": return .number(Double(NR))
        case "NF": return .number(Double(NF))
        case "FS": return .string(fieldSeparator)
        case "OFS": return .string(outputSeparator)
        case "ORS": return .string(outputRecordSeparator)
        case "RS": return .string("\n")
        case "FILENAME": return .string(FILENAME)
        case "FNR": return .number(Double(NR))
        default: break
        }
        if let v = locals[name] { return v }
        return globals[name] ?? .uninitialized
    }

    func setVar(_ name: String, value: AWKValue, locals: inout [String: AWKValue]) {
        switch name {
        case "FS": fieldSeparator = value.stringValue
        case "OFS": outputSeparator = value.stringValue
        case "ORS": outputRecordSeparator = value.stringValue
        case "NF":
            NF = Int(value.numberValue)
            while fields.count < NF { fields.append("") }
            if fields.count > NF { fields = Array(fields.prefix(NF)) }
            record = fields.joined(separator: outputSeparator)
        default:
            if locals.keys.contains(name) { locals[name] = value }
            else { globals[name] = value }
        }
    }

    func assignTo(_ expr: AWKExpr, value: AWKValue, locals: inout [String: AWKValue]) throws {
        switch expr {
        case .variable(let name): setVar(name, value: value, locals: &locals)
        case .fieldAccess(let idxExpr):
            let idx = Int(try evalExpr(idxExpr, locals: &locals).numberValue)
            setField(idx, value: value)
        case .arrayAccess(let name, let keys):
            let key = try keys.map { try evalExpr($0, locals: &locals).stringValue }.joined(separator: "\u{001C}")
            if arrays[name] == nil { arrays[name] = [:] }
            arrays[name]![key] = value
        default:
            throw AWKError(message: "invalid lvalue")
        }
    }

    // MARK: Built-in functions

    func callFunction(_ name: String, argExprs: [AWKExpr], locals: inout [String: AWKValue]) throws -> AWKValue {
        // User-defined function
        if let fn = program.functions[name] {
            var frame: [String: AWKValue] = [:]
            for (i, param) in fn.params.enumerated() {
                frame[param] = i < argExprs.count ? try evalExpr(argExprs[i], locals: &locals) : .uninitialized
            }
            do { try execStmt(fn.body, locals: &frame) }
            catch let r as AWKReturn { return r.value }
            return .uninitialized
        }
        // Built-ins
        let evalArgs: () throws -> [AWKValue] = { try argExprs.map { try self.evalExpr($0, locals: &locals) } }
        switch name {
        case "length":
            if argExprs.isEmpty { return .number(Double(record.count)) }
            let v = try evalExpr(argExprs[0], locals: &locals)
            if case .arrayAccess(let arrName, _) = argExprs[0] { return .number(Double(arrays[arrName]?.count ?? 0)) }
            if case .variable(let arrName) = argExprs[0], arrays[arrName] != nil {
                return .number(Double(arrays[arrName]!.count))
            }
            return .number(Double(v.stringValue.count))
        case "substr":
            let args = try evalArgs()
            let s = args[0].stringValue
            let start = max(1, Int(args[1].numberValue)) - 1
            if args.count >= 3 {
                let len = Int(args[2].numberValue)
                let end = min(s.count, start + len)
                if start >= s.count { return .string("") }
                let from = s.index(s.startIndex, offsetBy: start)
                let to = s.index(s.startIndex, offsetBy: end)
                return .string(String(s[from..<to]))
            }
            if start >= s.count { return .string("") }
            return .string(String(s[s.index(s.startIndex, offsetBy: start)...]))
        case "index":
            let args = try evalArgs()
            let haystack = args[0].stringValue
            let needle = args[1].stringValue
            if let r = haystack.range(of: needle) {
                return .number(Double(haystack.distance(from: haystack.startIndex, to: r.lowerBound) + 1))
            }
            return .number(0)
        case "split":
            guard argExprs.count >= 2 else { return .number(0) }
            let str = try evalExpr(argExprs[0], locals: &locals).stringValue
            guard case .variable(let arrName) = argExprs[1] else { return .number(0) }
            let sep = argExprs.count >= 3
                ? try evalExpr(argExprs[2], locals: &locals).stringValue
                : fieldSeparator
            arrays[arrName] = [:]
            let parts: [String] = sep == " "
                ? str.components(separatedBy: .whitespaces).filter { !$0.isEmpty }
                : str.components(separatedBy: sep)
            for (i, p) in parts.enumerated() { arrays[arrName]![String(i + 1)] = .string(p) }
            return .number(Double(parts.count))
        case "sub", "gsub":
            let isGlobal = name == "gsub"
            guard argExprs.count >= 2 else { return .number(0) }
            let pat: String
            if case .regex(let p) = argExprs[0] { pat = p }
            else { pat = try evalExpr(argExprs[0], locals: &locals).stringValue }
            let repl = try evalExpr(argExprs[1], locals: &locals).stringValue
            let target: AWKExpr = argExprs.count >= 3 ? argExprs[2] : .fieldAccess(.number(0))
            var str = try evalExpr(target, locals: &locals).stringValue
            var count = 0
            do {
                let rx = try NSRegularExpression(pattern: pat)
                let ns = str as NSString
                let matches = rx.matches(in: str, range: NSRange(location: 0, length: ns.length))
                var result = ""
                var last = str.startIndex
                for m in matches {
                    let range = Range(m.range, in: str)!
                    result += str[last..<range.lowerBound]
                    // Handle & in replacement
                    let matched = String(str[range])
                    result += repl.replacingOccurrences(of: "&", with: matched)
                    last = range.upperBound
                    count += 1
                    if !isGlobal { break }
                }
                result += str[last...]
                str = result
            } catch {}
            try assignTo(target, value: .string(str), locals: &locals)
            return .number(Double(count))
        case "match":
            let args = try evalArgs()
            let str = args[0].stringValue
            let pat = args[1].stringValue
            do {
                let rx = try NSRegularExpression(pattern: pat)
                if let m = rx.firstMatch(in: str, range: NSRange(str.startIndex..., in: str)) {
                    let range = Range(m.range, in: str)!
                    let start = str.distance(from: str.startIndex, to: range.lowerBound) + 1
                    let len = str.distance(from: range.lowerBound, to: range.upperBound)
                    globals["RSTART"] = .number(Double(start))
                    globals["RLENGTH"] = .number(Double(len))
                    return .number(Double(start))
                }
            } catch {}
            globals["RSTART"] = .number(0)
            globals["RLENGTH"] = .number(-1)
            return .number(0)
        case "sprintf":
            let args = try evalArgs()
            guard !args.isEmpty else { return .string("") }
            let fmt = args[0].stringValue
            return .string(sprintfFormat(fmt, args: Array(args.dropFirst())))
        case "printf":
            let args = try evalArgs()
            guard !args.isEmpty else { break }
            print(sprintfFormat(args[0].stringValue, args: Array(args.dropFirst())), terminator: "")
            return .uninitialized
        case "print":
            let args = try evalArgs()
            print(args.map { $0.stringValue }.joined(separator: outputSeparator))
            return .uninitialized
        case "int":
            let args = try evalArgs()
            return .number(Double(Int(args[0].numberValue)))
        case "sqrt":
            let args = try evalArgs()
            return .number(Foundation.sqrt(args[0].numberValue))
        case "log":
            let args = try evalArgs()
            return .number(Foundation.log(args[0].numberValue))
        case "exp":
            let args = try evalArgs()
            return .number(Foundation.exp(args[0].numberValue))
        case "sin":
            let args = try evalArgs()
            return .number(Foundation.sin(args[0].numberValue))
        case "cos":
            let args = try evalArgs()
            return .number(Foundation.cos(args[0].numberValue))
        case "atan2":
            let args = try evalArgs()
            return .number(Foundation.atan2(args[0].numberValue, args[1].numberValue))
        case "rand":
            return .number(Double.random(in: 0..<1))
        case "srand":
            // Swift doesn't expose a seed â€” approximate with time
            return .number(0)
        case "tolower":
            let args = try evalArgs()
            return .string(args[0].stringValue.lowercased())
        case "toupper":
            let args = try evalArgs()
            return .string(args[0].stringValue.uppercased())
        case "system":
            let args = try evalArgs()
            let cmd = args[0].stringValue
            let ret = Foundation.system(cmd)
            return .number(Double(ret))
        case "getline":
            if let line = readLine(strippingNewline: true) {
                NR += 1
                splitRecord(line)
                return .number(1)
            }
            return .number(0)
        default:
            fputs("awk: undefined function '\(name)'\n", stderr)
        }
        return .uninitialized
    }

    // MARK: Helpers

    func compareValues(_ a: AWKValue, _ b: AWKValue) -> Int {
        // Numeric if both look numeric, otherwise string compare
        switch (a, b) {
        case (.number(let x), .number(let y)):
            return x < y ? -1 : x > y ? 1 : 0
        case (.string(let x), .string(let y)):
            let xn = Double(x.trimmingCharacters(in: .whitespaces))
            let yn = Double(y.trimmingCharacters(in: .whitespaces))
            if let xd = xn, let yd = yn { return xd < yd ? -1 : xd > yd ? 1 : 0 }
            return x < y ? -1 : x > y ? 1 : 0
        default:
            let xn = a.numberValue, yn = b.numberValue
            return xn < yn ? -1 : xn > yn ? 1 : 0
        }
    }

    func matchRegex(_ str: String, pattern: String) -> Bool {
        guard let rx = try? NSRegularExpression(pattern: pattern) else { return false }
        let range = NSRange(str.startIndex..., in: str)
        return rx.firstMatch(in: str, range: range) != nil
    }

    func writeOutput(_ text: String, redirect: AWKRedirect?, locals: inout [String: AWKValue]) {
        guard let redirect = redirect else {
            print(text, terminator: "")
            return
        }
        switch redirect {
        case .file(let pathExpr):
            let path = (try? evalExpr(pathExpr, locals: &locals))?.stringValue ?? ""
            if let fh = openFiles[path] {
                fh.write(text.data(using: .utf8) ?? Data())
            } else if FileManager.default.createFile(atPath: path, contents: nil) {
                if let fh = FileHandle(forWritingAtPath: path) {
                    openFiles[path] = fh
                    fh.write(text.data(using: .utf8) ?? Data())
                }
            }
        case .append(let pathExpr):
            let path = (try? evalExpr(pathExpr, locals: &locals))?.stringValue ?? ""
            if let fh = FileHandle(forUpdatingAtPath: path) {
                fh.seekToEndOfFile()
                fh.write(text.data(using: .utf8) ?? Data())
                fh.closeFile()
            } else {
                FileManager.default.createFile(atPath: path, contents: text.data(using: .utf8))
            }
        case .pipe(let cmdExpr):
            let cmd = (try? evalExpr(cmdExpr, locals: &locals))?.stringValue ?? ""
            let p = Process()
            p.executableURL = URL(fileURLWithPath: "/bin/sh")
            p.arguments = ["-c", cmd]
            let pipe = Pipe()
            p.standardInput = pipe
            try? p.run()
            pipe.fileHandleForWriting.write(text.data(using: .utf8) ?? Data())
            pipe.fileHandleForWriting.closeFile()
            p.waitUntilExit()
        }
    }

    // MARK: sprintf / printf formatting

    func sprintfFormat(_ fmt: String, args: [AWKValue]) -> String {
        var result = ""
        var i = fmt.startIndex
        var argIdx = 0

        while i < fmt.endIndex {
            let ch = fmt[i]
            if ch != "%" {
                result.append(ch)
                i = fmt.index(after: i)
                continue
            }
            i = fmt.index(after: i)
            if i >= fmt.endIndex { break }
            if fmt[i] == "%" { result.append("%"); i = fmt.index(after: i); continue }

            // Parse flags
            var flags = ""
            while i < fmt.endIndex && "-+ #0".contains(fmt[i]) {
                flags.append(fmt[i]); i = fmt.index(after: i)
            }
            // Width
            var width = ""
            while i < fmt.endIndex && fmt[i].isNumber { width.append(fmt[i]); i = fmt.index(after: i) }
            // Precision
            var precision = ""
            if i < fmt.endIndex && fmt[i] == "." {
                i = fmt.index(after: i)
                while i < fmt.endIndex && fmt[i].isNumber { precision.append(fmt[i]); i = fmt.index(after: i) }
            }

            guard i < fmt.endIndex else { break }
            let spec = fmt[i]; i = fmt.index(after: i)
            let arg = argIdx < args.count ? args[argIdx] : .uninitialized
            argIdx += 1

            let w = Int(width) ?? 0
            let p = Int(precision) ?? -1
            let leftAlign = flags.contains("-")

            func pad(_ s: String, _ width: Int) -> String {
                if s.count >= width { return s }
                let pad = String(repeating: leftAlign ? " " : (flags.contains("0") ? "0" : " "), count: width - s.count)
                return leftAlign ? s + pad : pad + s
            }

            switch spec {
            case "d", "i":
                let n = Int(arg.numberValue)
                var s = String(n)
                if flags.contains("+") && n >= 0 { s = "+" + s }
                result += pad(s, w)
            case "u":
                result += pad(String(UInt(max(0, Int(arg.numberValue)))), w)
            case "o":
                result += pad(String(Int(arg.numberValue), radix: 8), w)
            case "x":
                result += pad(String(Int(arg.numberValue), radix: 16), w)
            case "X":
                result += pad(String(Int(arg.numberValue), radix: 16).uppercased(), w)
            case "f":
                let prec = p >= 0 ? p : 6
                let s = String(format: "%.\(prec)f", arg.numberValue)
                result += pad(s, w)
            case "e":
                let prec = p >= 0 ? p : 6
                result += pad(String(format: "%.\(prec)e", arg.numberValue), w)
            case "E":
                let prec = p >= 0 ? p : 6
                result += pad(String(format: "%.\(prec)E", arg.numberValue), w)
            case "g", "G":
                let prec = p >= 0 ? p : 6
                let fmt2 = spec == "g" ? "%.\(prec)g" : "%.\(prec)G"
                result += pad(String(format: fmt2, arg.numberValue), w)
            case "s":
                var s = arg.stringValue
                if p >= 0 && s.count > p { s = String(s.prefix(p)) }
                result += pad(s, w)
            case "c":
                let s: String
                if case .string(let str) = arg, !str.isEmpty { s = String(str.prefix(1)) }
                else { s = String(UnicodeScalar(UInt32(arg.numberValue) ?? 0)!) }
                result += pad(s, w)
            default:
                result += "%"
                result.append(spec)
            }
        }
        return result
    }
}

// MARK: Help and version

func awk_print_usage() {
    print("""
    Usage: awk [OPTION]... 'program' [FILE]...
       or: awk [OPTION]... -f progfile [FILE]...
    Scan and process patterns in each FILE (or standard input).

    Options:
      -F fs       use fs as the input field separator (FS)
      -v var=val  assign value val to variable var before execution
      -f file     read program text from file
      --help      display this help and exit
      --version   output version information and exit

    A program consists of rules: /pattern/ { action }
    Special patterns: BEGIN { ... }  and  END { ... }

    Variables:   NR  NF  FS  OFS  ORS  RS  FILENAME
    Functions:   length  substr  index  split  sub  gsub  match
                 sprintf  printf  print  int  sqrt  log  exp
                 sin  cos  atan2  rand  srand  tolower  toupper  system
    """)
}

func awk_print_version() {
    print("""
    awk (cacutils) v1.0
    IEEE Std 1003.1-2008 (POSIX) compatible implementation.
    There is NO WARRANTY, to the extent permitted by law.
    Written by Cyril John Magayaga.
    """)
}
