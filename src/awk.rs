use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io::{self, BufRead, Write};
use std::process::{Command as ProcessCommand, Stdio};

// ─── Entry point ─────────────────────────────────────────────────────────────

pub fn awk_command(arguments: Vec<String>) {
    if arguments.contains(&"--help".to_string()) {
        awk_print_usage();
        return;
    }
    if arguments.contains(&"--version".to_string()) {
        awk_print_version();
        return;
    }
    if arguments.is_empty() {
        eprintln!("awk: no program given\nTry 'awk --help' for more information.");
        return;
    }

    let mut program_source: Option<String> = None;
    let mut program_file: Option<String> = None;
    let mut field_separator = " ".to_string();
    let mut assignments: Vec<(String, String)> = Vec::new();
    let mut input_files: Vec<String> = Vec::new();
    let mut i = 0;

    while i < arguments.len() {
        match arguments[i].as_str() {
            "-f" => {
                i += 1;
                if i >= arguments.len() {
                    eprintln!("awk: -f requires a file argument");
                    return;
                }
                program_file = Some(arguments[i].clone());
            }
            "-F" => {
                i += 1;
                if i >= arguments.len() {
                    eprintln!("awk: -F requires a separator argument");
                    return;
                }
                field_separator = unescape_fs(&arguments[i]);
            }
            "-v" => {
                i += 1;
                if i >= arguments.len() {
                    eprintln!("awk: -v requires a var=value argument");
                    return;
                }
                if let Some(eq) = arguments[i].find('=') {
                    assignments.push((
                        arguments[i][..eq].to_string(),
                        arguments[i][eq + 1..].to_string(),
                    ));
                } else {
                    eprintln!("awk: invalid -v assignment: {}", arguments[i]);
                    return;
                }
            }
            arg if arg.starts_with("-F") && arg.len() > 2 => {
                field_separator = unescape_fs(&arg[2..].to_string());
            }
            arg => {
                if program_source.is_none() && program_file.is_none() {
                    program_source = Some(arg.to_string());
                } else {
                    // var=value assignment or file
                    if arg.contains('=') && !arg.starts_with('-') {
                        if let Some(eq) = arg.find('=') {
                            assignments.push((arg[..eq].to_string(), arg[eq + 1..].to_string()));
                        }
                    } else {
                        input_files.push(arg.to_string());
                    }
                }
            }
        }
        i += 1;
    }

    let source = if let Some(f) = program_file {
        match fs::read_to_string(&f) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("awk: cannot open program file '{}': {}", f, e);
                return;
            }
        }
    } else if let Some(s) = program_source {
        s
    } else {
        eprintln!("awk: no program given");
        return;
    };

    let program = match AWKParser::new(&source).parse() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("awk: {}", e);
            return;
        }
    };

    let mut interp = AWKInterpreter::new(field_separator);
    for (k, v) in assignments {
        interp.globals.insert(k, AWKValue::Str(v));
    }

    if input_files.is_empty() {
        let stdin = io::stdin();
        let lines: Vec<String> = stdin.lock().lines().filter_map(|l| l.ok()).collect();
        interp.run(&program, lines.iter().map(|s| s.as_str()));
    } else {
        for path in &input_files {
            let content = match fs::read_to_string(path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("awk: cannot open '{}': {}", path, e);
                    continue;
                }
            };
            interp.filename = path.clone();
            let lines: Vec<&str> = content.lines().collect();
            interp.run(&program, lines.iter().copied());
        }
    }
}

fn unescape_fs(s: &str) -> String {
    s.replace("\\t", "\t").replace("\\n", "\n")
}

// ─── Error ───────────────────────────────────────────────────────────────────

#[derive(Debug)]
struct AWKError(String);

impl fmt::Display for AWKError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

macro_rules! awk_err {
    ($($arg:tt)*) => { AWKError(format!($($arg)*)) }
}

// ─── Value ───────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
enum AWKValue {
    Uninit,
    Num(f64),
    Str(String),
}

impl AWKValue {
    fn to_str(&self) -> String {
        match self {
            AWKValue::Uninit => String::new(),
            AWKValue::Str(s) => s.clone(),
            AWKValue::Num(n) => format_number(*n),
        }
    }

    fn to_num(&self) -> f64 {
        match self {
            AWKValue::Uninit => 0.0,
            AWKValue::Num(n) => *n,
            AWKValue::Str(s) => s.trim().parse::<f64>().unwrap_or(0.0),
        }
    }

    fn to_bool(&self) -> bool {
        match self {
            AWKValue::Uninit => false,
            AWKValue::Num(n) => *n != 0.0,
            AWKValue::Str(s) => !s.is_empty(),
        }
    }
}

fn format_number(n: f64) -> String {
    if n == n.trunc() && n.abs() < 1e15 && !n.is_infinite() {
        format!("{}", n as i64)
    } else {
        format!("{:.6}", n)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

fn compare_values(a: &AWKValue, b: &AWKValue) -> std::cmp::Ordering {
    match (a, b) {
        (AWKValue::Num(x), AWKValue::Num(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
        _ => {
            let xs = a.to_str();
            let ys = b.to_str();
            let xn = xs.trim().parse::<f64>();
            let yn = ys.trim().parse::<f64>();
            match (xn, yn) {
                (Ok(x), Ok(y)) => x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal),
                _ => xs.cmp(&ys),
            }
        }
    }
}

// ─── Lexer ───────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
enum TK {
    Num(f64),
    Str(String),
    Ident(String),
    Regex(String),
    // Arithmetic
    Plus, Minus, Star, Slash, Percent, Caret,
    PlusEq, MinusEq, StarEq, SlashEq, PercentEq, CaretEq,
    PlusPlus, MinusMinus,
    // Comparison
    Eq, EqEq, BangEq, Lt, LtEq, Gt, GtEq,
    // Logic
    And, Or, Bang,
    // Regex match
    Tilde, BangTilde,
    // Misc
    Comma, Semi, Colon, Question,
    LBrace, RBrace, LParen, RParen, LBrack, RBrack,
    Dollar, Newline, Pipe, Append, Eof,
    // Keywords
    BEGIN, END, If, Else, While, Do, For, In,
    Break, Continue, Next, Exit, Print, Printf,
    Delete, Return, Function, Getline,
}

#[derive(Clone, Debug)]
struct Token {
    kind: TK,
    line: usize,
}

struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
}

impl Lexer {
    fn new(src: &str) -> Self {
        Lexer { chars: src.chars().collect(), pos: 0, line: 1 }
    }

    fn peek(&self) -> Option<char> { self.chars.get(self.pos).copied() }
    fn peek2(&self) -> Option<char> { self.chars.get(self.pos + 1).copied() }

    fn advance(&mut self) -> Option<char> {
        let c = self.chars.get(self.pos).copied();
        if c == Some('\n') { self.line += 1; }
        self.pos += 1;
        c
    }

    fn skip_ws_comments(&mut self) {
        loop {
            match self.peek() {
                Some('#') => { while self.peek().map_or(false, |c| c != '\n') { self.advance(); } }
                Some(' ') | Some('\t') | Some('\r') => { self.advance(); }
                _ => break,
            }
        }
    }

    fn next_token(&mut self) -> Result<Token, AWKError> {
        self.skip_ws_comments();
        let ln = self.line;
        let c = match self.peek() {
            None => return Ok(Token { kind: TK::Eof, line: ln }),
            Some(c) => c,
        };

        if c == '\n' { self.advance(); return Ok(Token { kind: TK::Newline, line: ln }); }

        // String literal
        if c == '"' {
            self.advance();
            let mut s = String::new();
            loop {
                match self.peek() {
                    None | Some('\n') => return Err(awk_err!("unterminated string at line {}", ln)),
                    Some('"') => { self.advance(); break; }
                    Some('\\') => {
                        self.advance();
                        match self.advance() {
                            Some('n') => s.push('\n'), Some('t') => s.push('\t'),
                            Some('r') => s.push('\r'), Some('\\') => s.push('\\'),
                            Some('"') => s.push('"'), Some('/') => s.push('/'),
                            Some(ch) => { s.push('\\'); s.push(ch); }
                            None => {}
                        }
                    }
                    Some(ch) => { self.advance(); s.push(ch); }
                }
            }
            return Ok(Token { kind: TK::Str(s), line: ln });
        }

        // Number
        if c.is_ascii_digit() || (c == '.' && self.peek2().map_or(false, |c2| c2.is_ascii_digit())) {
            let mut buf = String::new();
            while self.peek().map_or(false, |c| c.is_ascii_digit()) { buf.push(self.advance().unwrap()); }
            if self.peek() == Some('.') { buf.push(self.advance().unwrap());
                while self.peek().map_or(false, |c| c.is_ascii_digit()) { buf.push(self.advance().unwrap()); }
            }
            if matches!(self.peek(), Some('e') | Some('E')) {
                buf.push(self.advance().unwrap());
                if matches!(self.peek(), Some('+') | Some('-')) { buf.push(self.advance().unwrap()); }
                while self.peek().map_or(false, |c| c.is_ascii_digit()) { buf.push(self.advance().unwrap()); }
            }
            return Ok(Token { kind: TK::Num(buf.parse().unwrap_or(0.0)), line: ln });
        }

        // Identifier / keyword
        if c.is_alphabetic() || c == '_' {
            let mut buf = String::new();
            while self.peek().map_or(false, |c| c.is_alphanumeric() || c == '_') {
                buf.push(self.advance().unwrap());
            }
            let kind = match buf.as_str() {
                "BEGIN" => TK::BEGIN, "END" => TK::END,
                "if" => TK::If, "else" => TK::Else,
                "while" => TK::While, "do" => TK::Do, "for" => TK::For, "in" => TK::In,
                "break" => TK::Break, "continue" => TK::Continue,
                "next" => TK::Next, "exit" => TK::Exit,
                "print" => TK::Print, "printf" => TK::Printf,
                "delete" => TK::Delete, "return" => TK::Return,
                "function" => TK::Function, "getline" => TK::Getline,
                _ => TK::Ident(buf),
            };
            return Ok(Token { kind, line: ln });
        }

        self.advance();
        let kind = match c {
            '+' => match self.peek() { Some('+') => { self.advance(); TK::PlusPlus } Some('=') => { self.advance(); TK::PlusEq } _ => TK::Plus },
            '-' => match self.peek() { Some('-') => { self.advance(); TK::MinusMinus } Some('=') => { self.advance(); TK::MinusEq } _ => TK::Minus },
            '*' => if self.peek() == Some('=') { self.advance(); TK::StarEq } else { TK::Star },
            '/' => if self.peek() == Some('=') { self.advance(); TK::SlashEq } else { TK::Slash },
            '%' => if self.peek() == Some('=') { self.advance(); TK::PercentEq } else { TK::Percent },
            '^' => if self.peek() == Some('=') { self.advance(); TK::CaretEq } else { TK::Caret },
            '=' => if self.peek() == Some('=') { self.advance(); TK::EqEq } else { TK::Eq },
            '!' => match self.peek() { Some('=') => { self.advance(); TK::BangEq } Some('~') => { self.advance(); TK::BangTilde } _ => TK::Bang },
            '<' => if self.peek() == Some('=') { self.advance(); TK::LtEq } else { TK::Lt },
            '>' => match self.peek() { Some('>') => { self.advance(); TK::Append } Some('=') => { self.advance(); TK::GtEq } _ => TK::Gt },
            '&' => if self.peek() == Some('&') { self.advance(); TK::And } else { return Err(awk_err!("unexpected '&' at line {}", ln)); },
            '|' => if self.peek() == Some('|') { self.advance(); TK::Or } else { TK::Pipe },
            '~' => TK::Tilde,
            ',' => TK::Comma, ';' => TK::Semi, ':' => TK::Colon, '?' => TK::Question,
            '{' => TK::LBrace, '}' => TK::RBrace,
            '(' => TK::LParen, ')' => TK::RParen,
            '[' => TK::LBrack, ']' => TK::RBrack,
            '$' => TK::Dollar,
            '\\' if self.peek() == Some('\n') => { self.advance(); return self.next_token(); }
            ch => return Err(awk_err!("unexpected character '{}' at line {}", ch, ln)),
        };
        Ok(Token { kind, line: ln })
    }

    fn lex_regex(&mut self) -> Result<Token, AWKError> {
        let ln = self.line;
        let mut buf = String::new();
        loop {
            match self.peek() {
                None | Some('\n') => return Err(awk_err!("unterminated regex at line {}", ln)),
                Some('/') => { self.advance(); break; }
                Some('\\') => {
                    self.advance();
                    if let Some(c) = self.advance() { buf.push('\\'); buf.push(c); }
                }
                Some(c) => { self.advance(); buf.push(c); }
            }
        }
        Ok(Token { kind: TK::Regex(buf), line: ln })
    }
}

// ─── AST ─────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
enum Expr {
    Num(f64),
    Str(String),
    Regex(String),
    Field(Box<Expr>),
    Var(String),
    Array(String, Vec<Expr>),
    Assign(Box<Expr>, Box<Expr>),
    CompoundAssign(String, Box<Expr>, Box<Expr>),
    PreIncDec(String, Box<Expr>),
    PostIncDec(Box<Expr>, String),
    Unary(String, Box<Expr>),
    Binary(String, Box<Expr>, Box<Expr>),
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),
    Match(Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
    Getline(Option<Box<Expr>>),
    InArray(Box<Expr>, String),
    Concat(Vec<Expr>),
}

#[derive(Clone, Debug)]
enum Redirect {
    File(Expr),
    Append(Expr),
    Pipe(Expr),
}

#[derive(Clone, Debug)]
enum Stmt {
    Block(Vec<Stmt>),
    Expr(Expr),
    Print(Vec<Expr>, Option<Redirect>),
    Printf(Vec<Expr>, Option<Redirect>),
    If(Expr, Box<Stmt>, Option<Box<Stmt>>),
    While(Expr, Box<Stmt>),
    DoWhile(Box<Stmt>, Expr),
    For(Option<Box<Stmt>>, Option<Expr>, Option<Box<Stmt>>, Box<Stmt>),
    ForIn(String, String, Box<Stmt>),
    Break, Continue, Next,
    Exit(Option<Expr>),
    Return(Option<Expr>),
    Delete(String, Option<Vec<Expr>>),
    Empty,
}

#[derive(Clone, Debug)]
enum Pattern {
    Begin, End,
    Expr(Expr),
    Range(Expr, Expr),
    Always,
}

#[derive(Clone, Debug)]
struct Rule {
    pattern: Pattern,
    action: Stmt,
}

#[derive(Clone, Debug)]
struct Function {
    params: Vec<String>,
    body: Stmt,
}

#[derive(Clone, Debug)]
struct Program {
    rules: Vec<Rule>,
    functions: HashMap<String, Function>,
}

// ─── Parser ───────────────────────────────────────────────────────────────────

struct AWKParser {
    lexer: Lexer,
    current: Token,
    peeked: Option<Token>,
}

impl AWKParser {
    fn new(src: &str) -> Self {
        let mut lexer = Lexer::new(src);
        let current = lexer.next_token().unwrap_or(Token { kind: TK::Eof, line: 1 });
        AWKParser { lexer, current, peeked: None }
    }

    fn peek(&mut self) -> Result<&Token, AWKError> {
        if self.peeked.is_none() {
            self.peeked = Some(self.lexer.next_token()?);
        }
        Ok(self.peeked.as_ref().unwrap())
    }

    fn advance(&mut self) -> Result<Token, AWKError> {
        let t = if let Some(p) = self.peeked.take() {
            std::mem::replace(&mut self.current, p)
        } else {
            let next = self.lexer.next_token()?;
            std::mem::replace(&mut self.current, next)
        };
        Ok(t)
    }

    fn skip_nls(&mut self) -> Result<(), AWKError> {
        while self.current.kind == TK::Newline { self.advance()?; }
        Ok(())
    }

    fn skip_terminators(&mut self) -> Result<(), AWKError> {
        loop {
            match self.current.kind {
                TK::Newline | TK::Semi => { self.advance()?; }
                _ => break,
            }
        }
        Ok(())
    }

    fn expect(&mut self, kind: TK) -> Result<(), AWKError> {
        if self.current.kind == kind {
            self.advance()?;
            Ok(())
        } else {
            Err(awk_err!("expected {:?} at line {}, got {:?}", kind, self.current.line, self.current.kind))
        }
    }

    fn parse(mut self) -> Result<Program, AWKError> {
        let mut rules = Vec::new();
        let mut functions = HashMap::new();
        self.skip_terminators()?;
        while self.current.kind != TK::Eof {
            if self.current.kind == TK::Function {
                self.advance()?;
                let name = match self.current.kind.clone() {
                    TK::Ident(n) => { self.advance()?; n }
                    _ => return Err(awk_err!("expected function name at line {}", self.current.line)),
                };
                self.expect(TK::LParen)?;
                let mut params = Vec::new();
                while let TK::Ident(p) = self.current.kind.clone() {
                    params.push(p); self.advance()?;
                    if self.current.kind == TK::Comma { self.advance()?; }
                }
                self.expect(TK::RParen)?;
                self.skip_nls()?;
                let body = self.parse_block()?;
                functions.insert(name, Function { params, body });
            } else {
                rules.push(self.parse_rule()?);
            }
            self.skip_terminators()?;
        }
        Ok(Program { rules, functions })
    }

    fn parse_rule(&mut self) -> Result<Rule, AWKError> {
        let pattern = match self.current.kind.clone() {
            TK::BEGIN => { self.advance()?; Pattern::Begin }
            TK::END   => { self.advance()?; Pattern::End }
            TK::LBrace => Pattern::Always,
            _ => {
                let e1 = self.parse_expr()?;
                if self.current.kind == TK::Comma {
                    self.advance()?; self.skip_nls()?;
                    Pattern::Range(e1, self.parse_expr()?)
                } else {
                    Pattern::Expr(e1)
                }
            }
        };
        self.skip_nls()?;
        let action = if self.current.kind == TK::LBrace {
            self.parse_block()?
        } else {
            Stmt::Print(vec![], None)
        };
        Ok(Rule { pattern, action })
    }

    fn parse_block(&mut self) -> Result<Stmt, AWKError> {
        self.expect(TK::LBrace)?;
        self.skip_terminators()?;
        let mut stmts = Vec::new();
        while self.current.kind != TK::RBrace {
            if self.current.kind == TK::Eof {
                return Err(awk_err!("unclosed block"));
            }
            stmts.push(self.parse_stmt()?);
            self.skip_terminators()?;
        }
        self.advance()?;
        Ok(Stmt::Block(stmts))
    }

    fn parse_stmt(&mut self) -> Result<Stmt, AWKError> {
        let ln = self.current.line;
        match self.current.kind.clone() {
            TK::If => {
                self.advance()?;
                self.expect(TK::LParen)?;
                let cond = self.parse_expr()?;
                self.expect(TK::RParen)?;
                self.skip_nls()?;
                let then = self.parse_stmt()?;
                self.skip_terminators()?;
                let els = if self.current.kind == TK::Else {
                    self.advance()?; self.skip_nls()?;
                    Some(Box::new(self.parse_stmt()?))
                } else { None };
                Ok(Stmt::If(cond, Box::new(then), els))
            }
            TK::While => {
                self.advance()?;
                self.expect(TK::LParen)?;
                let cond = self.parse_expr()?;
                self.expect(TK::RParen)?;
                self.skip_nls()?;
                Ok(Stmt::While(cond, Box::new(self.parse_stmt()?)))
            }
            TK::Do => {
                self.advance()?; self.skip_nls()?;
                let body = self.parse_stmt()?;
                self.skip_terminators()?;
                self.advance()?; // while
                self.expect(TK::LParen)?;
                let cond = self.parse_expr()?;
                self.expect(TK::RParen)?;
                Ok(Stmt::DoWhile(Box::new(body), cond))
            }
            TK::For => {
                self.advance()?;
                self.expect(TK::LParen)?;
                // Detect for-in
                if let TK::Ident(var) = self.current.kind.clone() {
                    let saved_pos = self.lexer.pos;
                    let saved_line = self.lexer.line;
                    self.advance()?;
                    if self.current.kind == TK::In {
                        self.advance()?;
                        if let TK::Ident(arr) = self.current.kind.clone() {
                            self.advance()?;
                            self.expect(TK::RParen)?;
                            self.skip_nls()?;
                            let body = self.parse_stmt()?;
                            return Ok(Stmt::ForIn(var, arr, Box::new(body)));
                        }
                    }
                    // Not for-in — restore and re-parse as expression starting with var
                    // We can't truly backtrack, so construct a var expr and parse continuation
                    let _ = saved_pos; let _ = saved_line; // consumed already
                    let init_expr = self.finish_expr_from_var(var)?;
                    self.expect(TK::Semi)?;
                    let cond = if self.current.kind != TK::Semi { Some(self.parse_expr()?) } else { None };
                    self.expect(TK::Semi)?;
                    let post = if self.current.kind != TK::RParen { Some(Box::new(Stmt::Expr(self.parse_expr()?))) } else { None };
                    self.expect(TK::RParen)?;
                    self.skip_nls()?;
                    let body = self.parse_stmt()?;
                    return Ok(Stmt::For(Some(Box::new(Stmt::Expr(init_expr))), cond, post, Box::new(body)));
                }
                let init = if self.current.kind != TK::Semi { Some(Box::new(Stmt::Expr(self.parse_expr()?))) } else { None };
                self.expect(TK::Semi)?;
                let cond = if self.current.kind != TK::Semi { Some(self.parse_expr()?) } else { None };
                self.expect(TK::Semi)?;
                let post = if self.current.kind != TK::RParen { Some(Box::new(Stmt::Expr(self.parse_expr()?))) } else { None };
                self.expect(TK::RParen)?;
                self.skip_nls()?;
                let body = self.parse_stmt()?;
                Ok(Stmt::For(init, cond, post, Box::new(body)))
            }
            TK::Break    => { self.advance()?; Ok(Stmt::Break) }
            TK::Continue => { self.advance()?; Ok(Stmt::Continue) }
            TK::Next     => { self.advance()?; Ok(Stmt::Next) }
            TK::Exit => {
                self.advance()?;
                let e = if self.is_stmt_end() { None } else { Some(self.parse_expr()?) };
                Ok(Stmt::Exit(e))
            }
            TK::Return => {
                self.advance()?;
                let e = if self.is_stmt_end() { None } else { Some(self.parse_expr()?) };
                Ok(Stmt::Return(e))
            }
            TK::Delete => {
                self.advance()?;
                if let TK::Ident(name) = self.current.kind.clone() {
                    self.advance()?;
                    if self.current.kind == TK::LBrack {
                        self.advance()?;
                        let mut keys = vec![self.parse_expr()?];
                        while self.current.kind == TK::Comma { self.advance()?; keys.push(self.parse_expr()?); }
                        self.expect(TK::RBrack)?;
                        return Ok(Stmt::Delete(name, Some(keys)));
                    }
                    return Ok(Stmt::Delete(name, None));
                }
                Err(awk_err!("expected array name after delete at line {}", ln))
            }
            TK::Print | TK::Printf => {
                let is_printf = self.current.kind == TK::Printf;
                self.advance()?;
                let has_paren = self.current.kind == TK::LParen;
                if has_paren { self.advance()?; }
                let mut args = Vec::new();
                if !self.is_stmt_end() && self.current.kind != TK::RParen {
                    args.push(self.parse_expr()?);
                    while self.current.kind == TK::Comma { self.advance()?; args.push(self.parse_expr()?); }
                }
                if has_paren && self.current.kind == TK::RParen { self.advance()?; }
                let redirect = self.parse_redirect()?;
                if is_printf { Ok(Stmt::Printf(args, redirect)) } else { Ok(Stmt::Print(args, redirect)) }
            }
            TK::LBrace => self.parse_block(),
            _ => Ok(Stmt::Expr(self.parse_expr()?)),
        }
    }

    // After consuming an identifier in a for-init, finish parsing the expression
    fn finish_expr_from_var(&mut self, var: String) -> Result<Expr, AWKError> {
        let lhs = Expr::Var(var);
        match self.current.kind.clone() {
            TK::Eq => { self.advance()?; Ok(Expr::Assign(Box::new(lhs), Box::new(self.parse_expr()?))) }
            TK::PlusEq  => { self.advance()?; Ok(Expr::CompoundAssign("+".into(),  Box::new(lhs), Box::new(self.parse_expr()?))) }
            TK::MinusEq => { self.advance()?; Ok(Expr::CompoundAssign("-".into(),  Box::new(lhs), Box::new(self.parse_expr()?))) }
            TK::StarEq  => { self.advance()?; Ok(Expr::CompoundAssign("*".into(),  Box::new(lhs), Box::new(self.parse_expr()?))) }
            TK::SlashEq => { self.advance()?; Ok(Expr::CompoundAssign("/".into(),  Box::new(lhs), Box::new(self.parse_expr()?))) }
            TK::PercentEq => { self.advance()?; Ok(Expr::CompoundAssign("%".into(), Box::new(lhs), Box::new(self.parse_expr()?))) }
            TK::PlusPlus  => { self.advance()?; Ok(Expr::PostIncDec(Box::new(lhs), "++".into())) }
            TK::MinusMinus => { self.advance()?; Ok(Expr::PostIncDec(Box::new(lhs), "--".into())) }
            _ => Ok(lhs),
        }
    }

    fn is_stmt_end(&self) -> bool {
        matches!(self.current.kind, TK::Newline | TK::Semi | TK::RBrace | TK::Eof)
    }

    fn parse_redirect(&mut self) -> Result<Option<Redirect>, AWKError> {
        match self.current.kind.clone() {
            TK::Gt     => { self.advance()?; Ok(Some(Redirect::File(self.parse_primary()?))) }
            TK::Append => { self.advance()?; Ok(Some(Redirect::Append(self.parse_primary()?))) }
            TK::Pipe   => { self.advance()?; Ok(Some(Redirect::Pipe(self.parse_primary()?))) }
            _ => Ok(None),
        }
    }

    // Expression parsing — precedence climbing
    fn parse_expr(&mut self)    -> Result<Expr, AWKError> { self.parse_ternary() }

    fn parse_ternary(&mut self) -> Result<Expr, AWKError> {
        let lhs = self.parse_or()?;
        if self.current.kind == TK::Question {
            self.advance()?;
            let t = self.parse_ternary()?;
            self.expect(TK::Colon)?;
            let f = self.parse_ternary()?;
            return Ok(Expr::Ternary(Box::new(lhs), Box::new(t), Box::new(f)));
        }
        match self.current.kind.clone() {
            TK::Eq      => { self.advance()?; Ok(Expr::Assign(Box::new(lhs), Box::new(self.parse_ternary()?))) }
            TK::PlusEq  => { self.advance()?; Ok(Expr::CompoundAssign("+".into(), Box::new(lhs), Box::new(self.parse_ternary()?))) }
            TK::MinusEq => { self.advance()?; Ok(Expr::CompoundAssign("-".into(), Box::new(lhs), Box::new(self.parse_ternary()?))) }
            TK::StarEq  => { self.advance()?; Ok(Expr::CompoundAssign("*".into(), Box::new(lhs), Box::new(self.parse_ternary()?))) }
            TK::SlashEq => { self.advance()?; Ok(Expr::CompoundAssign("/".into(), Box::new(lhs), Box::new(self.parse_ternary()?))) }
            TK::PercentEq => { self.advance()?; Ok(Expr::CompoundAssign("%".into(), Box::new(lhs), Box::new(self.parse_ternary()?))) }
            TK::CaretEq => { self.advance()?; Ok(Expr::CompoundAssign("^".into(), Box::new(lhs), Box::new(self.parse_ternary()?))) }
            _ => Ok(lhs),
        }
    }

    fn parse_or(&mut self) -> Result<Expr, AWKError> {
        let mut lhs = self.parse_and()?;
        while self.current.kind == TK::Or { self.advance()?; lhs = Expr::Binary("||".into(), Box::new(lhs), Box::new(self.parse_and()?)); }
        Ok(lhs)
    }

    fn parse_and(&mut self) -> Result<Expr, AWKError> {
        let mut lhs = self.parse_match_expr()?;
        while self.current.kind == TK::And { self.advance()?; lhs = Expr::Binary("&&".into(), Box::new(lhs), Box::new(self.parse_match_expr()?)); }
        Ok(lhs)
    }

    fn parse_match_expr(&mut self) -> Result<Expr, AWKError> {
        let mut lhs = self.parse_in()?;
        loop {
            match self.current.kind.clone() {
                TK::Tilde     => { self.advance()?; lhs = Expr::Match(Box::new(lhs), Box::new(self.parse_in()?)); }
                TK::BangTilde => { self.advance()?; lhs = Expr::Unary("!".into(), Box::new(Expr::Match(Box::new(lhs), Box::new(self.parse_in()?)))); }
                _ => break,
            }
        }
        Ok(lhs)
    }

    fn parse_in(&mut self) -> Result<Expr, AWKError> {
        let mut lhs = self.parse_cmp()?;
        while self.current.kind == TK::In {
            self.advance()?;
            if let TK::Ident(name) = self.current.kind.clone() {
                self.advance()?;
                lhs = Expr::InArray(Box::new(lhs), name);
            } else {
                return Err(awk_err!("expected array name after 'in' at line {}", self.current.line));
            }
        }
        Ok(lhs)
    }

    fn parse_cmp(&mut self) -> Result<Expr, AWKError> {
        let mut lhs = self.parse_concat()?;
        loop {
            let op = match self.current.kind {
                TK::Lt => "<", TK::LtEq => "<=", TK::Gt => ">",
                TK::GtEq => ">=", TK::EqEq => "==", TK::BangEq => "!=",
                _ => break,
            }.to_string();
            self.advance()?;
            lhs = Expr::Binary(op, Box::new(lhs), Box::new(self.parse_concat()?));
        }
        Ok(lhs)
    }

    fn parse_concat(&mut self) -> Result<Expr, AWKError> {
        let mut parts = vec![self.parse_add()?];
        loop {
            // Concatenation: stop at anything that can't start a primary
            if self.is_stmt_end() || matches!(self.current.kind,
                TK::Comma | TK::RParen | TK::RBrack | TK::Pipe | TK::Gt | TK::Append |
                TK::Question | TK::Colon | TK::Eq | TK::PlusEq | TK::MinusEq | TK::StarEq |
                TK::SlashEq | TK::PercentEq | TK::CaretEq | TK::And | TK::Or |
                TK::Tilde | TK::BangTilde | TK::In | TK::Lt | TK::LtEq |
                TK::GtEq | TK::EqEq | TK::BangEq | TK::Eof
            ) { break; }
            parts.push(self.parse_add()?);
        }
        if parts.len() == 1 { Ok(parts.remove(0)) } else { Ok(Expr::Concat(parts)) }
    }

    fn parse_add(&mut self) -> Result<Expr, AWKError> {
        let mut lhs = self.parse_mul()?;
        loop {
            match self.current.kind {
                TK::Plus  => { self.advance()?; lhs = Expr::Binary("+".into(), Box::new(lhs), Box::new(self.parse_mul()?)); }
                TK::Minus => { self.advance()?; lhs = Expr::Binary("-".into(), Box::new(lhs), Box::new(self.parse_mul()?)); }
                _ => break,
            }
        }
        Ok(lhs)
    }

    fn parse_mul(&mut self) -> Result<Expr, AWKError> {
        let mut lhs = self.parse_pow()?;
        loop {
            match self.current.kind {
                TK::Star    => { self.advance()?; lhs = Expr::Binary("*".into(), Box::new(lhs), Box::new(self.parse_pow()?)); }
                TK::Slash   => { self.advance()?; lhs = Expr::Binary("/".into(), Box::new(lhs), Box::new(self.parse_pow()?)); }
                TK::Percent => { self.advance()?; lhs = Expr::Binary("%".into(), Box::new(lhs), Box::new(self.parse_pow()?)); }
                _ => break,
            }
        }
        Ok(lhs)
    }

    fn parse_pow(&mut self) -> Result<Expr, AWKError> {
        let lhs = self.parse_unary()?;
        if self.current.kind == TK::Caret { self.advance()?; return Ok(Expr::Binary("^".into(), Box::new(lhs), Box::new(self.parse_pow()?))); }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> Result<Expr, AWKError> {
        match self.current.kind.clone() {
            TK::Bang      => { self.advance()?; Ok(Expr::Unary("!".into(),  Box::new(self.parse_unary()?))) }
            TK::Minus     => { self.advance()?; Ok(Expr::Unary("-".into(),  Box::new(self.parse_unary()?))) }
            TK::Plus      => { self.advance()?; Ok(Expr::Unary("+".into(),  Box::new(self.parse_unary()?))) }
            TK::PlusPlus  => { self.advance()?; Ok(Expr::PreIncDec("++".into(), Box::new(self.parse_unary()?))) }
            TK::MinusMinus => { self.advance()?; Ok(Expr::PreIncDec("--".into(), Box::new(self.parse_unary()?))) }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Result<Expr, AWKError> {
        let e = self.parse_primary()?;
        match self.current.kind {
            TK::PlusPlus   => { self.advance()?; Ok(Expr::PostIncDec(Box::new(e), "++".into())) }
            TK::MinusMinus => { self.advance()?; Ok(Expr::PostIncDec(Box::new(e), "--".into())) }
            _ => Ok(e),
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, AWKError> {
        let ln = self.current.line;
        match self.current.kind.clone() {
            TK::Num(n) => { self.advance()?; Ok(Expr::Num(n)) }
            TK::Str(s) => { self.advance()?; Ok(Expr::Str(s)) }
            TK::Dollar => { self.advance()?; Ok(Expr::Field(Box::new(self.parse_unary()?))) }
            TK::LParen => {
                self.advance()?;
                let e = self.parse_expr()?;
                self.expect(TK::RParen)?;
                Ok(e)
            }
            TK::Bang => { self.advance()?; Ok(Expr::Unary("!".into(), Box::new(self.parse_primary()?))) }
            TK::Slash => {
                // Regex literal
                self.advance()?; // consume the already-lexed slash by re-lexing
                let tok = self.lexer.lex_regex()?;
                self.current = self.lexer.next_token()?;
                if let TK::Regex(p) = tok.kind { Ok(Expr::Regex(p)) }
                else { Err(awk_err!("expected regex at line {}", ln)) }
            }
            TK::Getline => {
                self.advance()?;
                if let TK::Ident(v) = self.current.kind.clone() {
                    self.advance()?;
                    Ok(Expr::Getline(Some(Box::new(Expr::Var(v)))))
                } else {
                    Ok(Expr::Getline(None))
                }
            }
            TK::Ident(name) => {
                self.advance()?;
                if self.current.kind == TK::LBrack {
                    self.advance()?;
                    let mut keys = vec![self.parse_expr()?];
                    while self.current.kind == TK::Comma { self.advance()?; keys.push(self.parse_expr()?); }
                    self.expect(TK::RBrack)?;
                    return Ok(Expr::Array(name, keys));
                }
                if self.current.kind == TK::LParen {
                    self.advance()?;
                    let mut args = Vec::new();
                    if self.current.kind != TK::RParen {
                        args.push(self.parse_expr()?);
                        while self.current.kind == TK::Comma { self.advance()?; args.push(self.parse_expr()?); }
                    }
                    self.expect(TK::RParen)?;
                    return Ok(Expr::Call(name, args));
                }
                Ok(Expr::Var(name))
            }
            _ => Err(awk_err!("unexpected token {:?} at line {}", self.current.kind, ln)),
        }
    }
}

// ─── Control flow ────────────────────────────────────────────────────────────

enum Signal {
    Break, Continue, Next,
    Exit(AWKValue),
    Return(AWKValue),
}

// ─── Interpreter ─────────────────────────────────────────────────────────────

struct AWKInterpreter {
    globals: HashMap<String, AWKValue>,
    arrays: HashMap<String, HashMap<String, AWKValue>>,
    fields: Vec<String>,
    record: String,
    field_sep: String,
    ofs: String,
    ors: String,
    nr: usize,
    nf: usize,
    pub filename: String,
    range_active: HashMap<usize, bool>,
}

impl AWKInterpreter {
    fn new(field_sep: String) -> Self {
        AWKInterpreter {
            globals: HashMap::new(), arrays: HashMap::new(),
            fields: Vec::new(), record: String::new(),
            field_sep, ofs: " ".into(), ors: "\n".into(),
            nr: 0, nf: 0, filename: String::new(),
            range_active: HashMap::new(),
        }
    }

    fn split_record(&mut self, line: &str) {
        self.record = line.to_string();
        self.fields = if self.field_sep == " " {
            line.split_whitespace().map(|s| s.to_string()).collect()
        } else if self.field_sep.len() == 1 {
            line.split(self.field_sep.chars().next().unwrap()).map(|s| s.to_string()).collect()
        } else {
            simple_split(line, &self.field_sep)
        };
        self.nf = self.fields.len();
    }

    fn get_field(&self, idx: usize) -> AWKValue {
        if idx == 0 { return AWKValue::Str(self.record.clone()); }
        if idx < 1 || idx > self.fields.len() { return AWKValue::Uninit; }
        AWKValue::Str(self.fields[idx - 1].clone())
    }

    fn set_field(&mut self, idx: usize, val: AWKValue) {
        if idx == 0 { self.record = val.to_str(); self.split_record(&self.record.clone()); return; }
        while self.fields.len() < idx { self.fields.push(String::new()); }
        self.fields[idx - 1] = val.to_str();
        self.nf = self.fields.len();
        self.record = self.fields.join(&self.ofs.clone());
    }

    fn get_var(&self, name: &str, locals: &HashMap<String, AWKValue>) -> AWKValue {
        match name {
            "NR" => AWKValue::Num(self.nr as f64),
            "NF" => AWKValue::Num(self.nf as f64),
            "FS" => AWKValue::Str(self.field_sep.clone()),
            "OFS" => AWKValue::Str(self.ofs.clone()),
            "ORS" => AWKValue::Str(self.ors.clone()),
            "FILENAME" => AWKValue::Str(self.filename.clone()),
            _ => locals.get(name).or_else(|| self.globals.get(name)).cloned().unwrap_or(AWKValue::Uninit),
        }
    }

    fn set_var(&mut self, name: &str, val: AWKValue, locals: &mut HashMap<String, AWKValue>) {
        match name {
            "FS" => self.field_sep = val.to_str(),
            "OFS" => self.ofs = val.to_str(),
            "ORS" => self.ors = val.to_str(),
            _ => {
                if locals.contains_key(name) { locals.insert(name.to_string(), val); }
                else { self.globals.insert(name.to_string(), val); }
            }
        }
    }

    fn run<'a, I: Iterator<Item = &'a str>>(&mut self, prog: &Program, input: I) {
        let mut locals = HashMap::new();

        // BEGIN
        for rule in &prog.rules {
            if matches!(rule.pattern, Pattern::Begin) {
                match self.exec_stmt(&rule.action, &mut locals, prog) {
                    Err(Signal::Exit(_)) => return,
                    _ => {}
                }
            }
        }

        // Records
        for line in input {
            self.nr += 1;
            self.split_record(line);
            'rules: for (i, rule) in prog.rules.iter().enumerate() {
                let matched = match &rule.pattern {
                    Pattern::Begin | Pattern::End => continue,
                    Pattern::Always => true,
                    Pattern::Expr(e) => {
                        match self.eval_expr(e, &mut locals, prog) {
                            Ok(v) => v.to_bool(),
                            Err(_) => false,
                        }
                    }
                    Pattern::Range(start, end) => {
                        let active = *self.range_active.get(&i).unwrap_or(&false);
                        if !active {
                            let v = self.eval_expr(start, &mut locals, prog).unwrap_or(AWKValue::Uninit);
                            if v.to_bool() {
                                self.range_active.insert(i, true);
                                true
                            } else { false }
                        } else {
                            let v = self.eval_expr(end, &mut locals, prog).unwrap_or(AWKValue::Uninit);
                            if v.to_bool() { self.range_active.insert(i, false); }
                            true
                        }
                    }
                };
                if matched {
                    match self.exec_stmt(&rule.action, &mut locals, prog) {
                        Err(Signal::Next) => break 'rules,
                        Err(Signal::Exit(_)) => return,
                        _ => {}
                    }
                }
            }
        }

        // END
        for rule in &prog.rules {
            if matches!(rule.pattern, Pattern::End) {
                match self.exec_stmt(&rule.action, &mut locals, prog) {
                    Err(Signal::Exit(_)) => return,
                    _ => {}
                }
            }
        }
    }

    fn exec_stmt(&mut self, stmt: &Stmt, locals: &mut HashMap<String, AWKValue>, prog: &Program) -> Result<(), Signal> {
        match stmt {
            Stmt::Empty => {}
            Stmt::Block(stmts) => {
                for s in stmts { self.exec_stmt(s, locals, prog)?; }
            }
            Stmt::Expr(e) => { self.eval_expr(e, locals, prog).ok(); }
            Stmt::Print(args, redirect) => {
                let out = if args.is_empty() {
                    self.record.clone()
                } else {
                    let ofs = self.ofs.clone();
                    let parts: Vec<String> = args.iter()
                        .map(|e| self.eval_expr(e, locals, prog).unwrap_or(AWKValue::Uninit).to_str())
                        .collect();
                    parts.join(&ofs)
                };
                let ors = self.ors.clone();
                let text = format!("{}{}", out, ors);
                self.write_output(&text, redirect.as_ref(), locals, prog);
            }
            Stmt::Printf(args, redirect) => {
                if args.is_empty() { return Ok(()); }
                let fmt = self.eval_expr(&args[0], locals, prog).unwrap_or(AWKValue::Uninit).to_str();
                let rest: Vec<AWKValue> = args[1..].iter()
                    .map(|e| self.eval_expr(e, locals, prog).unwrap_or(AWKValue::Uninit))
                    .collect();
                let text = sprintf_format(&fmt, &rest);
                self.write_output(&text, redirect.as_ref(), locals, prog);
            }
            Stmt::If(cond, then, els) => {
                let v = self.eval_expr(cond, locals, prog).unwrap_or(AWKValue::Uninit);
                if v.to_bool() { self.exec_stmt(then, locals, prog)?; }
                else if let Some(e) = els { self.exec_stmt(e, locals, prog)?; }
            }
            Stmt::While(cond, body) => {
                loop {
                    let v = self.eval_expr(cond, locals, prog).unwrap_or(AWKValue::Uninit);
                    if !v.to_bool() { break; }
                    match self.exec_stmt(body, locals, prog) {
                        Err(Signal::Break) => break,
                        Err(Signal::Continue) => continue,
                        Err(e) => return Err(e),
                        Ok(_) => {}
                    }
                }
            }
            Stmt::DoWhile(body, cond) => {
                loop {
                    match self.exec_stmt(body, locals, prog) {
                        Err(Signal::Break) => break,
                        Err(Signal::Continue) => {}
                        Err(e) => return Err(e),
                        Ok(_) => {}
                    }
                    let v = self.eval_expr(cond, locals, prog).unwrap_or(AWKValue::Uninit);
                    if !v.to_bool() { break; }
                }
            }
            Stmt::For(init, cond, post, body) => {
                if let Some(i) = init { self.exec_stmt(i, locals, prog)?; }
                loop {
                    if let Some(c) = cond {
                        let v = self.eval_expr(c, locals, prog).unwrap_or(AWKValue::Uninit);
                        if !v.to_bool() { break; }
                    }
                    match self.exec_stmt(body, locals, prog) {
                        Err(Signal::Break) => break,
                        Err(Signal::Continue) => {}
                        Err(e) => return Err(e),
                        Ok(_) => {}
                    }
                    if let Some(p) = post { self.exec_stmt(p, locals, prog)?; }
                }
            }
            Stmt::ForIn(var, arr_name, body) => {
                let keys: Vec<String> = self.arrays.get(arr_name).cloned()
                    .unwrap_or_default().keys().cloned().collect();
                for key in keys {
                    self.set_var(var, AWKValue::Str(key), locals);
                    match self.exec_stmt(body, locals, prog) {
                        Err(Signal::Break) => break,
                        Err(Signal::Continue) => continue,
                        Err(e) => return Err(e),
                        Ok(_) => {}
                    }
                }
            }
            Stmt::Break    => return Err(Signal::Break),
            Stmt::Continue => return Err(Signal::Continue),
            Stmt::Next     => return Err(Signal::Next),
            Stmt::Exit(e) => {
                let v = e.as_ref().and_then(|ex| self.eval_expr(ex, locals, prog).ok())
                    .unwrap_or(AWKValue::Num(0.0));
                return Err(Signal::Exit(v));
            }
            Stmt::Return(e) => {
                let v = e.as_ref().and_then(|ex| self.eval_expr(ex, locals, prog).ok())
                    .unwrap_or(AWKValue::Uninit);
                return Err(Signal::Return(v));
            }
            Stmt::Delete(name, keys) => {
                if let Some(ks) = keys {
                    let key = ks.iter()
                        .map(|e| self.eval_expr(e, locals, prog).unwrap_or(AWKValue::Uninit).to_str())
                        .collect::<Vec<_>>().join("\x1C");
                    if let Some(arr) = self.arrays.get_mut(name) { arr.remove(&key); }
                } else {
                    self.arrays.insert(name.clone(), HashMap::new());
                }
            }
        }
        Ok(())
    }

    fn eval_expr(&mut self, expr: &Expr, locals: &mut HashMap<String, AWKValue>, prog: &Program) -> Result<AWKValue, AWKError> {
        match expr {
            Expr::Num(n) => Ok(AWKValue::Num(*n)),
            Expr::Str(s) => Ok(AWKValue::Str(s.clone())),
            Expr::Regex(p) => Ok(if simple_match(&self.record, p) { AWKValue::Num(1.0) } else { AWKValue::Num(0.0) }),
            Expr::Field(idx) => {
                let n = self.eval_expr(idx, locals, prog)?.to_num() as usize;
                Ok(self.get_field(n))
            }
            Expr::Var(name) => Ok(self.get_var(name, locals)),
            Expr::Array(name, keys) => {
                let key = keys.iter()
                    .map(|e| self.eval_expr(e, locals, prog).map(|v| v.to_str()))
                    .collect::<Result<Vec<_>, _>>()?.join("\x1C");
                Ok(self.arrays.get(name).and_then(|a| a.get(&key)).cloned().unwrap_or(AWKValue::Uninit))
            }
            Expr::Assign(lhs, rhs) => {
                let val = self.eval_expr(rhs, locals, prog)?;
                self.assign_to(lhs, val.clone(), locals, prog)?;
                Ok(val)
            }
            Expr::CompoundAssign(op, lhs, rhs) => {
                let cur = self.eval_expr(lhs, locals, prog)?;
                let r   = self.eval_expr(rhs, locals, prog)?;
                let result = apply_op(op, &cur, &r);
                self.assign_to(lhs, result.clone(), locals, prog)?;
                Ok(result)
            }
            Expr::PreIncDec(op, e) => {
                let cur = self.eval_expr(e, locals, prog)?;
                let next = if op == "++" { AWKValue::Num(cur.to_num() + 1.0) } else { AWKValue::Num(cur.to_num() - 1.0) };
                self.assign_to(e, next.clone(), locals, prog)?;
                Ok(next)
            }
            Expr::PostIncDec(e, op) => {
                let cur = self.eval_expr(e, locals, prog)?;
                let next = if op == "++" { AWKValue::Num(cur.to_num() + 1.0) } else { AWKValue::Num(cur.to_num() - 1.0) };
                self.assign_to(e, next, locals, prog)?;
                Ok(cur)
            }
            Expr::Unary(op, e) => {
                let v = self.eval_expr(e, locals, prog)?;
                match op.as_str() {
                    "-" => Ok(AWKValue::Num(-v.to_num())),
                    "+" => Ok(AWKValue::Num(v.to_num())),
                    "!" => Ok(AWKValue::Num(if v.to_bool() { 0.0 } else { 1.0 })),
                    _ => Ok(v),
                }
            }
            Expr::Binary(op, l, r) => {
                if op == "&&" {
                    let lv = self.eval_expr(l, locals, prog)?;
                    if !lv.to_bool() { return Ok(AWKValue::Num(0.0)); }
                    return Ok(AWKValue::Num(if self.eval_expr(r, locals, prog)?.to_bool() { 1.0 } else { 0.0 }));
                }
                if op == "||" {
                    let lv = self.eval_expr(l, locals, prog)?;
                    if lv.to_bool() { return Ok(AWKValue::Num(1.0)); }
                    return Ok(AWKValue::Num(if self.eval_expr(r, locals, prog)?.to_bool() { 1.0 } else { 0.0 }));
                }
                let lv = self.eval_expr(l, locals, prog)?;
                let rv = self.eval_expr(r, locals, prog)?;
                match op.as_str() {
                    "+" | "-" | "*" | "/" | "%" | "^" => Ok(apply_op(op, &lv, &rv)),
                    "<"  => Ok(AWKValue::Num(if compare_values(&lv, &rv).is_lt()  { 1.0 } else { 0.0 })),
                    "<=" => Ok(AWKValue::Num(if compare_values(&lv, &rv).is_le()  { 1.0 } else { 0.0 })),
                    ">"  => Ok(AWKValue::Num(if compare_values(&lv, &rv).is_gt()  { 1.0 } else { 0.0 })),
                    ">=" => Ok(AWKValue::Num(if compare_values(&lv, &rv).is_ge()  { 1.0 } else { 0.0 })),
                    "==" => Ok(AWKValue::Num(if compare_values(&lv, &rv).is_eq()  { 1.0 } else { 0.0 })),
                    "!=" => Ok(AWKValue::Num(if !compare_values(&lv, &rv).is_eq() { 1.0 } else { 0.0 })),
                    _ => Ok(AWKValue::Uninit),
                }
            }
            Expr::Ternary(cond, t, f) => {
                if self.eval_expr(cond, locals, prog)?.to_bool() { self.eval_expr(t, locals, prog) }
                else { self.eval_expr(f, locals, prog) }
            }
            Expr::Match(lhs, pat) => {
                let s = self.eval_expr(lhs, locals, prog)?.to_str();
                let p = match pat.as_ref() {
                    Expr::Regex(r) => r.clone(),
                    _ => self.eval_expr(pat, locals, prog)?.to_str(),
                };
                Ok(AWKValue::Num(if simple_match(&s, &p) { 1.0 } else { 0.0 }))
            }
            Expr::InArray(key_expr, arr) => {
                let key = self.eval_expr(key_expr, locals, prog)?.to_str();
                let exists = self.arrays.get(arr).map_or(false, |a| a.contains_key(&key));
                Ok(AWKValue::Num(if exists { 1.0 } else { 0.0 }))
            }
            Expr::Concat(parts) => {
                let mut s = String::new();
                for p in parts { s.push_str(&self.eval_expr(p, locals, prog)?.to_str()); }
                Ok(AWKValue::Str(s))
            }
            Expr::Getline(var) => {
                let mut line = String::new();
                match io::stdin().lock().read_line(&mut line) {
                    Ok(0) => Ok(AWKValue::Num(-1.0)),
                    Ok(_) => {
                        let line = line.trim_end_matches('\n').trim_end_matches('\r').to_string();
                        if let Some(v) = var {
                            self.assign_to(v, AWKValue::Str(line), locals, prog)?;
                        } else {
                            self.nr += 1;
                            self.split_record(&line.clone());
                        }
                        Ok(AWKValue::Num(1.0))
                    }
                    Err(_) => Ok(AWKValue::Num(-1.0)),
                }
            }
            Expr::Call(name, arg_exprs) => self.call_builtin_or_fn(name, arg_exprs, locals, prog),
        }
    }

    fn assign_to(&mut self, expr: &Expr, val: AWKValue, locals: &mut HashMap<String, AWKValue>, prog: &Program) -> Result<(), AWKError> {
        match expr {
            Expr::Var(name) => { self.set_var(name, val, locals); Ok(()) }
            Expr::Field(idx) => {
                let n = self.eval_expr(idx, locals, prog)?.to_num() as usize;
                self.set_field(n, val);
                Ok(())
            }
            Expr::Array(name, keys) => {
                let key = keys.iter()
                    .map(|e| self.eval_expr(e, locals, prog).map(|v| v.to_str()))
                    .collect::<Result<Vec<_>, _>>()?.join("\x1C");
                self.arrays.entry(name.clone()).or_default().insert(key, val);
                Ok(())
            }
            _ => Err(awk_err!("invalid lvalue")),
        }
    }

    fn call_builtin_or_fn(&mut self, name: &str, arg_exprs: &[Expr], locals: &mut HashMap<String, AWKValue>, prog: &Program) -> Result<AWKValue, AWKError> {
        // User-defined function
        if let Some(func) = prog.functions.get(name).cloned() {
            let mut frame: HashMap<String, AWKValue> = HashMap::new();
            for (i, param) in func.params.iter().enumerate() {
                let v = if i < arg_exprs.len() {
                    self.eval_expr(&arg_exprs[i], locals, prog).unwrap_or(AWKValue::Uninit)
                } else { AWKValue::Uninit };
                frame.insert(param.clone(), v);
            }
            return match self.exec_stmt(&func.body, &mut frame, prog) {
                Err(Signal::Return(v)) => Ok(v),
                _ => Ok(AWKValue::Uninit),
            };
        }

        // Evaluate args eagerly for most builtins
        let eval_args = |interp: &mut AWKInterpreter| -> Result<Vec<AWKValue>, AWKError> {
            arg_exprs.iter().map(|e| interp.eval_expr(e, locals, prog)).collect()
        };

        match name {
            "length" => {
                if arg_exprs.is_empty() { return Ok(AWKValue::Num(self.record.len() as f64)); }
                // Check if it's an array name
                if let Expr::Var(n) = &arg_exprs[0] {
                    if let Some(arr) = self.arrays.get(n) { return Ok(AWKValue::Num(arr.len() as f64)); }
                }
                let v = self.eval_expr(&arg_exprs[0], locals, prog)?;
                Ok(AWKValue::Num(v.to_str().len() as f64))
            }
            "substr" => {
                let args = eval_args(self)?;
                let s = args[0].to_str();
                let chars: Vec<char> = s.chars().collect();
                let start = (args[1].to_num() as isize - 1).max(0) as usize;
                if args.len() >= 3 {
                    let len = args[2].to_num() as usize;
                    let end = (start + len).min(chars.len());
                    Ok(AWKValue::Str(chars[start.min(chars.len())..end].iter().collect()))
                } else {
                    Ok(AWKValue::Str(chars[start.min(chars.len())..].iter().collect()))
                }
            }
            "index" => {
                let args = eval_args(self)?;
                let haystack = args[0].to_str();
                let needle = args[1].to_str();
                if let Some(pos) = haystack.find(&needle[..]) {
                    Ok(AWKValue::Num((haystack[..pos].chars().count() + 1) as f64))
                } else { Ok(AWKValue::Num(0.0)) }
            }
            "split" => {
                if arg_exprs.len() < 2 { return Ok(AWKValue::Num(0.0)); }
                let s = self.eval_expr(&arg_exprs[0], locals, prog)?.to_str();
                let arr_name = match &arg_exprs[1] {
                    Expr::Var(n) => n.clone(),
                    _ => return Ok(AWKValue::Num(0.0)),
                };
                let sep = if arg_exprs.len() >= 3 {
                    self.eval_expr(&arg_exprs[2], locals, prog)?.to_str()
                } else { self.field_sep.clone() };
                let parts: Vec<String> = if sep == " " {
                    s.split_whitespace().map(|s| s.to_string()).collect()
                } else { s.split(&sep[..]).map(|s| s.to_string()).collect() };
                let mut arr = HashMap::new();
                for (i, p) in parts.iter().enumerate() {
                    arr.insert((i + 1).to_string(), AWKValue::Str(p.clone()));
                }
                self.arrays.insert(arr_name, arr);
                Ok(AWKValue::Num(parts.len() as f64))
            }
            "sub" | "gsub" => {
                let is_global = name == "gsub";
                if arg_exprs.is_empty() { return Ok(AWKValue::Num(0.0)); }
                let pat = match &arg_exprs[0] {
                    Expr::Regex(r) => r.clone(),
                    _ => self.eval_expr(&arg_exprs[0], locals, prog)?.to_str(),
                };
                let repl = self.eval_expr(&arg_exprs[1], locals, prog)?.to_str();
                let target_expr = if arg_exprs.len() >= 3 { arg_exprs[2].clone() } else { Expr::Field(Box::new(Expr::Num(0.0))) };
                let mut s = self.eval_expr(&target_expr, locals, prog)?.to_str();
                let count = simple_sub(&mut s, &pat, &repl, is_global);
                self.assign_to(&target_expr, AWKValue::Str(s), locals, prog)?;
                Ok(AWKValue::Num(count as f64))
            }
            "match" => {
                let args = eval_args(self)?;
                let s = args[0].to_str();
                let pat = args[1].to_str();
                let (start, len) = simple_match_pos(&s, &pat);
                self.globals.insert("RSTART".into(), AWKValue::Num(start as f64));
                self.globals.insert("RLENGTH".into(), AWKValue::Num(len as f64));
                Ok(AWKValue::Num(start as f64))
            }
            "sprintf" => {
                let args = eval_args(self)?;
                if args.is_empty() { return Ok(AWKValue::Str(String::new())); }
                Ok(AWKValue::Str(sprintf_format(&args[0].to_str(), &args[1..])))
            }
            "int"  => { let a = eval_args(self)?; Ok(AWKValue::Num((a[0].to_num() as i64) as f64)) }
            "sqrt" => { let a = eval_args(self)?; Ok(AWKValue::Num(a[0].to_num().sqrt())) }
            "log"  => { let a = eval_args(self)?; Ok(AWKValue::Num(a[0].to_num().ln())) }
            "exp"  => { let a = eval_args(self)?; Ok(AWKValue::Num(a[0].to_num().exp())) }
            "sin"  => { let a = eval_args(self)?; Ok(AWKValue::Num(a[0].to_num().sin())) }
            "cos"  => { let a = eval_args(self)?; Ok(AWKValue::Num(a[0].to_num().cos())) }
            "atan2" => { let a = eval_args(self)?; Ok(AWKValue::Num(a[0].to_num().atan2(a[1].to_num()))) }
            "rand"  => Ok(AWKValue::Num(rand_f64())),
            "srand" => Ok(AWKValue::Num(0.0)),
            "tolower" => { let a = eval_args(self)?; Ok(AWKValue::Str(a[0].to_str().to_lowercase())) }
            "toupper" => { let a = eval_args(self)?; Ok(AWKValue::Str(a[0].to_str().to_uppercase())) }
            "system" => {
                let args = eval_args(self)?;
                let status = ProcessCommand::new("sh").arg("-c").arg(args[0].to_str())
                    .stdout(Stdio::inherit()).stderr(Stdio::inherit()).status();
                Ok(AWKValue::Num(status.map(|s| s.code().unwrap_or(0) as f64).unwrap_or(-1.0)))
            }
            _ => { eprintln!("awk: undefined function '{}'", name); Ok(AWKValue::Uninit) }
        }
    }

    fn write_output(&self, text: &str, redirect: Option<&Redirect>, locals: &mut HashMap<String, AWKValue>, prog: &Program) {
        match redirect {
            None => { print!("{}", text); let _ = io::stdout().flush(); }
            Some(Redirect::File(path_expr)) => {
                // We need interior mutability here; use a workaround with a local eval
                // Since we can't mutably borrow self here, we write using cached path
                let path = match path_expr {
                    Expr::Str(s) => s.clone(),
                    Expr::Var(n) => self.globals.get(n).cloned().unwrap_or(AWKValue::Uninit).to_str(),
                    _ => String::new(),
                };
                if let Ok(mut f) = std::fs::OpenOptions::new().write(true).create(true).open(&path) {
                    let _ = f.write_all(text.as_bytes());
                }
            }
            Some(Redirect::Append(path_expr)) => {
                let path = match path_expr {
                    Expr::Str(s) => s.clone(),
                    Expr::Var(n) => self.globals.get(n).cloned().unwrap_or(AWKValue::Uninit).to_str(),
                    _ => String::new(),
                };
                if let Ok(mut f) = std::fs::OpenOptions::new().append(true).create(true).open(&path) {
                    let _ = f.write_all(text.as_bytes());
                }
            }
            Some(Redirect::Pipe(cmd_expr)) => {
                let cmd = match cmd_expr {
                    Expr::Str(s) => s.clone(),
                    Expr::Var(n) => self.globals.get(n).cloned().unwrap_or(AWKValue::Uninit).to_str(),
                    _ => String::new(),
                };
                if let Ok(mut child) = ProcessCommand::new("sh").arg("-c").arg(&cmd)
                    .stdin(Stdio::piped()).spawn()
                {
                    if let Some(stdin) = child.stdin.take() {
                        let mut stdin = stdin;
                        let _ = stdin.write_all(text.as_bytes());
                    }
                    let _ = child.wait();
                }
            }
        }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn apply_op(op: &str, a: &AWKValue, b: &AWKValue) -> AWKValue {
    let (x, y) = (a.to_num(), b.to_num());
    AWKValue::Num(match op {
        "+" => x + y,
        "-" => x - y,
        "*" => x * y,
        "/" => if y == 0.0 { eprintln!("awk: division by zero"); 0.0 } else { x / y },
        "%" => if y == 0.0 { eprintln!("awk: modulo by zero");   0.0 } else { x % y },
        "^" => x.powf(y),
        _ => 0.0,
    })
}

// Minimal regex-like matching using Rust's built-in string ops where possible,
// with a simple NFA for common patterns (no external crate needed).
fn simple_match(text: &str, pattern: &str) -> bool {
    simple_match_pos(text, pattern).0 > 0
}

fn simple_match_pos(text: &str, pattern: &str) -> (usize, i64) {
    // Use grep as a subprocess for full regex support, or fall back to literal
    // For common awk usage, a subprocess call is reliable and correct.
    let result = ProcessCommand::new("grep")
        .args(["-oP", "--", pattern])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    if let Ok(mut child) = result {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
            let _ = stdin.write_all(b"\n");
        }
        if let Ok(out) = child.wait_with_output() {
            let matched = String::from_utf8_lossy(&out.stdout);
            let matched = matched.trim_end_matches('\n');
            if !matched.is_empty() {
                if let Some(pos) = text.find(matched) {
                    let char_pos = text[..pos].chars().count() + 1;
                    let char_len = matched.chars().count() as i64;
                    return (char_pos, char_len);
                }
            }
        }
    }
    // Fallback: literal substring search
    if let Some(pos) = text.find(pattern) {
        let char_pos = text[..pos].chars().count() + 1;
        let char_len = pattern.chars().count() as i64;
        (char_pos, char_len)
    } else {
        (0, -1)
    }
}

fn simple_split(text: &str, sep: &str) -> Vec<String> {
    text.split(sep).map(|s| s.to_string()).collect()
}

fn simple_sub(text: &mut String, pattern: &str, repl: &str, global: bool) -> usize {
    let result = ProcessCommand::new("grep")
        .args(["-oP", "--", pattern])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();

    let mut count = 0;
    let mut new_text = String::new();
    let mut remaining = text.as_str();

    // Use grep to find all matches, replace them
    if let Ok(mut child) = result {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
        }
        if let Ok(out) = child.wait_with_output() {
            let matches_str = String::from_utf8_lossy(&out.stdout);
            let matches: Vec<&str> = matches_str.lines().collect();
            for m in matches {
                if m.is_empty() { continue; }
                if let Some(pos) = remaining.find(m) {
                    new_text.push_str(&remaining[..pos]);
                    let actual_repl = repl.replace('&', m);
                    new_text.push_str(&actual_repl);
                    remaining = &remaining[pos + m.len()..];
                    count += 1;
                    if !global { break; }
                }
            }
            new_text.push_str(remaining);
            *text = new_text;
            return count;
        }
    }
    // Fallback: literal replace
    if global {
        let c = text.matches(pattern).count();
        *text = text.replace(pattern, &repl.replace('&', pattern));
        c
    } else {
        if let Some(pos) = text.find(pattern) {
            let actual_repl = repl.replace('&', &text[pos..pos + pattern.len()]);
            text.replace_range(pos..pos + pattern.len(), &actual_repl);
            1
        } else { 0 }
    }
}

// Simple LCG for rand() — no external crate needed
static mut RAND_STATE: u64 = 12345;
fn rand_f64() -> f64 {
    unsafe {
        RAND_STATE = RAND_STATE.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((RAND_STATE >> 33) as f64) / (u32::MAX as f64)
    }
}

// ─── sprintf ────────────────────────────────────────────────────────────────

fn sprintf_format(fmt: &str, args: &[AWKValue]) -> String {
    let chars: Vec<char> = fmt.chars().collect();
    let mut result = String::new();
    let mut i = 0;
    let mut arg_idx = 0;

    while i < chars.len() {
        if chars[i] != '%' { result.push(chars[i]); i += 1; continue; }
        i += 1;
        if i >= chars.len() { break; }
        if chars[i] == '%' { result.push('%'); i += 1; continue; }

        // Flags
        let mut flags = String::new();
        while i < chars.len() && "-+ #0".contains(chars[i]) { flags.push(chars[i]); i += 1; }
        // Width
        let mut width_s = String::new();
        while i < chars.len() && chars[i].is_ascii_digit() { width_s.push(chars[i]); i += 1; }
        // Precision
        let mut prec_s = String::new();
        if i < chars.len() && chars[i] == '.' {
            i += 1;
            while i < chars.len() && chars[i].is_ascii_digit() { prec_s.push(chars[i]); i += 1; }
        }
        if i >= chars.len() { break; }
        let spec = chars[i]; i += 1;
        let arg = args.get(arg_idx).cloned().unwrap_or(AWKValue::Uninit);
        arg_idx += 1;

        let width: usize = width_s.parse().unwrap_or(0);
        let prec: i64 = prec_s.parse().unwrap_or(-1);
        let left = flags.contains('-');
        let zero = flags.contains('0');
        let plus = flags.contains('+');

        let pad = |s: String, w: usize| -> String {
            if s.len() >= w { return s; }
            let p = w - s.len();
            let pad_char = if zero && !left { '0' } else { ' ' };
            if left { format!("{}{}", s, " ".repeat(p)) }
            else { format!("{}{}", pad_char.to_string().repeat(p), s) }
        };

        match spec {
            'd' | 'i' => {
                let n = arg.to_num() as i64;
                let s = if plus && n >= 0 { format!("+{}", n) } else { n.to_string() };
                result.push_str(&pad(s, width));
            }
            'u' => { result.push_str(&pad((arg.to_num() as u64).to_string(), width)); }
            'o' => { result.push_str(&pad(format!("{:o}", arg.to_num() as u64), width)); }
            'x' => { result.push_str(&pad(format!("{:x}", arg.to_num() as u64), width)); }
            'X' => { result.push_str(&pad(format!("{:X}", arg.to_num() as u64), width)); }
            'f' => {
                let p = if prec >= 0 { prec as usize } else { 6 };
                result.push_str(&pad(format!("{:.prec$}", arg.to_num(), prec = p), width));
            }
            'e' => {
                let p = if prec >= 0 { prec as usize } else { 6 };
                result.push_str(&pad(format!("{:.prec$e}", arg.to_num(), prec = p), width));
            }
            'E' => {
                let p = if prec >= 0 { prec as usize } else { 6 };
                result.push_str(&pad(format!("{:.prec$E}", arg.to_num(), prec = p), width));
            }
            'g' | 'G' => {
                let p = if prec >= 0 { prec as usize } else { 6 };
                let n = arg.to_num();
                let s = if n.abs() < 1e-4 || n.abs() >= 10f64.powi(p as i32) {
                    format!("{:.prec$e}", n, prec = p.saturating_sub(1))
                } else {
                    let s = format!("{:.prec$}", n, prec = p);
                    s.trim_end_matches('0').trim_end_matches('.').to_string()
                };
                let s = if spec == 'G' { s.to_uppercase() } else { s };
                result.push_str(&pad(s, width));
            }
            's' => {
                let mut s = arg.to_str();
                if prec >= 0 { s = s.chars().take(prec as usize).collect(); }
                result.push_str(&pad(s, width));
            }
            'c' => {
                let s = match arg {
                    AWKValue::Str(ref sv) if !sv.is_empty() => sv.chars().next().unwrap().to_string(),
                    _ => char::from_u32(arg.to_num() as u32).unwrap_or('\0').to_string(),
                };
                result.push_str(&pad(s, width));
            }
            _ => { result.push('%'); result.push(spec); }
        }
    }
    result
}

// ─── Help / version ──────────────────────────────────────────────────────────

fn awk_print_usage() {
    println!("Usage: awk [OPTION]... 'program' [FILE]...");
    println!("   or: awk [OPTION]... -f progfile [FILE]...");
    println!("Scan and process patterns in each FILE (or standard input).");
    println!("\nOptions:");
    println!("  -F fs       use fs as the input field separator (FS)");
    println!("  -v var=val  assign value val to variable var before execution");
    println!("  -f file     read program text from file");
    println!("  --help      display this help and exit");
    println!("  --version   output version information and exit");
    println!("\nA program consists of rules: /pattern/ {{ action }}");
    println!("Special patterns: BEGIN {{ ... }}  and  END {{ ... }}");
    println!("\nVariables:   NR  NF  FS  OFS  ORS  RS  FILENAME");
    println!("Functions:   length  substr  index  split  sub  gsub  match");
    println!("             sprintf  int  sqrt  log  exp  sin  cos  atan2");
    println!("             rand  srand  tolower  toupper  system");
}

fn awk_print_version() {
    println!("awk (cacutils) v1.0");
    println!("IEEE Std 1003.1-2008 (POSIX) compatible implementation.");
    println!("There is NO WARRANTY, to the extent permitted by law.");
    println!("Written by Cyril John Magayaga.");
}
