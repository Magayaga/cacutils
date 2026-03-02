package main

import (
	"bufio"
	"fmt"
	"math"
	"math/rand"
	"os"
	"os/exec"
	"regexp"
	"strconv"
	"strings"
	"unicode"
)

// ─── Entry point ─────────────────────────────────────────────────────────────

func awkCommand(arguments []string) {
	if awkContains(arguments, "--help") {
		awkPrintUsage()
		return
	}
	if awkContains(arguments, "--version") {
		awkPrintVersion()
		return
	}
	if len(arguments) == 0 {
		fmt.Fprintln(os.Stderr, "awk: no program given\nTry 'awk --help' for more information.")
		return
	}

	var programSource string
	var programFile string
	fieldSep := " "
	var assignments [][]string
	var inputFiles []string
	i := 0

	for i < len(arguments) {
		switch arguments[i] {
		case "-f":
			i++
			if i >= len(arguments) {
				fmt.Fprintln(os.Stderr, "awk: -f requires a file argument")
				return
			}
			programFile = arguments[i]
		case "-F":
			i++
			if i >= len(arguments) {
				fmt.Fprintln(os.Stderr, "awk: -F requires a separator argument")
				return
			}
			fieldSep = awkUnescapeFS(arguments[i])
		case "-v":
			i++
			if i >= len(arguments) {
				fmt.Fprintln(os.Stderr, "awk: -v requires a var=value argument")
				return
			}
			parts := strings.SplitN(arguments[i], "=", 2)
			if len(parts) == 2 {
				assignments = append(assignments, parts)
			} else {
				fmt.Fprintf(os.Stderr, "awk: invalid -v assignment: %s\n", arguments[i])
				return
			}
		default:
			arg := arguments[i]
			if strings.HasPrefix(arg, "-F") && len(arg) > 2 {
				fieldSep = awkUnescapeFS(arg[2:])
			} else if programSource == "" && programFile == "" {
				programSource = arg
			} else if strings.Contains(arg, "=") && !strings.HasPrefix(arg, "-") {
				parts := strings.SplitN(arg, "=", 2)
				assignments = append(assignments, parts)
			} else {
				inputFiles = append(inputFiles, arg)
			}
		}
		i++
	}

	// Load source
	var source string
	if programFile != "" {
		data, err := os.ReadFile(programFile)
		if err != nil {
			fmt.Fprintf(os.Stderr, "awk: cannot open program file '%s': %v\n", programFile, err)
			return
		}
		source = string(data)
	} else if programSource != "" {
		source = programSource
	} else {
		fmt.Fprintln(os.Stderr, "awk: no program given")
		return
	}

	prog, err := awkParse(source)
	if err != nil {
		fmt.Fprintf(os.Stderr, "awk: %v\n", err)
		return
	}

	interp := newAWKInterpreter(fieldSep)
	for _, kv := range assignments {
		interp.globals[kv[0]] = awkStr(kv[1])
	}

	if len(inputFiles) == 0 {
		// Read from stdin
		scanner := bufio.NewScanner(os.Stdin)
		var lines []string
		for scanner.Scan() {
			lines = append(lines, scanner.Text())
		}
		interp.runLines(prog, lines)
	} else {
		for _, path := range inputFiles {
			data, err := os.ReadFile(path)
			if err != nil {
				fmt.Fprintf(os.Stderr, "awk: cannot open '%s': %v\n", path, err)
				continue
			}
			interp.filename = path
			lines := strings.Split(strings.TrimRight(string(data), "\n"), "\n")
			interp.runLines(prog, lines)
		}
	}
}

func awkUnescapeFS(s string) string {
	s = strings.ReplaceAll(s, `\t`, "\t")
	s = strings.ReplaceAll(s, `\n`, "\n")
	return s
}

func awkContains(slice []string, item string) bool {
	for _, s := range slice {
		if s == item {
			return true
		}
	}
	return false
}

// ─── Value ───────────────────────────────────────────────────────────────────

type AWKVal struct {
	s     string
	n     float64
	isNum bool
	isNil bool
}

func awkStr(s string) AWKVal  { return AWKVal{s: s} }
func awkNum(n float64) AWKVal { return AWKVal{n: n, isNum: true} }
var awkNil = AWKVal{isNil: true}

func (v AWKVal) str() string {
	if v.isNum {
		return awkFormatNum(v.n)
	}
	return v.s
}

func (v AWKVal) num() float64 {
	if v.isNum {
		return v.n
	}
	f, err := strconv.ParseFloat(strings.TrimSpace(v.s), 64)
	if err != nil {
		return 0
	}
	return f
}

func (v AWKVal) bool() bool {
	if v.isNil {
		return false
	}
	if v.isNum {
		return v.n != 0
	}
	return v.s != ""
}

func awkFormatNum(n float64) string {
	if n == math.Trunc(n) && math.Abs(n) < 1e15 && !math.IsInf(n, 0) {
		return strconv.FormatInt(int64(n), 10)
	}
	s := strconv.FormatFloat(n, 'g', 6, 64)
	return s
}

func awkCompare(a, b AWKVal) int {
	if a.isNum && b.isNum {
		if a.n < b.n {
			return -1
		}
		if a.n > b.n {
			return 1
		}
		return 0
	}
	as_, bs_ := a.str(), b.str()
	an, ae := strconv.ParseFloat(strings.TrimSpace(as_), 64)
	bn, be := strconv.ParseFloat(strings.TrimSpace(bs_), 64)
	if ae == nil && be == nil {
		if an < bn {
			return -1
		}
		if an > bn {
			return 1
		}
		return 0
	}
	if as_ < bs_ {
		return -1
	}
	if as_ > bs_ {
		return 1
	}
	return 0
}

// ─── Lexer ───────────────────────────────────────────────────────────────────

type awkTK int

const (
	tkNum awkTK = iota
	tkStr
	tkIdent
	tkRegex
	tkPlus
	tkMinus
	tkStar
	tkSlash
	tkPercent
	tkCaret
	tkPlusEq
	tkMinusEq
	tkStarEq
	tkSlashEq
	tkPercentEq
	tkCaretEq
	tkPlusPlus
	tkMinusMinus
	tkEq
	tkEqEq
	tkBangEq
	tkLt
	tkLtEq
	tkGt
	tkGtEq
	tkAnd
	tkOr
	tkBang
	tkTilde
	tkBangTilde
	tkComma
	tkSemi
	tkColon
	tkQuestion
	tkLBrace
	tkRBrace
	tkLParen
	tkRParen
	tkLBrack
	tkRBrack
	tkDollar
	tkNewline
	tkPipe
	tkAppend
	tkEOF
	// Keywords
	tkBEGIN
	tkEND
	tkIf
	tkElse
	tkWhile
	tkDo
	tkFor
	tkIn
	tkBreak
	tkContinue
	tkNext
	tkExit
	tkPrint
	tkPrintf
	tkDelete
	tkReturn
	tkFunction
	tkGetline
)

type awkToken struct {
	kind  awkTK
	sval  string
	nval  float64
	line  int
}

type awkLexer struct {
	chars []rune
	pos   int
	line  int
}

func newAWKLexer(src string) *awkLexer {
	return &awkLexer{chars: []rune(src), pos: 0, line: 1}
}

func (l *awkLexer) peek() (rune, bool) {
	if l.pos < len(l.chars) {
		return l.chars[l.pos], true
	}
	return 0, false
}

func (l *awkLexer) peek2() (rune, bool) {
	if l.pos+1 < len(l.chars) {
		return l.chars[l.pos+1], true
	}
	return 0, false
}

func (l *awkLexer) advance() rune {
	c := l.chars[l.pos]
	if c == '\n' {
		l.line++
	}
	l.pos++
	return c
}

func (l *awkLexer) skipWSComments() {
	for {
		c, ok := l.peek()
		if !ok {
			break
		}
		if c == '#' {
			for c2, ok2 := l.peek(); ok2 && c2 != '\n'; c2, ok2 = l.peek() {
				l.advance()
			}
		} else if c == ' ' || c == '\t' || c == '\r' {
			l.advance()
		} else {
			break
		}
	}
}

func (l *awkLexer) next() (awkToken, error) {
	l.skipWSComments()
	ln := l.line
	c, ok := l.peek()
	if !ok {
		return awkToken{kind: tkEOF, line: ln}, nil
	}
	if c == '\n' {
		l.advance()
		return awkToken{kind: tkNewline, line: ln}, nil
	}

	// String literal
	if c == '"' {
		l.advance()
		var buf strings.Builder
		for {
			ch, ok := l.peek()
			if !ok || ch == '\n' {
				return awkToken{}, fmt.Errorf("unterminated string at line %d", ln)
			}
			if ch == '"' {
				l.advance()
				break
			}
			if ch == '\\' {
				l.advance()
				esc, ok := l.peek()
				if ok {
					l.advance()
					switch esc {
					case 'n':
						buf.WriteRune('\n')
					case 't':
						buf.WriteRune('\t')
					case 'r':
						buf.WriteRune('\r')
					case '\\':
						buf.WriteRune('\\')
					case '"':
						buf.WriteRune('"')
					case '/':
						buf.WriteRune('/')
					default:
						buf.WriteRune('\\')
						buf.WriteRune(esc)
					}
				}
				continue
			}
			buf.WriteRune(ch)
			l.advance()
		}
		return awkToken{kind: tkStr, sval: buf.String(), line: ln}, nil
	}

	// Number
	c2, _ := l.peek2()
	if unicode.IsDigit(c) || (c == '.' && unicode.IsDigit(c2)) {
		var buf strings.Builder
		for ch, ok := l.peek(); ok && unicode.IsDigit(ch); ch, ok = l.peek() {
			buf.WriteRune(l.advance())
		}
		if ch, ok := l.peek(); ok && ch == '.' {
			buf.WriteRune(l.advance())
			for ch2, ok := l.peek(); ok && unicode.IsDigit(ch2); ch2, ok = l.peek() {
				buf.WriteRune(l.advance())
			}
		}
		if ch, ok := l.peek(); ok && (ch == 'e' || ch == 'E') {
			buf.WriteRune(l.advance())
			if ch2, ok := l.peek(); ok && (ch2 == '+' || ch2 == '-') {
				buf.WriteRune(l.advance())
			}
			for ch3, ok := l.peek(); ok && unicode.IsDigit(ch3); ch3, ok = l.peek() {
				buf.WriteRune(l.advance())
			}
		}
		n, _ := strconv.ParseFloat(buf.String(), 64)
		return awkToken{kind: tkNum, nval: n, line: ln}, nil
	}

	// Identifier / keyword
	if unicode.IsLetter(c) || c == '_' {
		var buf strings.Builder
		for ch, ok := l.peek(); ok && (unicode.IsLetter(ch) || unicode.IsDigit(ch) || ch == '_'); ch, ok = l.peek() {
			buf.WriteRune(l.advance())
		}
		kw := map[string]awkTK{
			"BEGIN": tkBEGIN, "END": tkEND,
			"if": tkIf, "else": tkElse,
			"while": tkWhile, "do": tkDo, "for": tkFor, "in": tkIn,
			"break": tkBreak, "continue": tkContinue,
			"next": tkNext, "exit": tkExit,
			"print": tkPrint, "printf": tkPrintf,
			"delete": tkDelete, "return": tkReturn,
			"function": tkFunction, "getline": tkGetline,
		}
		word := buf.String()
		if k, found := kw[word]; found {
			return awkToken{kind: k, line: ln}, nil
		}
		return awkToken{kind: tkIdent, sval: word, line: ln}, nil
	}

	// Operators
	l.advance()
	switch c {
	case '+':
		if ch, ok := l.peek(); ok {
			if ch == '+' {
				l.advance()
				return awkToken{kind: tkPlusPlus, line: ln}, nil
			}
			if ch == '=' {
				l.advance()
				return awkToken{kind: tkPlusEq, line: ln}, nil
			}
		}
		return awkToken{kind: tkPlus, line: ln}, nil
	case '-':
		if ch, ok := l.peek(); ok {
			if ch == '-' {
				l.advance()
				return awkToken{kind: tkMinusMinus, line: ln}, nil
			}
			if ch == '=' {
				l.advance()
				return awkToken{kind: tkMinusEq, line: ln}, nil
			}
		}
		return awkToken{kind: tkMinus, line: ln}, nil
	case '*':
		if ch, ok := l.peek(); ok && ch == '=' {
			l.advance()
			return awkToken{kind: tkStarEq, line: ln}, nil
		}
		return awkToken{kind: tkStar, line: ln}, nil
	case '/':
		if ch, ok := l.peek(); ok && ch == '=' {
			l.advance()
			return awkToken{kind: tkSlashEq, line: ln}, nil
		}
		return awkToken{kind: tkSlash, line: ln}, nil
	case '%':
		if ch, ok := l.peek(); ok && ch == '=' {
			l.advance()
			return awkToken{kind: tkPercentEq, line: ln}, nil
		}
		return awkToken{kind: tkPercent, line: ln}, nil
	case '^':
		if ch, ok := l.peek(); ok && ch == '=' {
			l.advance()
			return awkToken{kind: tkCaretEq, line: ln}, nil
		}
		return awkToken{kind: tkCaret, line: ln}, nil
	case '=':
		if ch, ok := l.peek(); ok && ch == '=' {
			l.advance()
			return awkToken{kind: tkEqEq, line: ln}, nil
		}
		return awkToken{kind: tkEq, line: ln}, nil
	case '!':
		if ch, ok := l.peek(); ok {
			if ch == '=' {
				l.advance()
				return awkToken{kind: tkBangEq, line: ln}, nil
			}
			if ch == '~' {
				l.advance()
				return awkToken{kind: tkBangTilde, line: ln}, nil
			}
		}
		return awkToken{kind: tkBang, line: ln}, nil
	case '<':
		if ch, ok := l.peek(); ok && ch == '=' {
			l.advance()
			return awkToken{kind: tkLtEq, line: ln}, nil
		}
		return awkToken{kind: tkLt, line: ln}, nil
	case '>':
		if ch, ok := l.peek(); ok {
			if ch == '>' {
				l.advance()
				return awkToken{kind: tkAppend, line: ln}, nil
			}
			if ch == '=' {
				l.advance()
				return awkToken{kind: tkGtEq, line: ln}, nil
			}
		}
		return awkToken{kind: tkGt, line: ln}, nil
	case '&':
		if ch, ok := l.peek(); ok && ch == '&' {
			l.advance()
			return awkToken{kind: tkAnd, line: ln}, nil
		}
		return awkToken{}, fmt.Errorf("unexpected '&' at line %d", ln)
	case '|':
		if ch, ok := l.peek(); ok && ch == '|' {
			l.advance()
			return awkToken{kind: tkOr, line: ln}, nil
		}
		return awkToken{kind: tkPipe, line: ln}, nil
	case '~':
		return awkToken{kind: tkTilde, line: ln}, nil
	case ',':
		return awkToken{kind: tkComma, line: ln}, nil
	case ';':
		return awkToken{kind: tkSemi, line: ln}, nil
	case ':':
		return awkToken{kind: tkColon, line: ln}, nil
	case '?':
		return awkToken{kind: tkQuestion, line: ln}, nil
	case '{':
		return awkToken{kind: tkLBrace, line: ln}, nil
	case '}':
		return awkToken{kind: tkRBrace, line: ln}, nil
	case '(':
		return awkToken{kind: tkLParen, line: ln}, nil
	case ')':
		return awkToken{kind: tkRParen, line: ln}, nil
	case '[':
		return awkToken{kind: tkLBrack, line: ln}, nil
	case ']':
		return awkToken{kind: tkRBrack, line: ln}, nil
	case '$':
		return awkToken{kind: tkDollar, line: ln}, nil
	case '\\':
		if ch, ok := l.peek(); ok && ch == '\n' {
			l.advance()
			return l.next()
		}
		return awkToken{}, fmt.Errorf("unexpected '\\' at line %d", ln)
	}
	return awkToken{}, fmt.Errorf("unexpected character '%c' at line %d", c, ln)
}

func (l *awkLexer) lexRegex() (awkToken, error) {
	ln := l.line
	var buf strings.Builder
	for {
		c, ok := l.peek()
		if !ok || c == '\n' {
			return awkToken{}, fmt.Errorf("unterminated regex at line %d", ln)
		}
		if c == '/' {
			l.advance()
			break
		}
		if c == '\\' {
			l.advance()
			if c2, ok := l.peek(); ok {
				l.advance()
				buf.WriteRune('\\')
				buf.WriteRune(c2)
			}
			continue
		}
		buf.WriteRune(c)
		l.advance()
	}
	return awkToken{kind: tkRegex, sval: buf.String(), line: ln}, nil
}

// ─── AST ─────────────────────────────────────────────────────────────────────

type awkExpr interface{ isExpr() }
type awkStmt interface{ isStmt() }

type (
	exNum      struct{ n float64 }
	exStr      struct{ s string }
	exRegex    struct{ pat string }
	exField    struct{ idx awkExpr }
	exVar      struct{ name string }
	exArray    struct{ name string; keys []awkExpr }
	exAssign   struct{ lhs, rhs awkExpr }
	exCompound struct{ op string; lhs, rhs awkExpr }
	exPreInc   struct{ op string; e awkExpr }
	exPostInc  struct{ e awkExpr; op string }
	exUnary    struct{ op string; e awkExpr }
	exBinary   struct{ op string; l, r awkExpr }
	exTernary  struct{ cond, t, f awkExpr }
	exMatch    struct{ l, r awkExpr }
	exCall     struct{ name string; args []awkExpr }
	exGetline  struct{ target awkExpr }
	exInArray  struct{ key awkExpr; arr string }
	exConcat   struct{ parts []awkExpr }
)

func (exNum) isExpr()      {}
func (exStr) isExpr()      {}
func (exRegex) isExpr()    {}
func (exField) isExpr()    {}
func (exVar) isExpr()      {}
func (exArray) isExpr()    {}
func (exAssign) isExpr()   {}
func (exCompound) isExpr() {}
func (exPreInc) isExpr()   {}
func (exPostInc) isExpr()  {}
func (exUnary) isExpr()    {}
func (exBinary) isExpr()   {}
func (exTernary) isExpr()  {}
func (exMatch) isExpr()    {}
func (exCall) isExpr()     {}
func (exGetline) isExpr()  {}
func (exInArray) isExpr()  {}
func (exConcat) isExpr()   {}

type awkRedirect struct {
	kind string // "file", "append", "pipe"
	expr awkExpr
}

type (
	stBlock  struct{ stmts []awkStmt }
	stExpr   struct{ e awkExpr }
	stPrint  struct{ args []awkExpr; redirect *awkRedirect; isPrintf bool }
	stIf     struct{ cond awkExpr; then, els awkStmt }
	stWhile  struct{ cond awkExpr; body awkStmt }
	stDo     struct{ body awkStmt; cond awkExpr }
	stFor    struct{ init, post awkStmt; cond awkExpr; body awkStmt }
	stForIn  struct{ varName, arrName string; body awkStmt }
	stBreak  struct{}
	stCont   struct{}
	stNext   struct{}
	stExit   struct{ e awkExpr }
	stReturn struct{ e awkExpr }
	stDelete struct{ name string; keys []awkExpr }
	stEmpty  struct{}
)

func (stBlock) isStmt()  {}
func (stExpr) isStmt()   {}
func (stPrint) isStmt()  {}
func (stIf) isStmt()     {}
func (stWhile) isStmt()  {}
func (stDo) isStmt()     {}
func (stFor) isStmt()    {}
func (stForIn) isStmt()  {}
func (stBreak) isStmt()  {}
func (stCont) isStmt()   {}
func (stNext) isStmt()   {}
func (stExit) isStmt()   {}
func (stReturn) isStmt() {}
func (stDelete) isStmt() {}
func (stEmpty) isStmt()  {}

type awkPattern interface{ isPattern() }
type (
	patBegin  struct{}
	patEnd    struct{}
	patExpr   struct{ e awkExpr }
	patRange  struct{ start, end awkExpr }
	patAlways struct{}
)

func (patBegin) isPattern()  {}
func (patEnd) isPattern()    {}
func (patExpr) isPattern()   {}
func (patRange) isPattern()  {}
func (patAlways) isPattern() {}

type awkRule struct {
	pattern awkPattern
	action  awkStmt
}

type awkFunction struct {
	params []string
	body   awkStmt
}

type awkProgram struct {
	rules     []awkRule
	functions map[string]*awkFunction
}

// ─── Parser ───────────────────────────────────────────────────────────────────

type awkParser struct {
	lexer   *awkLexer
	current awkToken
	peeked  *awkToken
}

func awkParse(src string) (*awkProgram, error) {
	p := &awkParser{lexer: newAWKLexer(src)}
	var err error
	p.current, err = p.lexer.next()
	if err != nil {
		return nil, err
	}
	return p.parse()
}

func (p *awkParser) peekTok() (awkToken, error) {
	if p.peeked != nil {
		return *p.peeked, nil
	}
	t, err := p.lexer.next()
	if err != nil {
		return awkToken{}, err
	}
	p.peeked = &t
	return t, nil
}

func (p *awkParser) advance() (awkToken, error) {
	t := p.current
	if p.peeked != nil {
		p.current = *p.peeked
		p.peeked = nil
	} else {
		next, err := p.lexer.next()
		if err != nil {
			return awkToken{}, err
		}
		p.current = next
	}
	return t, nil
}

func (p *awkParser) expect(k awkTK) error {
	if p.current.kind != k {
		return fmt.Errorf("expected token %d at line %d, got %d", k, p.current.line, p.current.kind)
	}
	_, err := p.advance()
	return err
}

func (p *awkParser) skipNLs() error {
	for p.current.kind == tkNewline {
		if _, err := p.advance(); err != nil {
			return err
		}
	}
	return nil
}

func (p *awkParser) skipTerminators() error {
	for p.current.kind == tkNewline || p.current.kind == tkSemi {
		if _, err := p.advance(); err != nil {
			return err
		}
	}
	return nil
}

func (p *awkParser) isStmtEnd() bool {
	return p.current.kind == tkNewline || p.current.kind == tkSemi ||
		p.current.kind == tkRBrace || p.current.kind == tkEOF
}

func (p *awkParser) parse() (*awkProgram, error) {
	prog := &awkProgram{functions: make(map[string]*awkFunction)}
	if err := p.skipTerminators(); err != nil {
		return nil, err
	}
	for p.current.kind != tkEOF {
		if p.current.kind == tkFunction {
			if _, err := p.advance(); err != nil {
				return nil, err
			}
			if p.current.kind != tkIdent {
				return nil, fmt.Errorf("expected function name at line %d", p.current.line)
			}
			name := p.current.sval
			if _, err := p.advance(); err != nil {
				return nil, err
			}
			if err := p.expect(tkLParen); err != nil {
				return nil, err
			}
			var params []string
			for p.current.kind == tkIdent {
				params = append(params, p.current.sval)
				if _, err := p.advance(); err != nil {
					return nil, err
				}
				if p.current.kind == tkComma {
					if _, err := p.advance(); err != nil {
						return nil, err
					}
				}
			}
			if err := p.expect(tkRParen); err != nil {
				return nil, err
			}
			if err := p.skipNLs(); err != nil {
				return nil, err
			}
			body, err := p.parseBlock()
			if err != nil {
				return nil, err
			}
			prog.functions[name] = &awkFunction{params: params, body: body}
		} else {
			rule, err := p.parseRule()
			if err != nil {
				return nil, err
			}
			prog.rules = append(prog.rules, rule)
		}
		if err := p.skipTerminators(); err != nil {
			return nil, err
		}
	}
	return prog, nil
}

func (p *awkParser) parseRule() (awkRule, error) {
	var pat awkPattern
	switch p.current.kind {
	case tkBEGIN:
		p.advance()
		pat = patBegin{}
	case tkEND:
		p.advance()
		pat = patEnd{}
	case tkLBrace:
		pat = patAlways{}
	default:
		e1, err := p.parseExpr()
		if err != nil {
			return awkRule{}, err
		}
		if p.current.kind == tkComma {
			p.advance()
			p.skipNLs()
			e2, err := p.parseExpr()
			if err != nil {
				return awkRule{}, err
			}
			pat = patRange{start: e1, end: e2}
		} else {
			pat = patExpr{e: e1}
		}
	}
	p.skipNLs()
	var action awkStmt
	var err error
	if p.current.kind == tkLBrace {
		action, err = p.parseBlock()
		if err != nil {
			return awkRule{}, err
		}
	} else {
		action = stPrint{}
	}
	return awkRule{pattern: pat, action: action}, nil
}

func (p *awkParser) parseBlock() (awkStmt, error) {
	if err := p.expect(tkLBrace); err != nil {
		return nil, err
	}
	p.skipTerminators()
	var stmts []awkStmt
	for p.current.kind != tkRBrace {
		if p.current.kind == tkEOF {
			return nil, fmt.Errorf("unclosed block")
		}
		s, err := p.parseStmt()
		if err != nil {
			return nil, err
		}
		stmts = append(stmts, s)
		p.skipTerminators()
	}
	p.advance()
	return stBlock{stmts: stmts}, nil
}

func (p *awkParser) parseStmt() (awkStmt, error) {
	ln := p.current.line
	switch p.current.kind {
	case tkIf:
		p.advance()
		if err := p.expect(tkLParen); err != nil {
			return nil, err
		}
		cond, err := p.parseExpr()
		if err != nil {
			return nil, err
		}
		if err := p.expect(tkRParen); err != nil {
			return nil, err
		}
		p.skipNLs()
		then, err := p.parseStmt()
		if err != nil {
			return nil, err
		}
		p.skipTerminators()
		var els awkStmt
		if p.current.kind == tkElse {
			p.advance()
			p.skipNLs()
			els, err = p.parseStmt()
			if err != nil {
				return nil, err
			}
		}
		return stIf{cond: cond, then: then, els: els}, nil
	case tkWhile:
		p.advance()
		if err := p.expect(tkLParen); err != nil {
			return nil, err
		}
		cond, err := p.parseExpr()
		if err != nil {
			return nil, err
		}
		if err := p.expect(tkRParen); err != nil {
			return nil, err
		}
		p.skipNLs()
		body, err := p.parseStmt()
		if err != nil {
			return nil, err
		}
		return stWhile{cond: cond, body: body}, nil
	case tkDo:
		p.advance()
		p.skipNLs()
		body, err := p.parseStmt()
		if err != nil {
			return nil, err
		}
		p.skipTerminators()
		p.advance() // while
		if err := p.expect(tkLParen); err != nil {
			return nil, err
		}
		cond, err := p.parseExpr()
		if err != nil {
			return nil, err
		}
		if err := p.expect(tkRParen); err != nil {
			return nil, err
		}
		return stDo{body: body, cond: cond}, nil
	case tkFor:
		p.advance()
		if err := p.expect(tkLParen); err != nil {
			return nil, err
		}
		// Detect for-in
		if p.current.kind == tkIdent {
			varName := p.current.sval
			p.advance()
			if p.current.kind == tkIn {
				p.advance()
				if p.current.kind != tkIdent {
					return nil, fmt.Errorf("expected array name at line %d", p.current.line)
				}
				arrName := p.current.sval
				p.advance()
				if err := p.expect(tkRParen); err != nil {
					return nil, err
				}
				p.skipNLs()
				body, err := p.parseStmt()
				if err != nil {
					return nil, err
				}
				return stForIn{varName: varName, arrName: arrName, body: body}, nil
			}
			// Finish expression starting with identifier
			initExpr, err := p.finishExprFromVar(varName)
			if err != nil {
				return nil, err
			}
			if err := p.expect(tkSemi); err != nil {
				return nil, err
			}
			var cond awkExpr
			if p.current.kind != tkSemi {
				cond, err = p.parseExpr()
				if err != nil {
					return nil, err
				}
			}
			if err := p.expect(tkSemi); err != nil {
				return nil, err
			}
			var post awkStmt
			if p.current.kind != tkRParen {
				postExpr, err := p.parseExpr()
				if err != nil {
					return nil, err
				}
				post = stExpr{e: postExpr}
			}
			if err := p.expect(tkRParen); err != nil {
				return nil, err
			}
			p.skipNLs()
			body, err := p.parseStmt()
			if err != nil {
				return nil, err
			}
			return stFor{init: stExpr{e: initExpr}, cond: cond, post: post, body: body}, nil
		}
		var init awkStmt
		var err error
		if p.current.kind != tkSemi {
			initExpr, err := p.parseExpr()
			if err != nil {
				return nil, err
			}
			init = stExpr{e: initExpr}
		}
		if err = p.expect(tkSemi); err != nil {
			return nil, err
		}
		var cond awkExpr
		if p.current.kind != tkSemi {
			cond, err = p.parseExpr()
			if err != nil {
				return nil, err
			}
		}
		if err = p.expect(tkSemi); err != nil {
			return nil, err
		}
		var post awkStmt
		if p.current.kind != tkRParen {
			postExpr, err := p.parseExpr()
			if err != nil {
				return nil, err
			}
			post = stExpr{e: postExpr}
		}
		if err = p.expect(tkRParen); err != nil {
			return nil, err
		}
		p.skipNLs()
		body, err := p.parseStmt()
		if err != nil {
			return nil, err
		}
		return stFor{init: init, cond: cond, post: post, body: body}, nil
	case tkBreak:
		p.advance()
		return stBreak{}, nil
	case tkContinue:
		p.advance()
		return stCont{}, nil
	case tkNext:
		p.advance()
		return stNext{}, nil
	case tkExit:
		p.advance()
		if p.isStmtEnd() {
			return stExit{}, nil
		}
		e, err := p.parseExpr()
		if err != nil {
			return nil, err
		}
		return stExit{e: e}, nil
	case tkReturn:
		p.advance()
		if p.isStmtEnd() {
			return stReturn{}, nil
		}
		e, err := p.parseExpr()
		if err != nil {
			return nil, err
		}
		return stReturn{e: e}, nil
	case tkDelete:
		p.advance()
		if p.current.kind != tkIdent {
			return nil, fmt.Errorf("expected array name after delete at line %d", ln)
		}
		name := p.current.sval
		p.advance()
		if p.current.kind == tkLBrack {
			p.advance()
			keys := []awkExpr{}
			k, err := p.parseExpr()
			if err != nil {
				return nil, err
			}
			keys = append(keys, k)
			for p.current.kind == tkComma {
				p.advance()
				k, err = p.parseExpr()
				if err != nil {
					return nil, err
				}
				keys = append(keys, k)
			}
			if err := p.expect(tkRBrack); err != nil {
				return nil, err
			}
			return stDelete{name: name, keys: keys}, nil
		}
		return stDelete{name: name}, nil
	case tkPrint, tkPrintf:
		isPrintf := p.current.kind == tkPrintf
		p.advance()
		hasParen := p.current.kind == tkLParen
		if hasParen {
			p.advance()
		}
		var args []awkExpr
		if !p.isStmtEnd() && p.current.kind != tkRParen {
			a, err := p.parseExpr()
			if err != nil {
				return nil, err
			}
			args = append(args, a)
			for p.current.kind == tkComma {
				p.advance()
				a, err = p.parseExpr()
				if err != nil {
					return nil, err
				}
				args = append(args, a)
			}
		}
		if hasParen && p.current.kind == tkRParen {
			p.advance()
		}
		redirect, err := p.parseRedirect()
		if err != nil {
			return nil, err
		}
		return stPrint{args: args, redirect: redirect, isPrintf: isPrintf}, nil
	case tkLBrace:
		return p.parseBlock()
	default:
		e, err := p.parseExpr()
		if err != nil {
			return nil, err
		}
		return stExpr{e: e}, nil
	}
}

func (p *awkParser) finishExprFromVar(varName string) (awkExpr, error) {
	lhs := exVar{name: varName}
	switch p.current.kind {
	case tkEq:
		p.advance()
		rhs, err := p.parseExpr()
		if err != nil {
			return nil, err
		}
		return exAssign{lhs: lhs, rhs: rhs}, nil
	case tkPlusEq:
		p.advance()
		rhs, err := p.parseExpr()
		if err != nil {
			return nil, err
		}
		return exCompound{op: "+", lhs: lhs, rhs: rhs}, nil
	case tkMinusEq:
		p.advance()
		rhs, err := p.parseExpr()
		if err != nil {
			return nil, err
		}
		return exCompound{op: "-", lhs: lhs, rhs: rhs}, nil
	case tkStarEq:
		p.advance()
		rhs, err := p.parseExpr()
		if err != nil {
			return nil, err
		}
		return exCompound{op: "*", lhs: lhs, rhs: rhs}, nil
	case tkSlashEq:
		p.advance()
		rhs, err := p.parseExpr()
		if err != nil {
			return nil, err
		}
		return exCompound{op: "/", lhs: lhs, rhs: rhs}, nil
	case tkPlusPlus:
		p.advance()
		return exPostInc{e: lhs, op: "++"}, nil
	case tkMinusMinus:
		p.advance()
		return exPostInc{e: lhs, op: "--"}, nil
	}
	return lhs, nil
}

func (p *awkParser) parseRedirect() (*awkRedirect, error) {
	switch p.current.kind {
	case tkGt:
		p.advance()
		e, err := p.parsePrimary()
		if err != nil {
			return nil, err
		}
		return &awkRedirect{kind: "file", expr: e}, nil
	case tkAppend:
		p.advance()
		e, err := p.parsePrimary()
		if err != nil {
			return nil, err
		}
		return &awkRedirect{kind: "append", expr: e}, nil
	case tkPipe:
		p.advance()
		e, err := p.parsePrimary()
		if err != nil {
			return nil, err
		}
		return &awkRedirect{kind: "pipe", expr: e}, nil
	}
	return nil, nil
}

// Expression parsing — precedence climbing
func (p *awkParser) parseExpr() (awkExpr, error)    { return p.parseTernary() }

func (p *awkParser) parseTernary() (awkExpr, error) {
	lhs, err := p.parseOr()
	if err != nil {
		return nil, err
	}
	if p.current.kind == tkQuestion {
		p.advance()
		t, err := p.parseTernary()
		if err != nil {
			return nil, err
		}
		if err := p.expect(tkColon); err != nil {
			return nil, err
		}
		f, err := p.parseTernary()
		if err != nil {
			return nil, err
		}
		return exTernary{cond: lhs, t: t, f: f}, nil
	}
	switch p.current.kind {
	case tkEq:
		p.advance()
		rhs, err := p.parseTernary()
		if err != nil {
			return nil, err
		}
		return exAssign{lhs: lhs, rhs: rhs}, nil
	case tkPlusEq:
		p.advance()
		rhs, err := p.parseTernary()
		if err != nil {
			return nil, err
		}
		return exCompound{op: "+", lhs: lhs, rhs: rhs}, nil
	case tkMinusEq:
		p.advance()
		rhs, err := p.parseTernary()
		if err != nil {
			return nil, err
		}
		return exCompound{op: "-", lhs: lhs, rhs: rhs}, nil
	case tkStarEq:
		p.advance()
		rhs, err := p.parseTernary()
		if err != nil {
			return nil, err
		}
		return exCompound{op: "*", lhs: lhs, rhs: rhs}, nil
	case tkSlashEq:
		p.advance()
		rhs, err := p.parseTernary()
		if err != nil {
			return nil, err
		}
		return exCompound{op: "/", lhs: lhs, rhs: rhs}, nil
	case tkPercentEq:
		p.advance()
		rhs, err := p.parseTernary()
		if err != nil {
			return nil, err
		}
		return exCompound{op: "%", lhs: lhs, rhs: rhs}, nil
	case tkCaretEq:
		p.advance()
		rhs, err := p.parseTernary()
		if err != nil {
			return nil, err
		}
		return exCompound{op: "^", lhs: lhs, rhs: rhs}, nil
	}
	return lhs, nil
}

func (p *awkParser) parseOr() (awkExpr, error) {
	lhs, err := p.parseAnd()
	if err != nil {
		return nil, err
	}
	for p.current.kind == tkOr {
		p.advance()
		rhs, err := p.parseAnd()
		if err != nil {
			return nil, err
		}
		lhs = exBinary{op: "||", l: lhs, r: rhs}
	}
	return lhs, nil
}

func (p *awkParser) parseAnd() (awkExpr, error) {
	lhs, err := p.parseMatchExpr()
	if err != nil {
		return nil, err
	}
	for p.current.kind == tkAnd {
		p.advance()
		rhs, err := p.parseMatchExpr()
		if err != nil {
			return nil, err
		}
		lhs = exBinary{op: "&&", l: lhs, r: rhs}
	}
	return lhs, nil
}

func (p *awkParser) parseMatchExpr() (awkExpr, error) {
	lhs, err := p.parseIn()
	if err != nil {
		return nil, err
	}
	for {
		switch p.current.kind {
		case tkTilde:
			p.advance()
			rhs, err := p.parseIn()
			if err != nil {
				return nil, err
			}
			lhs = exMatch{l: lhs, r: rhs}
		case tkBangTilde:
			p.advance()
			rhs, err := p.parseIn()
			if err != nil {
				return nil, err
			}
			lhs = exUnary{op: "!", e: exMatch{l: lhs, r: rhs}}
		default:
			return lhs, nil
		}
	}
}

func (p *awkParser) parseIn() (awkExpr, error) {
	lhs, err := p.parseCmp()
	if err != nil {
		return nil, err
	}
	for p.current.kind == tkIn {
		p.advance()
		if p.current.kind != tkIdent {
			return nil, fmt.Errorf("expected array name after 'in'")
		}
		arrName := p.current.sval
		p.advance()
		lhs = exInArray{key: lhs, arr: arrName}
	}
	return lhs, nil
}

func (p *awkParser) parseCmp() (awkExpr, error) {
	lhs, err := p.parseConcat()
	if err != nil {
		return nil, err
	}
	for {
		var op string
		switch p.current.kind {
		case tkLt:
			op = "<"
		case tkLtEq:
			op = "<="
		case tkGt:
			op = ">"
		case tkGtEq:
			op = ">="
		case tkEqEq:
			op = "=="
		case tkBangEq:
			op = "!="
		default:
			return lhs, nil
		}
		p.advance()
		rhs, err := p.parseConcat()
		if err != nil {
			return nil, err
		}
		lhs = exBinary{op: op, l: lhs, r: rhs}
	}
}

func (p *awkParser) isConcatStop() bool {
	switch p.current.kind {
	case tkNewline, tkSemi, tkRBrace, tkRParen, tkRBrack,
		tkComma, tkEOF, tkPipe, tkGt, tkAppend,
		tkQuestion, tkColon, tkEq, tkPlusEq, tkMinusEq, tkStarEq,
		tkSlashEq, tkPercentEq, tkCaretEq, tkAnd, tkOr,
		tkTilde, tkBangTilde, tkIn,
		tkLt, tkLtEq, tkGtEq, tkEqEq, tkBangEq:
		return true
	}
	return false
}

func (p *awkParser) parseConcat() (awkExpr, error) {
	parts := []awkExpr{}
	first, err := p.parseAdd()
	if err != nil {
		return nil, err
	}
	parts = append(parts, first)
	for !p.isConcatStop() {
		e, err := p.parseAdd()
		if err != nil {
			return nil, err
		}
		parts = append(parts, e)
	}
	if len(parts) == 1 {
		return parts[0], nil
	}
	return exConcat{parts: parts}, nil
}

func (p *awkParser) parseAdd() (awkExpr, error) {
	lhs, err := p.parseMul()
	if err != nil {
		return nil, err
	}
	for {
		switch p.current.kind {
		case tkPlus:
			p.advance()
			rhs, err := p.parseMul()
			if err != nil {
				return nil, err
			}
			lhs = exBinary{op: "+", l: lhs, r: rhs}
		case tkMinus:
			p.advance()
			rhs, err := p.parseMul()
			if err != nil {
				return nil, err
			}
			lhs = exBinary{op: "-", l: lhs, r: rhs}
		default:
			return lhs, nil
		}
	}
}

func (p *awkParser) parseMul() (awkExpr, error) {
	lhs, err := p.parsePow()
	if err != nil {
		return nil, err
	}
	for {
		switch p.current.kind {
		case tkStar:
			p.advance()
			rhs, err := p.parsePow()
			if err != nil {
				return nil, err
			}
			lhs = exBinary{op: "*", l: lhs, r: rhs}
		case tkSlash:
			p.advance()
			rhs, err := p.parsePow()
			if err != nil {
				return nil, err
			}
			lhs = exBinary{op: "/", l: lhs, r: rhs}
		case tkPercent:
			p.advance()
			rhs, err := p.parsePow()
			if err != nil {
				return nil, err
			}
			lhs = exBinary{op: "%", l: lhs, r: rhs}
		default:
			return lhs, nil
		}
	}
}

func (p *awkParser) parsePow() (awkExpr, error) {
	lhs, err := p.parseUnary()
	if err != nil {
		return nil, err
	}
	if p.current.kind == tkCaret {
		p.advance()
		rhs, err := p.parsePow()
		if err != nil {
			return nil, err
		}
		return exBinary{op: "^", l: lhs, r: rhs}, nil
	}
	return lhs, nil
}

func (p *awkParser) parseUnary() (awkExpr, error) {
	switch p.current.kind {
	case tkBang:
		p.advance()
		e, err := p.parseUnary()
		if err != nil {
			return nil, err
		}
		return exUnary{op: "!", e: e}, nil
	case tkMinus:
		p.advance()
		e, err := p.parseUnary()
		if err != nil {
			return nil, err
		}
		return exUnary{op: "-", e: e}, nil
	case tkPlus:
		p.advance()
		e, err := p.parseUnary()
		if err != nil {
			return nil, err
		}
		return exUnary{op: "+", e: e}, nil
	case tkPlusPlus:
		p.advance()
		e, err := p.parseUnary()
		if err != nil {
			return nil, err
		}
		return exPreInc{op: "++", e: e}, nil
	case tkMinusMinus:
		p.advance()
		e, err := p.parseUnary()
		if err != nil {
			return nil, err
		}
		return exPreInc{op: "--", e: e}, nil
	}
	return p.parsePostfix()
}

func (p *awkParser) parsePostfix() (awkExpr, error) {
	e, err := p.parsePrimary()
	if err != nil {
		return nil, err
	}
	switch p.current.kind {
	case tkPlusPlus:
		p.advance()
		return exPostInc{e: e, op: "++"}, nil
	case tkMinusMinus:
		p.advance()
		return exPostInc{e: e, op: "--"}, nil
	}
	return e, nil
}

func (p *awkParser) parsePrimary() (awkExpr, error) {
	ln := p.current.line
	switch p.current.kind {
	case tkNum:
		n := p.current.nval
		p.advance()
		return exNum{n: n}, nil
	case tkStr:
		s := p.current.sval
		p.advance()
		return exStr{s: s}, nil
	case tkDollar:
		p.advance()
		e, err := p.parseUnary()
		if err != nil {
			return nil, err
		}
		return exField{idx: e}, nil
	case tkLParen:
		p.advance()
		e, err := p.parseExpr()
		if err != nil {
			return nil, err
		}
		if err := p.expect(tkRParen); err != nil {
			return nil, err
		}
		return e, nil
	case tkBang:
		p.advance()
		e, err := p.parsePrimary()
		if err != nil {
			return nil, err
		}
		return exUnary{op: "!", e: e}, nil
	case tkSlash:
		// Regex literal: re-lex
		p.advance()
		tok, err := p.lexer.lexRegex()
		if err != nil {
			return nil, err
		}
		p.current, err = p.lexer.next()
		if err != nil {
			return nil, err
		}
		return exRegex{pat: tok.sval}, nil
	case tkGetline:
		p.advance()
		if p.current.kind == tkIdent {
			v := p.current.sval
			p.advance()
			return exGetline{target: exVar{name: v}}, nil
		}
		return exGetline{}, nil
	case tkIdent:
		name := p.current.sval
		p.advance()
		if p.current.kind == tkLBrack {
			p.advance()
			keys := []awkExpr{}
			k, err := p.parseExpr()
			if err != nil {
				return nil, err
			}
			keys = append(keys, k)
			for p.current.kind == tkComma {
				p.advance()
				k, err = p.parseExpr()
				if err != nil {
					return nil, err
				}
				keys = append(keys, k)
			}
			if err := p.expect(tkRBrack); err != nil {
				return nil, err
			}
			return exArray{name: name, keys: keys}, nil
		}
		if p.current.kind == tkLParen {
			p.advance()
			var args []awkExpr
			if p.current.kind != tkRParen {
				a, err := p.parseExpr()
				if err != nil {
					return nil, err
				}
				args = append(args, a)
				for p.current.kind == tkComma {
					p.advance()
					a, err = p.parseExpr()
					if err != nil {
						return nil, err
					}
					args = append(args, a)
				}
			}
			if err := p.expect(tkRParen); err != nil {
				return nil, err
			}
			return exCall{name: name, args: args}, nil
		}
		return exVar{name: name}, nil
	}
	return nil, fmt.Errorf("unexpected token %d at line %d", p.current.kind, ln)
}

// ─── Control flow signals ─────────────────────────────────────────────────────

type awkSignal int

const (
	sigNone awkSignal = iota
	sigBreak
	sigContinue
	sigNext
	sigExit
	sigReturn
)

type execResult struct {
	signal    awkSignal
	exitValue AWKVal
	retValue  AWKVal
}

// ─── Interpreter ─────────────────────────────────────────────────────────────

type awkInterp struct {
	globals      map[string]AWKVal
	arrays       map[string]map[string]AWKVal
	fields       []string
	record       string
	fieldSep     string
	ofs          string
	ors          string
	nr           int
	nf           int
	filename     string
	rangeActive  map[int]bool
}

func newAWKInterpreter(fs string) *awkInterp {
	return &awkInterp{
		globals:     make(map[string]AWKVal),
		arrays:      make(map[string]map[string]AWKVal),
		fieldSep:    fs,
		ofs:         " ",
		ors:         "\n",
		rangeActive: make(map[int]bool),
	}
}

func (interp *awkInterp) splitRecord(line string) {
	interp.record = line
	if interp.fieldSep == " " {
		interp.fields = strings.Fields(line)
	} else if len(interp.fieldSep) == 1 {
		interp.fields = strings.Split(line, interp.fieldSep)
	} else {
		re, err := regexp.Compile(interp.fieldSep)
		if err != nil {
			interp.fields = []string{line}
		} else {
			interp.fields = re.Split(line, -1)
		}
	}
	interp.nf = len(interp.fields)
}

func (interp *awkInterp) getField(idx int) AWKVal {
	if idx == 0 {
		return awkStr(interp.record)
	}
	if idx < 1 || idx > len(interp.fields) {
		return awkNil
	}
	return awkStr(interp.fields[idx-1])
}

func (interp *awkInterp) setField(idx int, val AWKVal) {
	if idx == 0 {
		interp.record = val.str()
		interp.splitRecord(interp.record)
		return
	}
	for len(interp.fields) < idx {
		interp.fields = append(interp.fields, "")
	}
	interp.fields[idx-1] = val.str()
	interp.nf = len(interp.fields)
	interp.record = strings.Join(interp.fields, interp.ofs)
}

func (interp *awkInterp) getVar(name string, locals map[string]AWKVal) AWKVal {
	switch name {
	case "NR":
		return awkNum(float64(interp.nr))
	case "NF":
		return awkNum(float64(interp.nf))
	case "FS":
		return awkStr(interp.fieldSep)
	case "OFS":
		return awkStr(interp.ofs)
	case "ORS":
		return awkStr(interp.ors)
	case "FILENAME":
		return awkStr(interp.filename)
	}
	if v, ok := locals[name]; ok {
		return v
	}
	if v, ok := interp.globals[name]; ok {
		return v
	}
	return awkNil
}

func (interp *awkInterp) setVar(name string, val AWKVal, locals map[string]AWKVal) {
	switch name {
	case "FS":
		interp.fieldSep = val.str()
	case "OFS":
		interp.ofs = val.str()
	case "ORS":
		interp.ors = val.str()
	default:
		if _, ok := locals[name]; ok {
			locals[name] = val
		} else {
			interp.globals[name] = val
		}
	}
}

func (interp *awkInterp) arrayKey(keys []awkExpr, locals map[string]AWKVal, prog *awkProgram) string {
	parts := make([]string, len(keys))
	for i, k := range keys {
		v, _ := interp.eval(k, locals, prog)
		parts[i] = v.str()
	}
	return strings.Join(parts, "\x1C")
}

func (interp *awkInterp) runLines(prog *awkProgram, lines []string) {
	locals := make(map[string]AWKVal)

	// BEGIN
	for _, rule := range prog.rules {
		if _, ok := rule.pattern.(patBegin); ok {
			res := interp.execStmt(rule.action, locals, prog)
			if res.signal == sigExit {
				os.Exit(int(res.exitValue.num()))
			}
		}
	}

	// Records
	for _, line := range lines {
		interp.nr++
		interp.splitRecord(line)
	ruleLoop:
		for i, rule := range prog.rules {
			var matched bool
			switch pat := rule.pattern.(type) {
			case patBegin, patEnd:
				continue
			case patAlways:
				matched = true
			case patExpr:
				v, _ := interp.eval(pat.e, locals, prog)
				matched = v.bool()
			case patRange:
				active := interp.rangeActive[i]
				if !active {
					v, _ := interp.eval(pat.start, locals, prog)
					if v.bool() {
						interp.rangeActive[i] = true
						matched = true
					}
				} else {
					matched = true
					v, _ := interp.eval(pat.end, locals, prog)
					if v.bool() {
						interp.rangeActive[i] = false
					}
				}
			}
			if matched {
				res := interp.execStmt(rule.action, locals, prog)
				if res.signal == sigNext {
					break ruleLoop
				}
				if res.signal == sigExit {
					os.Exit(int(res.exitValue.num()))
				}
			}
		}
	}

	// END
	for _, rule := range prog.rules {
		if _, ok := rule.pattern.(patEnd); ok {
			res := interp.execStmt(rule.action, locals, prog)
			if res.signal == sigExit {
				os.Exit(int(res.exitValue.num()))
			}
		}
	}
}

func (interp *awkInterp) execStmt(stmt awkStmt, locals map[string]AWKVal, prog *awkProgram) execResult {
	switch s := stmt.(type) {
	case stEmpty:
	case stBlock:
		for _, sub := range s.stmts {
			res := interp.execStmt(sub, locals, prog)
			if res.signal != sigNone {
				return res
			}
		}
	case stExpr:
		interp.eval(s.e, locals, prog)
	case stPrint:
		var out string
		if len(s.args) == 0 {
			out = interp.record
		} else {
			parts := make([]string, len(s.args))
			for i, a := range s.args {
				v, _ := interp.eval(a, locals, prog)
				parts[i] = v.str()
			}
			out = strings.Join(parts, interp.ofs)
		}
		text := out + interp.ors
		if s.isPrintf && len(s.args) > 0 {
			fmtVal, _ := interp.eval(s.args[0], locals, prog)
			rest := make([]AWKVal, 0, len(s.args)-1)
			for _, a := range s.args[1:] {
				v, _ := interp.eval(a, locals, prog)
				rest = append(rest, v)
			}
			text = awkSprintf(fmtVal.str(), rest)
		}
		interp.writeOutput(text, s.redirect, locals, prog)
	case stIf:
		cond, _ := interp.eval(s.cond, locals, prog)
		if cond.bool() {
			return interp.execStmt(s.then, locals, prog)
		} else if s.els != nil {
			return interp.execStmt(s.els, locals, prog)
		}
	case stWhile:
		for {
			cond, _ := interp.eval(s.cond, locals, prog)
			if !cond.bool() {
				break
			}
			res := interp.execStmt(s.body, locals, prog)
			if res.signal == sigBreak {
				break
			}
			if res.signal == sigContinue {
				continue
			}
			if res.signal != sigNone {
				return res
			}
		}
	case stDo:
		for {
			res := interp.execStmt(s.body, locals, prog)
			if res.signal == sigBreak {
				break
			}
			if res.signal != sigNone && res.signal != sigContinue {
				return res
			}
			cond, _ := interp.eval(s.cond, locals, prog)
			if !cond.bool() {
				break
			}
		}
	case stFor:
		if s.init != nil {
			interp.execStmt(s.init, locals, prog)
		}
		for {
			if s.cond != nil {
				v, _ := interp.eval(s.cond, locals, prog)
				if !v.bool() {
					break
				}
			}
			res := interp.execStmt(s.body, locals, prog)
			if res.signal == sigBreak {
				break
			}
			if res.signal != sigNone && res.signal != sigContinue {
				return res
			}
			if s.post != nil {
				interp.execStmt(s.post, locals, prog)
			}
		}
	case stForIn:
		arr := interp.arrays[s.arrName]
		keys := make([]string, 0, len(arr))
		for k := range arr {
			keys = append(keys, k)
		}
		for _, k := range keys {
			interp.setVar(s.varName, awkStr(k), locals)
			res := interp.execStmt(s.body, locals, prog)
			if res.signal == sigBreak {
				break
			}
			if res.signal == sigContinue {
				continue
			}
			if res.signal != sigNone {
				return res
			}
		}
	case stBreak:
		return execResult{signal: sigBreak}
	case stCont:
		return execResult{signal: sigContinue}
	case stNext:
		return execResult{signal: sigNext}
	case stExit:
		var v AWKVal
		if s.e != nil {
			v, _ = interp.eval(s.e, locals, prog)
		}
		return execResult{signal: sigExit, exitValue: v}
	case stReturn:
		var v AWKVal
		if s.e != nil {
			v, _ = interp.eval(s.e, locals, prog)
		}
		return execResult{signal: sigReturn, retValue: v}
	case stDelete:
		if len(s.keys) > 0 {
			key := interp.arrayKey(s.keys, locals, prog)
			if arr, ok := interp.arrays[s.name]; ok {
				delete(arr, key)
			}
		} else {
			interp.arrays[s.name] = make(map[string]AWKVal)
		}
	}
	return execResult{}
}

func (interp *awkInterp) eval(expr awkExpr, locals map[string]AWKVal, prog *awkProgram) (AWKVal, error) {
	switch e := expr.(type) {
	case exNum:
		return awkNum(e.n), nil
	case exStr:
		return awkStr(e.s), nil
	case exRegex:
		if awkMatchRegex(interp.record, e.pat) {
			return awkNum(1), nil
		}
		return awkNum(0), nil
	case exField:
		iv, _ := interp.eval(e.idx, locals, prog)
		return interp.getField(int(iv.num())), nil
	case exVar:
		return interp.getVar(e.name, locals), nil
	case exArray:
		key := interp.arrayKey(e.keys, locals, prog)
		if arr, ok := interp.arrays[e.name]; ok {
			if v, ok := arr[key]; ok {
				return v, nil
			}
		}
		return awkNil, nil
	case exAssign:
		val, _ := interp.eval(e.rhs, locals, prog)
		interp.assignTo(e.lhs, val, locals, prog)
		return val, nil
	case exCompound:
		cur, _ := interp.eval(e.lhs, locals, prog)
		rhs, _ := interp.eval(e.rhs, locals, prog)
		result := awkApplyOp(e.op, cur, rhs)
		interp.assignTo(e.lhs, result, locals, prog)
		return result, nil
	case exPreInc:
		cur, _ := interp.eval(e.e, locals, prog)
		var next AWKVal
		if e.op == "++" {
			next = awkNum(cur.num() + 1)
		} else {
			next = awkNum(cur.num() - 1)
		}
		interp.assignTo(e.e, next, locals, prog)
		return next, nil
	case exPostInc:
		cur, _ := interp.eval(e.e, locals, prog)
		var next AWKVal
		if e.op == "++" {
			next = awkNum(cur.num() + 1)
		} else {
			next = awkNum(cur.num() - 1)
		}
		interp.assignTo(e.e, next, locals, prog)
		return cur, nil
	case exUnary:
		v, _ := interp.eval(e.e, locals, prog)
		switch e.op {
		case "-":
			return awkNum(-v.num()), nil
		case "+":
			return awkNum(v.num()), nil
		case "!":
			if v.bool() {
				return awkNum(0), nil
			}
			return awkNum(1), nil
		}
	case exBinary:
		if e.op == "&&" {
			l, _ := interp.eval(e.l, locals, prog)
			if !l.bool() {
				return awkNum(0), nil
			}
			r, _ := interp.eval(e.r, locals, prog)
			if r.bool() {
				return awkNum(1), nil
			}
			return awkNum(0), nil
		}
		if e.op == "||" {
			l, _ := interp.eval(e.l, locals, prog)
			if l.bool() {
				return awkNum(1), nil
			}
			r, _ := interp.eval(e.r, locals, prog)
			if r.bool() {
				return awkNum(1), nil
			}
			return awkNum(0), nil
		}
		l, _ := interp.eval(e.l, locals, prog)
		r, _ := interp.eval(e.r, locals, prog)
		switch e.op {
		case "+", "-", "*", "/", "%", "^":
			return awkApplyOp(e.op, l, r), nil
		case "<":
			if awkCompare(l, r) < 0 {
				return awkNum(1), nil
			}
			return awkNum(0), nil
		case "<=":
			if awkCompare(l, r) <= 0 {
				return awkNum(1), nil
			}
			return awkNum(0), nil
		case ">":
			if awkCompare(l, r) > 0 {
				return awkNum(1), nil
			}
			return awkNum(0), nil
		case ">=":
			if awkCompare(l, r) >= 0 {
				return awkNum(1), nil
			}
			return awkNum(0), nil
		case "==":
			if awkCompare(l, r) == 0 {
				return awkNum(1), nil
			}
			return awkNum(0), nil
		case "!=":
			if awkCompare(l, r) != 0 {
				return awkNum(1), nil
			}
			return awkNum(0), nil
		}
	case exTernary:
		cond, _ := interp.eval(e.cond, locals, prog)
		if cond.bool() {
			return interp.eval(e.t, locals, prog)
		}
		return interp.eval(e.f, locals, prog)
	case exMatch:
		l, _ := interp.eval(e.l, locals, prog)
		var pat string
		if rx, ok := e.r.(exRegex); ok {
			pat = rx.pat
		} else {
			rv, _ := interp.eval(e.r, locals, prog)
			pat = rv.str()
		}
		if awkMatchRegex(l.str(), pat) {
			return awkNum(1), nil
		}
		return awkNum(0), nil
	case exInArray:
		key, _ := interp.eval(e.key, locals, prog)
		if arr, ok := interp.arrays[e.arr]; ok {
			if _, ok := arr[key.str()]; ok {
				return awkNum(1), nil
			}
		}
		return awkNum(0), nil
	case exConcat:
		var buf strings.Builder
		for _, p := range e.parts {
			v, _ := interp.eval(p, locals, prog)
			buf.WriteString(v.str())
		}
		return awkStr(buf.String()), nil
	case exGetline:
		scanner := bufio.NewScanner(os.Stdin)
		if scanner.Scan() {
			line := scanner.Text()
			if e.target != nil {
				interp.assignTo(e.target, awkStr(line), locals, prog)
			} else {
				interp.nr++
				interp.splitRecord(line)
			}
			return awkNum(1), nil
		}
		return awkNum(-1), nil
	case exCall:
		return interp.callFunction(e.name, e.args, locals, prog)
	}
	return awkNil, nil
}

func (interp *awkInterp) assignTo(expr awkExpr, val AWKVal, locals map[string]AWKVal, prog *awkProgram) {
	switch e := expr.(type) {
	case exVar:
		interp.setVar(e.name, val, locals)
	case exField:
		iv, _ := interp.eval(e.idx, locals, prog)
		interp.setField(int(iv.num()), val)
	case exArray:
		key := interp.arrayKey(e.keys, locals, prog)
		if _, ok := interp.arrays[e.name]; !ok {
			interp.arrays[e.name] = make(map[string]AWKVal)
		}
		interp.arrays[e.name][key] = val
	}
}

func (interp *awkInterp) callFunction(name string, argExprs []awkExpr, locals map[string]AWKVal, prog *awkProgram) (AWKVal, error) {
	// User-defined function
	if fn, ok := prog.functions[name]; ok {
		frame := make(map[string]AWKVal)
		for i, param := range fn.params {
			if i < len(argExprs) {
				v, _ := interp.eval(argExprs[i], locals, prog)
				frame[param] = v
			} else {
				frame[param] = awkNil
			}
		}
		res := interp.execStmt(fn.body, frame, prog)
		if res.signal == sigReturn {
			return res.retValue, nil
		}
		return awkNil, nil
	}

	evalArgs := func() []AWKVal {
		args := make([]AWKVal, len(argExprs))
		for i, a := range argExprs {
			args[i], _ = interp.eval(a, locals, prog)
		}
		return args
	}

	switch name {
	case "length":
		if len(argExprs) == 0 {
			return awkNum(float64(len([]rune(interp.record)))), nil
		}
		if v, ok := argExprs[0].(exVar); ok {
			if arr, ok := interp.arrays[v.name]; ok {
				return awkNum(float64(len(arr))), nil
			}
		}
		a, _ := interp.eval(argExprs[0], locals, prog)
		return awkNum(float64(len([]rune(a.str())))), nil
	case "substr":
		args := evalArgs()
		s := []rune(args[0].str())
		start := int(args[1].num()) - 1
		if start < 0 {
			start = 0
		}
		if len(args) >= 3 {
			length := int(args[2].num())
			end := start + length
			if end > len(s) {
				end = len(s)
			}
			if start > len(s) {
				return awkStr(""), nil
			}
			return awkStr(string(s[start:end])), nil
		}
		if start > len(s) {
			return awkStr(""), nil
		}
		return awkStr(string(s[start:])), nil
	case "index":
		args := evalArgs()
		haystack := args[0].str()
		needle := args[1].str()
		idx := strings.Index(haystack, needle)
		if idx < 0 {
			return awkNum(0), nil
		}
		return awkNum(float64(len([]rune(haystack[:idx])) + 1)), nil
	case "split":
		if len(argExprs) < 2 {
			return awkNum(0), nil
		}
		s, _ := interp.eval(argExprs[0], locals, prog)
		arrVar, ok := argExprs[1].(exVar)
		if !ok {
			return awkNum(0), nil
		}
		var sep string
		if len(argExprs) >= 3 {
			sv, _ := interp.eval(argExprs[2], locals, prog)
			sep = sv.str()
		} else {
			sep = interp.fieldSep
		}
		var parts []string
		if sep == " " {
			parts = strings.Fields(s.str())
		} else {
			parts = strings.Split(s.str(), sep)
		}
		arr := make(map[string]AWKVal)
		for i, p := range parts {
			arr[strconv.Itoa(i+1)] = awkStr(p)
		}
		interp.arrays[arrVar.name] = arr
		return awkNum(float64(len(parts))), nil
	case "sub", "gsub":
		isGlobal := name == "gsub"
		if len(argExprs) < 2 {
			return awkNum(0), nil
		}
		var pat string
		if rx, ok := argExprs[0].(exRegex); ok {
			pat = rx.pat
		} else {
			pv, _ := interp.eval(argExprs[0], locals, prog)
			pat = pv.str()
		}
		rv, _ := interp.eval(argExprs[1], locals, prog)
		repl := rv.str()
		var targetExpr awkExpr
		if len(argExprs) >= 3 {
			targetExpr = argExprs[2]
		} else {
			targetExpr = exField{idx: exNum{n: 0}}
		}
		sv, _ := interp.eval(targetExpr, locals, prog)
		s := sv.str()
		count := 0
		re, err := regexp.Compile(pat)
		if err == nil {
			if isGlobal {
				count = len(re.FindAllString(s, -1))
				s = re.ReplaceAllStringFunc(s, func(m string) string {
					return strings.ReplaceAll(repl, "&", m)
				})
			} else {
				loc := re.FindStringIndex(s)
				if loc != nil {
					m := s[loc[0]:loc[1]]
					s = s[:loc[0]] + strings.ReplaceAll(repl, "&", m) + s[loc[1]:]
					count = 1
				}
			}
		}
		interp.assignTo(targetExpr, awkStr(s), locals, prog)
		return awkNum(float64(count)), nil
	case "match":
		args := evalArgs()
		s := args[0].str()
		pat := args[1].str()
		re, err := regexp.Compile(pat)
		if err != nil {
			interp.globals["RSTART"] = awkNum(0)
			interp.globals["RLENGTH"] = awkNum(-1)
			return awkNum(0), nil
		}
		loc := re.FindStringIndex(s)
		if loc == nil {
			interp.globals["RSTART"] = awkNum(0)
			interp.globals["RLENGTH"] = awkNum(-1)
			return awkNum(0), nil
		}
		start := len([]rune(s[:loc[0]])) + 1
		length := len([]rune(s[loc[0]:loc[1]]))
		interp.globals["RSTART"] = awkNum(float64(start))
		interp.globals["RLENGTH"] = awkNum(float64(length))
		return awkNum(float64(start)), nil
	case "sprintf":
		args := evalArgs()
		if len(args) == 0 {
			return awkStr(""), nil
		}
		return awkStr(awkSprintf(args[0].str(), args[1:])), nil
	case "int":
		args := evalArgs()
		return awkNum(float64(int64(args[0].num()))), nil
	case "sqrt":
		args := evalArgs()
		return awkNum(math.Sqrt(args[0].num())), nil
	case "log":
		args := evalArgs()
		return awkNum(math.Log(args[0].num())), nil
	case "exp":
		args := evalArgs()
		return awkNum(math.Exp(args[0].num())), nil
	case "sin":
		args := evalArgs()
		return awkNum(math.Sin(args[0].num())), nil
	case "cos":
		args := evalArgs()
		return awkNum(math.Cos(args[0].num())), nil
	case "atan2":
		args := evalArgs()
		return awkNum(math.Atan2(args[0].num(), args[1].num())), nil
	case "rand":
		return awkNum(rand.Float64()), nil
	case "srand":
		if len(argExprs) > 0 {
			args := evalArgs()
			rand.Seed(int64(args[0].num()))
		}
		return awkNum(0), nil
	case "tolower":
		args := evalArgs()
		return awkStr(strings.ToLower(args[0].str())), nil
	case "toupper":
		args := evalArgs()
		return awkStr(strings.ToUpper(args[0].str())), nil
	case "system":
		args := evalArgs()
		cmd := exec.Command("sh", "-c", args[0].str())
		cmd.Stdout = os.Stdout
		cmd.Stderr = os.Stderr
		err := cmd.Run()
		if err != nil {
			return awkNum(-1), nil
		}
		return awkNum(float64(cmd.ProcessState.ExitCode())), nil
	default:
		fmt.Fprintf(os.Stderr, "awk: undefined function '%s'\n", name)
		return awkNil, nil
	}
}

func (interp *awkInterp) writeOutput(text string, redirect *awkRedirect, locals map[string]AWKVal, prog *awkProgram) {
	if redirect == nil {
		fmt.Print(text)
		return
	}
	pathVal, _ := interp.eval(redirect.expr, locals, prog)
	path := pathVal.str()
	switch redirect.kind {
	case "file":
		f, err := os.OpenFile(path, os.O_WRONLY|os.O_CREATE|os.O_TRUNC, 0644)
		if err == nil {
			fmt.Fprint(f, text)
			f.Close()
		}
	case "append":
		f, err := os.OpenFile(path, os.O_WRONLY|os.O_CREATE|os.O_APPEND, 0644)
		if err == nil {
			fmt.Fprint(f, text)
			f.Close()
		}
	case "pipe":
		cmd := exec.Command("sh", "-c", path)
		cmd.Stdin = strings.NewReader(text)
		cmd.Stdout = os.Stdout
		cmd.Stderr = os.Stderr
		cmd.Run()
	}
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

func awkApplyOp(op string, a, b AWKVal) AWKVal {
	x, y := a.num(), b.num()
	switch op {
	case "+":
		return awkNum(x + y)
	case "-":
		return awkNum(x - y)
	case "*":
		return awkNum(x * y)
	case "/":
		if y == 0 {
			fmt.Fprintln(os.Stderr, "awk: division by zero")
			return awkNum(0)
		}
		return awkNum(x / y)
	case "%":
		if y == 0 {
			fmt.Fprintln(os.Stderr, "awk: modulo by zero")
			return awkNum(0)
		}
		return awkNum(math.Mod(x, y))
	case "^":
		return awkNum(math.Pow(x, y))
	}
	return awkNum(0)
}

func awkMatchRegex(text, pattern string) bool {
	re, err := regexp.Compile(pattern)
	if err != nil {
		return strings.Contains(text, pattern)
	}
	return re.MatchString(text)
}

// ─── sprintf ─────────────────────────────────────────────────────────────────

func awkSprintf(format string, args []AWKVal) string {
	runes := []rune(format)
	var buf strings.Builder
	i := 0
	argIdx := 0

	for i < len(runes) {
		if runes[i] != '%' {
			buf.WriteRune(runes[i])
			i++
			continue
		}
		i++
		if i >= len(runes) {
			break
		}
		if runes[i] == '%' {
			buf.WriteRune('%')
			i++
			continue
		}

		// Flags
		var flags string
		for i < len(runes) && strings.ContainsRune("-+ #0", runes[i]) {
			flags += string(runes[i])
			i++
		}
		// Width
		var widthS string
		for i < len(runes) && unicode.IsDigit(runes[i]) {
			widthS += string(runes[i])
			i++
		}
		// Precision
		precS := "-1"
		if i < len(runes) && runes[i] == '.' {
			i++
			precS = ""
			for i < len(runes) && unicode.IsDigit(runes[i]) {
				precS += string(runes[i])
				i++
			}
		}
		if i >= len(runes) {
			break
		}
		spec := runes[i]
		i++

		var arg AWKVal
		if argIdx < len(args) {
			arg = args[argIdx]
		} else {
			arg = awkNil
		}
		argIdx++

		width, _ := strconv.Atoi(widthS)
		prec, _ := strconv.Atoi(precS)
		leftAlign := strings.Contains(flags, "-")
		zeroPad := strings.Contains(flags, "0")
		plus := strings.Contains(flags, "+")

		pad := func(s string, w int) string {
			if len(s) >= w {
				return s
			}
			p := w - len(s)
			padChar := " "
			if zeroPad && !leftAlign {
				padChar = "0"
			}
			if leftAlign {
				return s + strings.Repeat(" ", p)
			}
			return strings.Repeat(padChar, p) + s
		}

		switch spec {
		case 'd', 'i':
			n := int64(arg.num())
			s := strconv.FormatInt(n, 10)
			if plus && n >= 0 {
				s = "+" + s
			}
			buf.WriteString(pad(s, width))
		case 'u':
			buf.WriteString(pad(strconv.FormatUint(uint64(arg.num()), 10), width))
		case 'o':
			buf.WriteString(pad(strconv.FormatUint(uint64(arg.num()), 8), width))
		case 'x':
			buf.WriteString(pad(strconv.FormatUint(uint64(arg.num()), 16), width))
		case 'X':
			buf.WriteString(pad(strings.ToUpper(strconv.FormatUint(uint64(arg.num()), 16)), width))
		case 'f':
			p := 6
			if precS != "-1" {
				p = prec
			}
			buf.WriteString(pad(strconv.FormatFloat(arg.num(), 'f', p, 64), width))
		case 'e':
			p := 6
			if precS != "-1" {
				p = prec
			}
			buf.WriteString(pad(strconv.FormatFloat(arg.num(), 'e', p, 64), width))
		case 'E':
			p := 6
			if precS != "-1" {
				p = prec
			}
			buf.WriteString(pad(strings.ToUpper(strconv.FormatFloat(arg.num(), 'e', p, 64)), width))
		case 'g', 'G':
			p := 6
			if precS != "-1" {
				p = prec
			}
			s := strconv.FormatFloat(arg.num(), 'g', p, 64)
			if spec == 'G' {
				s = strings.ToUpper(s)
			}
			buf.WriteString(pad(s, width))
		case 's':
			s := arg.str()
			if precS != "-1" && prec >= 0 && prec < len([]rune(s)) {
				s = string([]rune(s)[:prec])
			}
			buf.WriteString(pad(s, width))
		case 'c':
			var s string
			if arg.isNum {
				r := rune(arg.num())
				s = string(r)
			} else if len(arg.s) > 0 {
				s = string([]rune(arg.s)[0:1])
			}
			buf.WriteString(pad(s, width))
		default:
			buf.WriteRune('%')
			buf.WriteRune(spec)
		}
	}
	return buf.String()
}

// ─── Help / version ──────────────────────────────────────────────────────────

func awkPrintUsage() {
	fmt.Println("Usage: awk [OPTION]... 'program' [FILE]...")
	fmt.Println("   or: awk [OPTION]... -f progfile [FILE]...")
	fmt.Println("Scan and process patterns in each FILE (or standard input).")
	fmt.Println("\nOptions:")
	fmt.Println("  -F fs       use fs as the input field separator (FS)")
	fmt.Println("  -v var=val  assign value val to variable var before execution")
	fmt.Println("  -f file     read program text from file")
	fmt.Println("  --help      display this help and exit")
	fmt.Println("  --version   output version information and exit")
	fmt.Println("\nA program consists of rules: /pattern/ { action }")
	fmt.Println("Special patterns: BEGIN { ... }  and  END { ... }")
	fmt.Println("\nVariables:   NR  NF  FS  OFS  ORS  RS  FILENAME")
	fmt.Println("Functions:   length  substr  index  split  sub  gsub  match")
	fmt.Println("             sprintf  int  sqrt  log  exp  sin  cos  atan2")
	fmt.Println("             rand  srand  tolower  toupper  system")
}

func awkPrintVersion() {
	fmt.Println("awk (cacutils) v1.0")
	fmt.Println("IEEE Std 1003.1-2008 (POSIX) compatible implementation.")
	fmt.Println("There is NO WARRANTY, to the extent permitted by law.")
	fmt.Println("Written by Cyril John Magayaga.")
}
