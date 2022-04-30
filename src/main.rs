use std::io::{BufRead, Write};
use std::str::FromStr;
use std::{env, fs, io, str};

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut app = App::new();

    match &args[..] {
        [_] => app.run_prompt(),
        [_, path] => app.run_file(path),
        _ => {
            println!("Usage: rlox-interpreter [script]");
            std::process::exit(64);
        }
    }
}

struct App {
    had_error: bool,
}

impl App {
    fn new() -> App {
        return App { had_error: false };
    }

    fn error(&mut self, line: u64, message: &str) {
        self.report(line, "", message);
    }

    fn report(&mut self, line: u64, origin: &str, message: &str) {
        self.had_error = true;
        eprintln!("[line {}] Error{}: {}", line, origin, message);
    }

    fn run_file(&mut self, path: &str) {
        match fs::read_to_string(path) {
            Ok(content) => {
                println!("{}", content.len());
                self.run(&content);
                if self.had_error {
                    std::process::exit(65);
                }
            }
            _ => {
                println!("Error: could not open file {}", path);
                std::process::exit(66);
            }
        }
    }

    fn run_prompt(&mut self) {
        let mut line = String::with_capacity(1024);
        let stdin = io::stdin();
        let mut handle = stdin.lock();

        loop {
            print!("> ");
            io::stdout().flush().expect("Could not flush stdout");

            line.clear();
            match handle.read_line(&mut line) {
                Ok(bytes_read) => {
                    if bytes_read == 0 {
                        // we reached EOF (user probably pressed Ctrl+D)
                        std::process::exit(0);
                    }
                    self.run(&line);
                    self.had_error = false;
                }
                Err(error) => {
                    println!("Error: {}", error);
                    std::process::exit(70);
                }
            }
        }
    }

    fn run(&mut self, source: &str) {
        let mut scanner = Scanner::new(self, source.as_bytes());
        let tokens = scanner.scan_tokens();
        dbg!(tokens);
    }
}

struct Scanner<'a> {
    source: &'a [u8],
    line: u64,
    start: usize,
    current: usize,
    tokens: Vec<Token>,
    app: &'a mut App,
}

impl Scanner<'_> {
    fn new<'a>(app: &'a mut App, source: &'a [u8]) -> Scanner<'a> {
        return Scanner {
            source,
            line: 1,
            start: 0,
            current: 0,
            tokens: vec![],
            app,
        };
    }

    pub fn scan_tokens(&mut self) -> Vec<Token> {
        self.tokens.clear();

        while !self.is_at_end() {
            self.start = self.current;
            self.scan_token();
        }

        self.tokens.push(Token {
            token_type: TokenType::Eof,
            lexeme: "".to_string(),
            literal: TokenLiteral::None,
            line: self.line,
        });

        self.tokens.clone()
    }

    fn scan_token(&mut self) {
        let c = self.advance();
        match c {
            b'(' => self.add_token(TokenType::LeftParen),
            b')' => self.add_token(TokenType::RightParen),
            b'{' => self.add_token(TokenType::LeftBrace),
            b'}' => self.add_token(TokenType::RightBrace),
            b',' => self.add_token(TokenType::Comma),
            b'.' => self.add_token(TokenType::Dot),
            b'-' => self.add_token(TokenType::Minus),
            b'+' => self.add_token(TokenType::Plus),
            b';' => self.add_token(TokenType::Semicolon),
            b'*' => self.add_token(TokenType::Star),
            b'!' => {
                let token_type = if self.match_char(b'=') {
                    TokenType::BangEqual
                } else {
                    TokenType::Bang
                };
                self.add_token(token_type);
            }
            b'=' => {
                let token_type = if self.match_char(b'=') {
                    TokenType::EqualEqual
                } else {
                    TokenType::Equal
                };
                self.add_token(token_type);
            }
            b'<' => {
                let token_type = if self.match_char(b'=') {
                    TokenType::LessEqual
                } else {
                    TokenType::Less
                };
                self.add_token(token_type);
            }
            b'>' => {
                let token_type = if self.match_char(b'=') {
                    TokenType::GreaterEqual
                } else {
                    TokenType::Greater
                };
                self.add_token(token_type);
            }
            b'/' => {
                if self.match_char(b'/') {
                    while self.peek_char() != b'\n' && !self.is_at_end() {
                        self.advance();
                    }
                } else {
                    self.add_token(TokenType::Slash)
                }
            }
            b' ' | b'\r' | b'\t' => {}
            b'\n' => self.line += 1,
            b'"' => self.string(),
            _ => {
                if is_digit(c) {
                    self.number();
                } else if is_alpha(c) {
                    self.identifier();
                } else {
                    self.app.error(self.line, "Unexpected character.");
                }
            }
        }
    }

    fn identifier(&mut self) {
        while is_alpha_numeric(self.peek_char()) {
            self.advance();
        }

        let text = &self.source[self.start..=self.current];
        let token_type = match text {
            b"and" => TokenType::And,
            b"class" => TokenType::Class,
            b"else" => TokenType::Else,
            b"false" => TokenType::False,
            b"for" => TokenType::For,
            b"fun" => TokenType::Fun,
            b"if" => TokenType::If,
            b"nil" => TokenType::Nil,
            b"or" => TokenType::Or,
            b"print" => TokenType::Print,
            b"return" => TokenType::Return,
            b"super" => TokenType::Super,
            b"this" => TokenType::This,
            b"true" => TokenType::True,
            b"var" => TokenType::Var,
            b"while" => TokenType::While,
            _ => TokenType::Identifier,
        };

        self.add_token(token_type);
    }

    fn number(&mut self) {
        while is_digit(self.peek_char()) {
            self.advance();
        }

        // Look for a fractional part.
        if self.peek_char() == b'.' && is_digit(self.peek_next_char()) {
            // Consume the "."
            self.advance();

            while is_digit(self.peek_char()) {
                self.advance();
            }
        }

        let string = str::from_utf8(&self.source[self.start..self.current]).unwrap();
        self.add_token_with_literal(
            TokenType::Number,
            TokenLiteral::Number(f64::from_str(string).unwrap()),
        )
    }

    fn peek_next_char(&self) -> u8 {
        if self.current + 1 >= self.source.len() {
            0
        } else {
            self.source[self.current + 1]
        }
    }

    fn string(&mut self) {
        while self.peek_char() != b'"' && !self.is_at_end() {
            if self.peek_char() == b'\n' {
                self.line += 1;
            }
            self.advance();
        }

        if self.is_at_end() {
            self.app.error(self.line, "Unterminated string.")
        }

        // The closing ".
        self.advance();

        // Trim the surrounding quotes.
        let value = &self.source[(self.start + 1)..(self.current - 1)];
        self.add_token_with_literal(
            TokenType::String,
            TokenLiteral::String(str::from_utf8(value).unwrap().to_string()),
        );
    }

    fn match_char(&mut self, expected: u8) -> bool {
        if self.is_at_end() || self.peek_char() != expected {
            false
        } else {
            self.advance();
            true
        }
    }

    fn peek_char(&self) -> u8 {
        if self.is_at_end() {
            0
        } else {
            self.source[self.current]
        }
    }

    fn advance(&mut self) -> u8 {
        let c = self.source[self.current];
        self.current += 1;
        c
    }

    fn add_token(&mut self, token_type: TokenType) {
        self.add_token_with_literal(token_type, TokenLiteral::None);
    }

    fn add_token_with_literal(&mut self, token_type: TokenType, literal: TokenLiteral) {
        let lexeme = str::from_utf8(&self.source[self.start..self.current]).unwrap();
        self.tokens.push(Token {
            token_type,
            lexeme: lexeme.to_string(),
            literal,
            line: self.line,
        })
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TokenType {
    And,
    Bang,
    BangEqual,
    Class,
    Comma,
    Dot,
    Else,
    Eof,
    Equal,
    EqualEqual,
    False,
    For,
    Fun,
    Greater,
    GreaterEqual,
    Identifier,
    If,
    LeftBrace,
    LeftParen,
    Less,
    LessEqual,
    Minus,
    Nil,
    Number,
    Or,
    Plus,
    Print,
    Return,
    RightBrace,
    RightParen,
    Semicolon,
    Slash,
    Star,
    String,
    Super,
    This,
    True,
    Var,
    While,
}

#[derive(Debug, Clone)]
struct Token {
    token_type: TokenType,
    lexeme: String,
    literal: TokenLiteral,
    line: u64,
}

#[derive(Debug, Clone)]
enum TokenLiteral {
    String(String),
    Number(f64),
    None,
}

fn is_alpha(c: u8) -> bool {
    (c >= b'a' && c <= b'z') || (c >= b'A' && c <= b'A') || c == b'_'
}

fn is_alpha_numeric(c: u8) -> bool {
    is_alpha(c) || is_digit(c)
}

fn is_digit(c: u8) -> bool {
    c >= b'0' && c <= b'9'
}

enum Expr {
    Assign {
        name: Token,
        value: Box<Expr>,
    },

    Binary {
        left: Box<Expr>,
        operator: Token,
        right: Box<Expr>,
    },

    Call {
        callee: Box<Expr>,
        paren: Token,
        arguments: Vec<Expr>,
    },

    Get {
        object: Box<Expr>,
        name: Token,
    },

    Grouping {
        expression: Box<Expr>,
    },

    Literal {
        value: TokenLiteral,
    },

    Logical {
        left: Box<Expr>,
        operator: Token,
        right: Box<Expr>,
    },

    Set {
        object: Box<Expr>,
        name: Token,
        value: Box<Expr>,
    },

    Super {
        keyword: Token,
        method: Token,
    },

    This {
        keyword: Token,
    },

    Unary {
        operator: Token,
        right: Box<Expr>,
    },

    Variable(ExprVariable),
}

struct ExprVariable {
    name: Token,
}

struct StmtFunction {
    name: Token,
    params: Vec<Token>,
    body: Vec<Stmt>,
}

enum Stmt {
    Block {
        statements: Vec<Stmt>,
    },

    Class {
        name: Token,
        superclass: ExprVariable,
        methods: Vec<StmtFunction>,
    },

    Expression(Expr),

    Function(StmtFunction),

    If {
        condition: Expr,
        then_branch: Box<Stmt>,
        else_branch: Box<Stmt>,
    },

    Print {
        expression: Expr,
    },

    Return {
        keyword: Token,
        value: Expr,
    },

    Var {
        name: Token,
        initializer: Expr,
    },

    While {
        condition: Expr,
        body: Box<Stmt>,
    },
}
