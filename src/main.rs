use std::borrow::Borrow;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::rc::Rc;
use std::str::FromStr;
use std::{env, fs, io, str, time};
use string_interner::backend::StringBackend;
use string_interner::symbol::SymbolU32;
use string_interner::StringInterner;

type Symbol = SymbolU32;

struct Interner {
    interner: RefCell<StringInterner<StringBackend<Symbol>>>,
    sym_this: Symbol,
    sym_init: Symbol,
    sym_super: Symbol,
}

impl Interner {
    fn new() -> Interner {
        let mut interner = StringInterner::<StringBackend<Symbol>>::new();
        Interner {
            sym_this: interner.get_or_intern("this"),
            sym_init: interner.get_or_intern("init"),
            sym_super: interner.get_or_intern("super"),
            interner: RefCell::new(interner),
        }
    }

    fn resolve(&self, symbol: Symbol) -> String {
        String::from(
            self.interner
                .borrow()
                .resolve(symbol)
                .expect("Resolving an invalid symbol"),
        )
    }

    fn get_or_intern<T>(&self, string: T) -> Symbol
    where
        T: AsRef<str>,
    {
        self.interner.borrow_mut().get_or_intern(string)
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let app = App::new();
    let mut interpreter = Interpreter::new(&app.interner);

    match &args[..] {
        [_] => app.run_prompt(&mut interpreter),
        [_, path] => app.run_file(&mut interpreter, path),
        _ => {
            println!("Usage: rlox-interpreter [script]");
            std::process::exit(64);
        }
    }
}

struct App {
    had_error: Cell<bool>,
    had_runtime_error: Cell<bool>,
    interner: Interner,
}

impl App {
    fn new() -> App {
        App {
            had_error: Cell::new(false),
            had_runtime_error: Cell::new(false),
            interner: Interner::new(),
        }
    }

    fn error(&self, line: u64, message: &str) {
        self.report(line, "", message);
    }

    fn error_token(&self, token: &Token, message: &str) {
        if token.token_type == TokenType::Eof {
            self.report(token.line, " at end", message);
        } else {
            self.report(
                token.line,
                &format!(" at '{}'", self.interner.resolve(token.lexeme)),
                message,
            );
        }
    }

    fn runtime_error(&self, token: &Token, message: &str) {
        self.had_runtime_error.set(true);
        eprintln!("{}\n[line {}]", message, token.line);
    }

    fn report(&self, line: u64, origin: &str, message: &str) {
        self.had_error.set(true);
        eprintln!("[line {}] Error{}: {}", line, origin, message);
    }

    fn run_file(&self, interpreter: &mut Interpreter, path: &str) {
        match fs::read_to_string(path) {
            Ok(content) => {
                self.run(interpreter, &content);
                if self.had_error.get() {
                    std::process::exit(65);
                }
            }
            _ => {
                println!("Error: could not open file {}", path);
                std::process::exit(66);
            }
        }
    }

    fn run_prompt(&self, interpreter: &mut Interpreter) {
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
                    self.run(interpreter, &line);
                    self.had_error.set(false);
                }
                Err(error) => {
                    println!("Error: {}", error);
                    std::process::exit(70);
                }
            }
        }
    }

    fn run(&self, interpreter: &mut Interpreter, source: &str) {
        let mut scanner = Scanner::new(self, source.as_bytes());
        let tokens = scanner.scan_tokens();
        let mut parser = Parser::new(self, tokens);
        let statements = parser.parse();

        if self.had_error.get() {
            return;
        }

        let mut resolver = Resolver::new(self, interpreter);
        resolver.resolve(&statements);

        if self.had_error.get() {
            return;
        }

        interpreter.interpret(self, &statements);
    }
}

struct Scanner<'a> {
    source: &'a [u8],
    line: u64,
    start: usize,
    current: usize,
    tokens: Vec<Token>,
    app: &'a App,
}

impl Scanner<'_> {
    fn new<'a>(app: &'a App, source: &'a [u8]) -> Scanner<'a> {
        Scanner {
            source,
            line: 1,
            start: 0,
            current: 0,
            tokens: vec![],
            app,
        }
    }

    pub fn scan_tokens(&mut self) -> Vec<Token> {
        self.tokens.clear();

        while !self.is_at_end() {
            self.start = self.current;
            self.scan_token();
        }

        self.tokens.push(Token {
            token_type: TokenType::Eof,
            lexeme: self.app.interner.get_or_intern("<EOF>"),
            literal: TokenLiteral::Nil,
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

        let text = &self.source[self.start..self.current];
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
            TokenLiteral::String(String::from(str::from_utf8(value).unwrap())),
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
        self.add_token_with_literal(token_type, TokenLiteral::Nil);
    }

    fn add_token_with_literal(&mut self, token_type: TokenType, literal: TokenLiteral) {
        let lexeme = str::from_utf8(&self.source[self.start..self.current]).unwrap();
        self.tokens.push(Token {
            token_type,
            lexeme: self.app.interner.get_or_intern(lexeme),
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
    lexeme: Symbol,
    literal: TokenLiteral,
    line: u64,
}

#[derive(Debug, Clone)]
enum TokenLiteral {
    String(String),
    Number(f64),
    Bool(bool),
    Nil,
}

fn is_alpha(c: u8) -> bool {
    (b'a'..=b'z').contains(&c) || (b'A'..=b'Z').contains(&c) || c == b'_'
}

fn is_alpha_numeric(c: u8) -> bool {
    is_alpha(c) || is_digit(c)
}

fn is_digit(c: u8) -> bool {
    (b'0'..=b'9').contains(&c)
}

#[derive(Clone, Debug)]
enum Expr {
    Assign {
        name: Token,
        value: Box<Expr>,
        id: ExprId,
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
        id: ExprId,
    },

    This {
        keyword: Token,
        id: ExprId,
    },

    Unary {
        operator: Token,
        right: Box<Expr>,
    },

    Variable(ExprId, ExprVariable),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
struct ExprId(u64);

#[derive(Clone, Debug)]
struct ExprVariable {
    name: Token,
}

#[derive(Clone)]
struct StmtFunction {
    name: Token,
    params: Vec<Token>,
    body: Vec<Stmt>,
}

#[derive(Clone)]
enum Stmt {
    Block {
        statements: Vec<Stmt>,
    },

    Class {
        name: Token,
        superclass: Option<Expr>,
        methods: Vec<StmtFunction>,
    },

    Expression(Expr),

    Function(StmtFunction),

    If {
        condition: Expr,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
    },

    Print {
        expression: Expr,
    },

    Return {
        keyword: Token,
        value: Option<Expr>,
    },

    Var {
        name: Token,
        initializer: Option<Expr>,
    },

    While {
        condition: Expr,
        body: Box<Stmt>,
    },
}

struct Parser<'a> {
    tokens: Vec<Token>,
    current: usize,
    app: &'a App,
    expr_id_count: u64,
}

impl Parser<'_> {
    fn new(app: &App, tokens: Vec<Token>) -> Parser {
        Parser {
            tokens,
            current: 0,
            app,
            expr_id_count: 0,
        }
    }

    fn expression(&mut self) -> Option<Expr> {
        self.assignment()
    }

    fn assignment(&mut self) -> Option<Expr> {
        let expr = self.or()?;

        if self.match_one_of([TokenType::Equal]) {
            let value = self.assignment()?;
            let equals = self.previous_token().clone();

            return match expr {
                Expr::Variable(_, ExprVariable { name }) => Some(Expr::Assign {
                    name,
                    value: Box::from(value),
                    id: self.gen_expr_id(),
                }),
                Expr::Get { object, name } => Some(Expr::Set {
                    object,
                    name,
                    value: Box::new(value),
                }),
                _ => {
                    self.app.error_token(&equals, "Invalid assignment target.");
                    None
                }
            };
        }

        Some(expr)
    }

    fn or(&mut self) -> Option<Expr> {
        let mut expr = self.and();

        expr.as_ref()?;

        while self.match_one_of([TokenType::Or]) {
            let operator = self.previous_token().clone();
            let right = self.and()?;
            expr = Some(Expr::Logical {
                left: Box::from(expr?),
                operator,
                right: Box::from(right),
            });
        }

        expr
    }

    fn and(&mut self) -> Option<Expr> {
        let mut expr = self.equality();

        expr.as_ref()?;

        while self.match_one_of([TokenType::And]) {
            let operator = self.previous_token().clone();
            let right = self.equality()?;
            expr = Some(Expr::Logical {
                left: Box::from(expr?),
                operator,
                right: Box::from(right),
            });
        }

        expr
    }

    fn equality(&mut self) -> Option<Expr> {
        let mut expr = self.comparison();

        expr.as_ref()?;

        while self.match_one_of([TokenType::EqualEqual, TokenType::BangEqual]) {
            let operator = self.previous_token().clone();
            let right = self.comparison()?;
            expr = Some(Expr::Binary {
                left: Box::from(expr?),
                operator,
                right: Box::from(right),
            })
        }

        expr
    }

    fn comparison(&mut self) -> Option<Expr> {
        let mut expr = self.term();

        expr.as_ref()?;

        while self.match_one_of([
            TokenType::Greater,
            TokenType::GreaterEqual,
            TokenType::Less,
            TokenType::LessEqual,
        ]) {
            let operator = self.previous_token().clone();
            let right = self.term()?;
            expr = Some(Expr::Binary {
                left: Box::new(expr?),
                operator,
                right: Box::new(right),
            })
        }

        expr
    }

    fn term(&mut self) -> Option<Expr> {
        let mut expr = self.factor();

        expr.as_ref()?;

        while self.match_one_of([TokenType::Minus, TokenType::Plus]) {
            let operator = self.previous_token().clone();
            let right = self.factor()?;
            expr = Some(Expr::Binary {
                left: Box::new(expr?),
                operator,
                right: Box::new(right),
            })
        }

        expr
    }

    fn factor(&mut self) -> Option<Expr> {
        let mut expr = self.unary();

        expr.as_ref()?;

        while self.match_one_of([TokenType::Slash, TokenType::Star]) {
            let operator = self.previous_token().clone();
            let right = self.unary()?;
            expr = Some(Expr::Binary {
                left: Box::new(expr?),
                operator,
                right: Box::new(right),
            })
        }

        expr
    }

    fn unary(&mut self) -> Option<Expr> {
        if self.match_one_of([TokenType::Bang, TokenType::Minus]) {
            let operator = self.previous_token().clone();
            let right = self.unary()?;
            Some(Expr::Unary {
                operator,
                right: Box::new(right),
            })
        } else {
            self.call()
        }
    }

    fn call(&mut self) -> Option<Expr> {
        let mut expr = self.primary();

        expr.as_ref()?;

        loop {
            if self.match_one_of([TokenType::LeftParen]) {
                expr = self.finish_call(expr?);
            } else if self.match_one_of([TokenType::Dot]) {
                let name =
                    self.consume(TokenType::Identifier, "Expect property name after '.'.")?;
                expr = Some(Expr::Get {
                    object: Box::new(expr?.clone()),
                    name,
                })
            } else {
                break;
            }
        }

        expr
    }

    fn finish_call(&mut self, callee: Expr) -> Option<Expr> {
        let mut arguments = Vec::new();

        if !self.check_token(TokenType::RightParen) {
            loop {
                if arguments.len() >= 255 {
                    self.app.error_token(
                        &self.peek_token().clone(),
                        "Can't have more than 255 arguments.",
                    );
                }
                arguments.push(self.expression()?);
                if !self.match_one_of([TokenType::Comma]) {
                    break;
                }
            }
        }

        let paren = self.consume(TokenType::RightParen, "Expect ')' after arguments.")?;

        Some(Expr::Call {
            callee: Box::new(callee),
            paren,
            arguments,
        })
    }

    fn primary(&mut self) -> Option<Expr> {
        if self.match_one_of([TokenType::False]) {
            Some(Expr::Literal {
                value: TokenLiteral::Bool(false),
            })
        } else if self.match_one_of([TokenType::True]) {
            Some(Expr::Literal {
                value: TokenLiteral::Bool(true),
            })
        } else if self.match_one_of([TokenType::Nil]) {
            Some(Expr::Literal {
                value: TokenLiteral::Nil,
            })
        } else if self.match_one_of([TokenType::Number, TokenType::String]) {
            Some(Expr::Literal {
                value: self.previous_token().literal.clone(),
            })
        } else if self.match_one_of([TokenType::Identifier]) {
            Some(Expr::Variable(
                self.gen_expr_id(),
                ExprVariable {
                    name: self.previous_token().clone(),
                },
            ))
        } else if self.match_one_of([TokenType::Super]) {
            let keyword = self.previous_token().clone();
            self.consume(TokenType::Dot, "Expect '.' after 'super'.")?;
            let method = self.consume(TokenType::Identifier, "Expect superclass method name.")?;
            Some(Expr::Super {
                keyword,
                method,
                id: self.gen_expr_id(),
            })
        } else if self.match_one_of([TokenType::This]) {
            Some(Expr::This {
                keyword: self.previous_token().clone(),
                id: self.gen_expr_id(),
            })
        } else if self.match_one_of([TokenType::LeftParen]) {
            let expr = self.expression()?;
            self.consume(TokenType::RightParen, "Expect ')' after expression.")?;
            Some(Expr::Grouping {
                expression: Box::new(expr),
            })
        } else {
            self.app
                .error_token(&self.peek_token().clone(), "Expect expression.");
            None
        }
    }

    fn gen_expr_id(&mut self) -> ExprId {
        let id = ExprId(self.expr_id_count);
        self.expr_id_count += 1;
        return id;
    }

    fn parse(&mut self) -> Vec<Stmt> {
        let mut statements = Vec::new();

        while !self.is_at_end() {
            if let Some(statement) = self.declaration() {
                statements.push(statement);
            }
        }

        statements
    }

    fn statement(&mut self) -> Option<Stmt> {
        if self.match_one_of([TokenType::For]) {
            self.for_statement()
        } else if self.match_one_of([TokenType::If]) {
            self.if_statement()
        } else if self.match_one_of([TokenType::Print]) {
            self.print_statement()
        } else if self.match_one_of([TokenType::Return]) {
            self.return_statement()
        } else if self.match_one_of([TokenType::While]) {
            self.while_statement()
        } else if self.match_one_of([TokenType::LeftBrace]) {
            Some(Stmt::Block {
                statements: self.block()?,
            })
        } else {
            self.expression_statement()
        }
    }

    fn block(&mut self) -> Option<Vec<Stmt>> {
        let mut statements = Vec::new();

        while !self.check_token(TokenType::RightBrace) && !self.is_at_end() {
            statements.push(self.declaration()?);
        }

        self.consume(TokenType::RightBrace, "Expect '}' after block.")?;

        Some(statements)
    }

    fn for_statement(&mut self) -> Option<Stmt> {
        self.consume(TokenType::LeftParen, "Expect '(' after 'for'.")?;

        let initializer = if self.match_one_of([TokenType::Semicolon]) {
            None
        } else if self.match_one_of([TokenType::Var]) {
            Some(self.var_declaration()?)
        } else {
            Some(self.expression_statement()?)
        };

        let condition = if !self.check_token(TokenType::Semicolon) {
            Some(self.expression()?)
        } else {
            None
        };

        self.consume(TokenType::Semicolon, "Expect ';' after loop condition.")?;

        let increment = if !self.check_token(TokenType::RightParen) {
            Some(self.expression()?)
        } else {
            None
        };

        self.consume(TokenType::RightParen, "Expect ')' after for clauses.")?;

        let mut body = self.statement()?;

        if let Some(increment) = increment {
            body = Stmt::Block {
                statements: vec![body, Stmt::Expression(increment)],
            };
        };

        body = Stmt::While {
            condition: condition.unwrap_or(Expr::Literal {
                value: TokenLiteral::Bool(true),
            }),
            body: Box::new(body),
        };

        if let Some(initializer) = initializer {
            body = Stmt::Block {
                statements: vec![initializer, body],
            };
        }

        Some(body)
    }

    fn if_statement(&mut self) -> Option<Stmt> {
        self.consume(TokenType::LeftParen, "Expect '(' after 'if'.")?;
        let condition = self.expression()?;
        self.consume(TokenType::RightParen, "Expect ')' after if condition.")?;

        let then_branch = self.statement()?;
        let mut else_branch = None;
        if self.match_one_of([TokenType::Else]) {
            else_branch = Some(Box::from(self.statement()?));
        }

        Some(Stmt::If {
            condition,
            then_branch: Box::new(then_branch),
            else_branch,
        })
    }

    fn print_statement(&mut self) -> Option<Stmt> {
        let expression = self.expression()?;
        self.consume(TokenType::Semicolon, "Expect ';' after value.")?;
        Some(Stmt::Print { expression })
    }

    fn return_statement(&mut self) -> Option<Stmt> {
        let keyword = self.previous_token().clone();
        let mut value = None;
        if !self.check_token(TokenType::Semicolon) {
            value = Some(self.expression()?);
        }

        self.consume(TokenType::Semicolon, "Expect ';' after return value.")?;

        Some(Stmt::Return { keyword, value })
    }

    fn while_statement(&mut self) -> Option<Stmt> {
        self.consume(TokenType::LeftParen, "Expect '(' after 'while'.")?;
        let condition = self.expression()?;
        self.consume(TokenType::RightParen, "Expect ')' after condition.")?;
        let body = self.statement()?;

        Some(Stmt::While {
            condition,
            body: Box::from(body),
        })
    }

    fn expression_statement(&mut self) -> Option<Stmt> {
        let expression = self.expression()?;
        self.consume(TokenType::Semicolon, "Expect ';' after expression.")?;
        Some(Stmt::Expression(expression))
    }

    fn declaration(&mut self) -> Option<Stmt> {
        if self.match_one_of([TokenType::Class]) {
            self.class_declaration()
        } else if self.match_one_of([TokenType::Fun]) {
            self.function("function").map(|f| Stmt::Function(f))
        } else if self.match_one_of([TokenType::Var]) {
            self.var_declaration()
        } else {
            match self.statement() {
                statement @ Some(_) => statement,
                None => {
                    self.synchronize();
                    None
                }
            }
        }
    }

    fn class_declaration(&mut self) -> Option<Stmt> {
        let name = self.consume(TokenType::Identifier, "Expect class name.")?;

        let superclass = if self.match_one_of([TokenType::Less]) {
            self.consume(TokenType::Identifier, "Expect superclass name.")?;
            Some(Expr::Variable(
                self.gen_expr_id(),
                ExprVariable {
                    name: self.previous_token().clone(),
                },
            ))
        } else {
            None
        };

        self.consume(TokenType::LeftBrace, "Expect '{' before class body.")?;

        let mut methods = Vec::new();
        while !self.check_token(TokenType::RightBrace) && !self.is_at_end() {
            methods.push(self.function("method")?);
        }

        self.consume(TokenType::RightBrace, "Expect '}' after class body.")?;

        Some(Stmt::Class {
            name,
            superclass,
            methods,
        })
    }

    fn function(&mut self, kind: &str) -> Option<StmtFunction> {
        let name = self.consume(TokenType::Identifier, &format!("Expect {} name.", kind))?;

        self.consume(
            TokenType::LeftParen,
            &format!("Expect '(' after {} name.", kind),
        )?;

        let mut params = Vec::new();
        if !self.check_token(TokenType::RightParen) {
            loop {
                if params.len() >= 255 {
                    self.app.error_token(
                        &self.peek_token().clone(),
                        "Can't have more than 255 parameters.",
                    );
                }

                params.push(self.consume(TokenType::Identifier, "Expect parameter name.")?);

                if !self.match_one_of([TokenType::Comma]) {
                    break;
                }
            }
        }

        self.consume(TokenType::RightParen, "Expect ')' after parameters.")?;

        self.consume(
            TokenType::LeftBrace,
            &format!("Expect '{{' before {} body.", kind),
        )?;

        let body = self.block()?;

        Some(StmtFunction { name, params, body })
    }

    fn var_declaration(&mut self) -> Option<Stmt> {
        let name = self.consume(TokenType::Identifier, "Expect variable name.")?;

        let mut initializer = None;
        if self.match_one_of([TokenType::Equal]) {
            initializer = self.expression();
        }

        self.consume(
            TokenType::Semicolon,
            "Expect ';' after variable declaration.",
        )?;

        Some(Stmt::Var { name, initializer })
    }

    fn consume(&mut self, token_type: TokenType, message: &str) -> Option<Token> {
        if self.check_token(token_type) {
            Some(self.advance().clone())
        } else {
            self.app.error_token(&self.peek_token().clone(), message);
            None
        }
    }

    fn synchronize(&mut self) {
        self.advance();

        while !self.is_at_end() {
            if self.previous_token().token_type == TokenType::Semicolon {
                return;
            }

            match self.peek_token().token_type {
                TokenType::Class
                | TokenType::Fun
                | TokenType::Var
                | TokenType::For
                | TokenType::If
                | TokenType::While
                | TokenType::Print
                | TokenType::Return => return,
                _ => {}
            }

            self.advance();
        }
    }

    fn match_one_of<const N: usize>(&mut self, token_types: [TokenType; N]) -> bool {
        for token_type in token_types {
            if self.check_token(token_type) {
                self.advance();
                return true;
            }
        }

        false
    }

    fn check_token(&self, token_type: TokenType) -> bool {
        if self.is_at_end() {
            false
        } else {
            self.peek_token().token_type == token_type
        }
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous_token()
    }

    fn is_at_end(&self) -> bool {
        self.peek_token().token_type == TokenType::Eof
    }

    fn peek_token(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn previous_token(&self) -> &Token {
        &self.tokens[self.current - 1]
    }
}

#[derive(Clone)]
enum Value {
    String(Rc<String>),
    Number(f64),
    Bool(bool),
    Callable(Rc<Function>),
    Instance(Rc<Instance>),
    Nil,
}

impl Value {
    fn to_instance(&self) -> Option<Rc<Instance>> {
        match self {
            Value::Instance(instance) => Some(Rc::clone(instance)),
            _ => None,
        }
    }

    fn is_class(&self) -> bool {
        if let Value::Callable(function) = self {
            if let Function::Class(..) = Rc::borrow(function) {
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    fn to_class(&self) -> Option<Rc<Class>> {
        match self {
            Value::Callable(function) => {
                if let Function::Class(_, class) = Rc::borrow(function) {
                    Some(Rc::clone(class))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

enum Function {
    Native(
        usize,
        fn(&mut Interpreter, &[Value]) -> Result<Value, ErrCause>,
    ),
    Declared(StmtFunction, Rc<Environment>, bool),
    Class(usize, Rc<Class>),
}

impl Function {
    fn call(
        &self,
        interpreter: &mut Interpreter,
        interner: &Interner,
        arguments: &[Value],
    ) -> Result<Value, ErrCause> {
        match self {
            Function::Native(_, function) => function(interpreter, arguments),
            Function::Declared(StmtFunction { params, body, .. }, closure, is_initializer) => {
                let environment = Environment::new(Some(Rc::clone(closure)));

                for i in 0..params.len() {
                    environment.define(params[i].lexeme.clone(), arguments[i].clone())
                }

                let result = interpreter.execute_block(interner, body, environment);

                if *is_initializer {
                    return Ok(closure
                        .values
                        .borrow()
                        .get(&interner.sym_this)
                        .unwrap()
                        .clone());
                }

                result.map(|_| Value::Nil)
            }
            Function::Class(_, class) => {
                let instance = Rc::new(Instance::new(Rc::clone(class)));
                if let Some(Value::Callable(initializer)) = instance.find_method(interner.sym_init)
                {
                    initializer.bind(interner, Rc::clone(&instance)).call(
                        interpreter,
                        interner,
                        arguments,
                    )?;
                }

                Ok(Value::Instance(instance))
            }
        }
    }

    fn bind(&self, interner: &Interner, instance: Rc<Instance>) -> Function {
        if let Function::Declared(stmt_function, closure, is_initializer) = self {
            let environment = Environment::new(Some(Rc::clone(closure)));
            environment.define(interner.sym_this, Value::Instance(instance));
            Function::Declared(stmt_function.clone(), Rc::new(environment), *is_initializer)
        } else {
            unreachable!()
        }
    }

    fn arity(&self) -> usize {
        match self {
            Function::Native(arity, _) => *arity,
            Function::Declared(stmt_function, _, _) => stmt_function.params.len(),
            Function::Class(arity, _) => *arity,
        }
    }
}

struct Interpreter {
    global_environment: Rc<Environment>,
    environment: Rc<Environment>,
    locals: HashMap<ExprId, usize>,
}

enum ErrCause {
    Error(Token, String),
    Return(Value),
}

impl Interpreter {
    fn new(interner: &Interner) -> Interpreter {
        let global_environment = Rc::new(Environment::new(None));

        global_environment.define(
            interner.get_or_intern("clock"),
            Value::Callable(Rc::new(Function::Native(0, |_, _| {
                if let Ok(n) = time::SystemTime::now().duration_since(time::UNIX_EPOCH) {
                    Ok(Value::Number(n.as_secs_f64()))
                } else {
                    panic!("SystemTime before UNIX_EPOCH.");
                }
            }))),
        );

        let environment = Rc::clone(&global_environment);

        Interpreter {
            global_environment,
            environment,
            locals: HashMap::new(),
        }
    }

    fn interpret(&mut self, app: &App, statements: &[Stmt]) {
        for statement in statements {
            match self.execute(&app.interner, statement) {
                Ok(_) => {}
                Err(ErrCause::Error(token, message)) => {
                    app.runtime_error(&token, &message);
                    break;
                }
                Err(ErrCause::Return(_)) => panic!("Unexpected top level return."),
            }
        }
    }

    fn execute(&mut self, interner: &Interner, statement: &Stmt) -> Result<(), ErrCause> {
        match statement {
            Stmt::Expression(expr) => {
                self.evaluate(interner, expr)?;
            }
            Stmt::Print { expression } => {
                let value = self.evaluate(interner, expression)?;
                println!("{}", stringify(interner, &value));
            }
            Stmt::Var { name, initializer } => {
                let value = match initializer {
                    Some(expr) => self.evaluate(interner, expr)?,
                    _ => Value::Nil,
                };
                self.environment.define(name.lexeme.clone(), value);
            }
            Stmt::Block { statements } => {
                let environment = Environment::new(Some(Rc::clone(&self.environment)));
                self.execute_block(interner, statements, environment)?;
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => {
                if is_truthy(&self.evaluate(interner, condition)?) {
                    self.execute(interner, then_branch)?;
                } else if let Some(else_branch) = else_branch {
                    self.execute(interner, else_branch)?;
                }
            }
            Stmt::While { condition, body } => {
                while is_truthy(&self.evaluate(interner, condition)?) {
                    self.execute(interner, body)?;
                }
            }
            Stmt::Function(function_stmt) => {
                let function = Value::Callable(Rc::new(Function::Declared(
                    function_stmt.clone(),
                    Rc::clone(&self.environment),
                    false,
                )));

                self.environment
                    .define(function_stmt.name.lexeme.clone(), function);
            }
            Stmt::Class {
                name,
                methods,
                superclass,
            } => {
                let superclass_value = if let Some(superclass) = superclass {
                    let value = self.evaluate(interner, superclass)?;
                    if value.is_class() {
                        Some(value)
                    } else {
                        if let Expr::Variable(_, superclass) = superclass {
                            return Err(ErrCause::Error(
                                superclass.name.clone(),
                                String::from("Superclass must be a class."),
                            ));
                        } else {
                            unreachable!();
                        }
                    }
                } else {
                    None
                };

                self.environment.define(name.lexeme.clone(), Value::Nil);

                let environment = if let Some(superclass) = &superclass_value {
                    let environment = Rc::new(Environment::new(Some(Rc::clone(&self.environment))));
                    environment.define(interner.sym_super, superclass.clone());
                    environment
                } else {
                    Rc::clone(&self.environment)
                };

                let mut initializer_arity = 0;
                let mut class_methods = HashMap::new();
                for method in methods {
                    let is_initializer = method.name.lexeme == interner.sym_init;
                    if is_initializer {
                        initializer_arity = method.params.len();
                    }
                    let function = Value::Callable(Rc::new(Function::Declared(
                        method.clone(),
                        Rc::clone(&environment),
                        is_initializer,
                    )));
                    class_methods.insert(method.name.lexeme.clone(), function);
                }

                let superclass = superclass_value
                    .map(|superclass| superclass.to_class())
                    .flatten();

                let class = Value::Callable(Rc::new(Function::Class(
                    initializer_arity,
                    Rc::new(Class {
                        name: name.lexeme,
                        methods: class_methods,
                        superclass,
                    }),
                )));

                self.environment.assign(interner, &name, class)?;
            }
            Stmt::Return { value, .. } => {
                let return_value = match value {
                    Some(value_expr) => self.evaluate(interner, value_expr)?,
                    None => Value::Nil,
                };

                return Err(ErrCause::Return(return_value));
            }
        }
        Ok(())
    }

    fn execute_block(
        &mut self,
        interner: &Interner,
        statements: &[Stmt],
        environment: Environment,
    ) -> Result<(), ErrCause> {
        let previous = std::mem::replace(&mut self.environment, Rc::new(environment));

        let mut ret = Ok(());
        for statement in statements {
            ret = self.execute(interner, statement);
            if ret.is_err() {
                break;
            }
        }

        self.environment = previous;

        ret
    }

    fn evaluate(&mut self, interner: &Interner, expr: &Expr) -> Result<Value, ErrCause> {
        match expr {
            Expr::Binary {
                left,
                operator,
                right,
            } => {
                let left = self.evaluate(interner, left)?;
                let right = self.evaluate(interner, right)?;

                match operator.token_type {
                    TokenType::Minus => {
                        let (left_num, right_num) =
                            self.check_number_operands(operator, &left, &right)?;
                        Ok(Value::Number(left_num - right_num))
                    }
                    TokenType::Slash => {
                        let (left_num, right_num) =
                            self.check_number_operands(operator, &left, &right)?;
                        Ok(Value::Number(left_num / right_num))
                    }
                    TokenType::Star => {
                        let (left_num, right_num) =
                            self.check_number_operands(operator, &left, &right)?;
                        Ok(Value::Number(left_num * right_num))
                    }
                    TokenType::Plus => match (left, right) {
                        (Value::Number(left_num), Value::Number(right_num)) => {
                            Ok(Value::Number(left_num + right_num))
                        }
                        (Value::String(left_str), Value::String(right_str)) => {
                            Ok(Value::String(Rc::from((&*left_str).clone() + &*right_str)))
                        }
                        _ => Err(ErrCause::Error(
                            operator.clone(),
                            String::from("Operands must be two numbers or two strings."),
                        )),
                    },
                    TokenType::Greater => {
                        let (left_num, right_num) =
                            self.check_number_operands(operator, &left, &right)?;
                        Ok(Value::Bool(left_num > right_num))
                    }
                    TokenType::GreaterEqual => {
                        let (left_num, right_num) =
                            self.check_number_operands(operator, &left, &right)?;
                        Ok(Value::Bool(left_num >= right_num))
                    }
                    TokenType::Less => {
                        let (left_num, right_num) =
                            self.check_number_operands(operator, &left, &right)?;
                        Ok(Value::Bool(left_num < right_num))
                    }
                    TokenType::LessEqual => {
                        let (left_num, right_num) =
                            self.check_number_operands(operator, &left, &right)?;
                        Ok(Value::Bool(left_num <= right_num))
                    }
                    TokenType::BangEqual => Ok(Value::Bool(!is_equal(&left, &right))),
                    TokenType::EqualEqual => Ok(Value::Bool(is_equal(&left, &right))),
                    _ => panic!("Unexpected binary operator token."),
                }
            }
            Expr::Grouping { expression } => self.evaluate(interner, expression),
            Expr::Literal { value } => match value {
                TokenLiteral::String(str) => Ok(Value::String(Rc::new(str.clone()))),
                TokenLiteral::Number(num) => Ok(Value::Number(*num)),
                TokenLiteral::Bool(bool) => Ok(Value::Bool(*bool)),
                TokenLiteral::Nil => Ok(Value::Nil),
            },
            Expr::Unary { operator, right } => {
                let right = self.evaluate(interner, right)?;

                match operator.token_type {
                    TokenType::Bang => Ok(Value::Bool(!is_truthy(&right))),
                    TokenType::Minus => {
                        let num = self.check_number_operand(operator, &right);
                        Ok(Value::Number(-(num?)))
                    }
                    _ => panic!("Unexpected unary operator token."),
                }
            }
            Expr::Variable(id, ExprVariable { name }) => self.look_up_variable(interner, name, *id),
            Expr::Assign { name, value, id } => {
                let value = self.evaluate(interner, value)?;
                if let Some(distance) = self.locals.get(id).cloned() {
                    Environment::assign_at(
                        interner,
                        &self.environment,
                        distance,
                        &name,
                        value.clone(),
                    )?;
                } else {
                    self.global_environment
                        .assign(interner, name, value.clone())?;
                }
                Ok(value)
            }
            Expr::Logical {
                left,
                operator,
                right,
            } => {
                let left = self.evaluate(interner, left)?;

                if operator.token_type == TokenType::Or {
                    if is_truthy(&left) {
                        return Ok(left);
                    }
                } else if !is_truthy(&left) {
                    return Ok(left);
                }

                self.evaluate(interner, right)
            }
            Expr::Call {
                callee,
                paren,
                arguments,
            } => {
                let callee = self.evaluate(interner, callee)?;

                let mut argument_values = Vec::new();
                for argument in arguments {
                    argument_values.push(self.evaluate(interner, argument)?);
                }

                if let Value::Callable(function) = callee {
                    if argument_values.len() == function.arity() {
                        let f: &Function = Rc::borrow(&function);
                        match f.call(self, interner, &argument_values) {
                            Err(ErrCause::Return(value)) => Ok(value),
                            result => result,
                        }
                    } else {
                        let message = format!(
                            "Expected {} arguments but got {} .",
                            function.arity(),
                            argument_values.len()
                        );
                        Err(ErrCause::Error(paren.clone(), message))
                    }
                } else {
                    Err(ErrCause::Error(
                        paren.clone(),
                        String::from("Can only call functions and classes."),
                    ))
                }
            }
            Expr::Get { object, name } => {
                let object = self.evaluate(interner, object)?;
                if let Value::Instance(instance) = object {
                    instance.get(interner, name)
                } else {
                    Err(ErrCause::Error(
                        name.clone(),
                        String::from("Only instances have properties."),
                    ))
                }
            }
            Expr::Set {
                object,
                name,
                value,
            } => {
                let mut object = self.evaluate(interner, object)?;

                if let Value::Instance(instance) = &mut object {
                    let value = self.evaluate(interner, value)?;
                    instance.set(name, value.clone());
                    Ok(value)
                } else {
                    Err(ErrCause::Error(
                        name.clone(),
                        String::from("Only instances have fields."),
                    ))
                }
            }
            Expr::This { keyword, id } => self.look_up_variable(interner, keyword, *id),
            Expr::Super { method, id, .. } => {
                let distance = self.locals.get(id).cloned().unwrap();
                let superclass =
                    Environment::get_at(&self.environment, distance, interner.sym_super);
                let object =
                    Environment::get_at(&self.environment, distance - 1, interner.sym_this);
                let method_value = if let Value::Callable(function) = superclass {
                    if let Function::Class(_, class) = Rc::borrow(&function) {
                        class.find_method(method.lexeme)
                    } else {
                        unreachable!()
                    }
                } else {
                    unreachable!()
                };
                match method_value {
                    Some(Value::Callable(function)) => Ok(Value::Callable(Rc::new(
                        function.bind(interner, object.to_instance().unwrap()),
                    ))),
                    None => Err(ErrCause::Error(
                        method.clone(),
                        format!(
                            "Undefined property '{}'.",
                            interner.interner.borrow().resolve(method.lexeme).unwrap()
                        ),
                    )),
                    _ => unreachable!(),
                }
            }
        }
    }

    fn resolve(&mut self, id: ExprId, depth: usize) {
        self.locals.insert(id, depth);
    }

    fn look_up_variable(
        &mut self,
        interner: &Interner,
        name: &Token,
        id: ExprId,
    ) -> Result<Value, ErrCause> {
        let distance = self.locals.get(&id);
        if let Some(distance) = distance {
            Ok(Environment::get_at(
                &self.environment,
                *distance,
                name.lexeme,
            ))
        } else {
            self.global_environment.get(interner, name)
        }
    }

    fn check_number_operand(&mut self, operator: &Token, operand: &Value) -> Result<f64, ErrCause> {
        match operand {
            Value::Number(num) => Ok(*num),
            _ => Err(ErrCause::Error(
                operator.clone(),
                String::from("Operand must be a number."),
            )),
        }
    }

    fn check_number_operands(
        &mut self,
        operator: &Token,
        left: &Value,
        right: &Value,
    ) -> Result<(f64, f64), ErrCause> {
        match (left, right) {
            (Value::Number(left_num), Value::Number(right_num)) => Ok((*left_num, *right_num)),
            _ => Err(ErrCause::Error(
                operator.clone(),
                String::from("Operands must be a number."),
            )),
        }
    }
}

fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Bool(bool) => *bool,
        Value::Nil => false,
        _ => true,
    }
}

fn is_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::String(l), Value::String(r)) => l == r,
        (Value::Number(l), Value::Number(r)) => l == r,
        (Value::Bool(l), Value::Bool(r)) => l == r,
        (Value::Nil, Value::Nil) => true,
        (_, _) => false,
    }
}

fn stringify(interner: &Interner, value: &Value) -> String {
    match value {
        Value::String(str) => str.as_ref().clone(),
        Value::Number(num) => format!("{}", num),
        Value::Bool(b) => {
            if *b {
                String::from("true")
            } else {
                String::from("false")
            }
        }
        Value::Nil => String::from("nil"),
        Value::Callable(function) => match &*Rc::borrow(function) {
            Function::Native(..) => String::from("<native fn>"),
            Function::Declared(StmtFunction { name, .. }, ..) => {
                format!("<fn {}>", interner.resolve(name.lexeme))
            }
            Function::Class(_, class) => interner.resolve(class.name),
        },
        Value::Instance(instance) => format!("{} instance", interner.resolve(instance.class.name)),
    }
}

#[derive(Clone)]
struct Environment {
    values: RefCell<HashMap<Symbol, Value>>,
    enclosing: Option<Rc<Environment>>,
}

macro_rules! env_ancestor {
    ($init:expr, $distance:expr) => {{
        let mut env = $init;
        for _ in 0..$distance {
            env = env.enclosing.as_ref().unwrap();
        }
        env
    }};
}

impl Environment {
    fn new(enclosing: Option<Rc<Environment>>) -> Environment {
        Environment {
            values: RefCell::new(HashMap::new()),
            enclosing,
        }
    }

    fn define(&self, name: Symbol, value: Value) {
        self.values.borrow_mut().insert(name, value);
    }

    fn assign(&self, interner: &Interner, name: &Token, value: Value) -> Result<(), ErrCause> {
        if self.values.borrow().get(&name.lexeme).is_some() {
            self.values.borrow_mut().insert(name.lexeme.clone(), value);
            Ok(())
        } else {
            self.enclosing.as_ref().map_or(
                Err(ErrCause::Error(
                    name.clone(),
                    format!("Undefined variable '{}'.", interner.resolve(name.lexeme)),
                )),
                |enclosing| enclosing.assign(interner, name, value),
            )
        }
    }

    fn get(&self, interner: &Interner, name: &Token) -> Result<Value, ErrCause> {
        match self.values.borrow().get(&name.lexeme) {
            Some(value) => Ok(value.clone()),
            None => self.enclosing.as_ref().map_or(
                Err(ErrCause::Error(
                    name.clone(),
                    format!("Undefined variable '{}'.", interner.resolve(name.lexeme)),
                )),
                |enclosing| enclosing.get(interner, name),
            ),
        }
    }

    fn get_at(environment: &Rc<Environment>, distance: usize, name: Symbol) -> Value {
        env_ancestor!(environment, distance)
            .values
            .borrow()
            .get(&name)
            .unwrap()
            .clone()
    }

    fn assign_at(
        interner: &Interner,
        environment: &Rc<Environment>,
        distance: usize,
        name: &Token,
        value: Value,
    ) -> Result<(), ErrCause> {
        env_ancestor!(environment, distance).assign(interner, name, value)
    }
}

struct Resolver<'a> {
    app: &'a App,
    interpreter: &'a mut Interpreter,
    scopes: Vec<HashMap<Symbol, bool>>,
    current_function: FunctionType,
    current_class: ClassType,
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum FunctionType {
    None,
    Function,
    Initializer,
    Method,
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum ClassType {
    None,
    Class,
    Subclass,
}

impl Resolver<'_> {
    fn new<'a>(app: &'a App, interpreter: &'a mut Interpreter) -> Resolver<'a> {
        Resolver {
            app,
            interpreter,
            scopes: Vec::new(),
            current_function: FunctionType::None,
            current_class: ClassType::None,
        }
    }

    fn resolve(&mut self, statements: &[Stmt]) {
        for stmt in statements {
            self.resolve_stmt(stmt)
        }
    }

    fn resolve_stmt(&mut self, statement: &Stmt) {
        match statement {
            Stmt::Block { statements } => {
                self.begin_scope();
                self.resolve(statements);
                self.end_scope();
            }
            Stmt::Expression(expr) => self.resolve_expr(expr),
            Stmt::Function(function) => {
                self.declare(&function.name);
                self.define(&function.name);

                self.resolve_function(&function, FunctionType::Function);
            }
            Stmt::Class {
                name,
                methods,
                superclass,
            } => {
                let enclosing_class = self.current_class;
                self.current_class = ClassType::Class;

                self.declare(name);
                self.define(name);

                if let Some(superclass) = superclass {
                    if let Expr::Variable(
                        _,
                        ExprVariable {
                            name: superclass_name,
                        },
                    ) = superclass
                    {
                        if superclass_name.lexeme == name.lexeme {
                            self.app
                                .error_token(name, "A class can't inherit from itself.");
                        }
                    } else {
                        unreachable!();
                    }

                    self.current_class = ClassType::Subclass;

                    self.resolve_expr(superclass);

                    self.begin_scope();
                    let last = self.scopes.len() - 1;
                    self.scopes[last].insert(self.app.interner.sym_super, true);
                }

                self.begin_scope();
                let last = self.scopes.len() - 1;
                self.scopes[last].insert(self.app.interner.sym_this, true);

                for method in methods {
                    let declaration = if method.name.lexeme == self.app.interner.sym_init {
                        FunctionType::Initializer
                    } else {
                        FunctionType::Method
                    };
                    self.resolve_function(&method, declaration);
                }

                self.end_scope();

                if superclass.is_some() {
                    self.end_scope();
                }

                self.current_class = enclosing_class;
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.resolve_expr(condition);
                self.resolve_stmt(then_branch);
                else_branch.as_ref().map(|stmt| self.resolve_stmt(stmt));
            }
            Stmt::Print { expression } => self.resolve_expr(expression),
            Stmt::Return { keyword, value } => {
                if self.current_function == FunctionType::None {
                    self.app
                        .error_token(keyword, "Can't return from top-level code.")
                }

                value.as_ref().map(|expr| {
                    if self.current_function == FunctionType::Initializer {
                        self.app
                            .error_token(keyword, "Can't return a value from an initializer.")
                    }

                    self.resolve_expr(expr)
                });
            }
            Stmt::Var { name, initializer } => {
                self.declare(name);
                if let Some(initializer) = initializer {
                    self.resolve_expr(initializer);
                }
                self.define(name);
            }
            Stmt::While { condition, body } => {
                self.resolve_expr(condition);
                self.resolve_stmt(body);
            }
        }
    }

    fn resolve_function(&mut self, stmt_function: &StmtFunction, function_type: FunctionType) {
        let enclosing_function = self.current_function;
        self.current_function = function_type;

        self.begin_scope();
        for param in &stmt_function.params {
            self.declare(&param);
            self.define(&param);
        }
        self.resolve(&stmt_function.body);
        self.end_scope();

        self.current_function = enclosing_function;
    }

    fn resolve_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Assign { name, value, id } => {
                self.resolve_expr(value);
                self.resolve_local(*id, &name);
            }
            Expr::Binary { left, right, .. } => {
                self.resolve_expr(left);
                self.resolve_expr(right);
            }
            Expr::Call {
                callee, arguments, ..
            } => {
                self.resolve_expr(callee);
                for argument in arguments {
                    self.resolve_expr(argument);
                }
            }
            Expr::Grouping { expression } => self.resolve_expr(expression),
            Expr::Literal { .. } => {}
            Expr::Logical { left, right, .. } => {
                self.resolve_expr(left);
                self.resolve_expr(right);
            }
            Expr::Unary { right, .. } => self.resolve_expr(right),
            Expr::Variable(id, ExprVariable { name }) => {
                if let Some(scope) = self.scopes.last() {
                    if let Some(defined) = scope.get(&name.lexeme) {
                        if !defined {
                            self.app.error_token(
                                name,
                                "Can't read local variable in its own initializer.",
                            );
                        }
                    }
                }
                self.resolve_local(*id, name);
            }
            Expr::Get { object, .. } => self.resolve_expr(object),
            Expr::Set { object, value, .. } => {
                self.resolve_expr(value);
                self.resolve_expr(object);
            }
            Expr::This { keyword, id } => {
                if self.current_class == ClassType::None {
                    self.app
                        .error_token(keyword, "Can't use 'this' outside of a class.");
                } else {
                    self.resolve_local(*id, keyword);
                }
            }
            Expr::Super { keyword, id, .. } => {
                if self.current_class == ClassType::None {
                    self.app
                        .error_token(keyword, "Can't use 'super' outside of a class.");
                } else if self.current_class != ClassType::Subclass {
                    self.app
                        .error_token(keyword, "Can't use 'super' in a class with no superclass.");
                }
                self.resolve_local(*id, keyword);
            }
        }
    }

    fn resolve_local(&mut self, id: ExprId, name: &Token) {
        for i in (0..self.scopes.len()).rev() {
            if self.scopes[i].contains_key(&name.lexeme) {
                self.interpreter.resolve(id, self.scopes.len() - 1 - i);
                return;
            }
        }
    }

    fn declare(&mut self, name: &Token) {
        if let Some(scope) = self.scopes.last_mut() {
            if scope.contains_key(&name.lexeme) {
                self.app
                    .error_token(name, "Already a variable with this name in in this scope.")
            }
            scope.insert(name.lexeme.clone(), false);
        }
    }

    fn define(&mut self, name: &Token) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.lexeme.clone(), true);
        }
    }

    fn begin_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn end_scope(&mut self) {
        self.scopes.pop();
    }
}

struct Class {
    name: Symbol,
    methods: HashMap<Symbol, Value>,
    superclass: Option<Rc<Class>>,
}

impl Class {
    fn find_method(&self, name: Symbol) -> Option<Value> {
        self.methods.get(&name).cloned().or_else(|| {
            self.superclass
                .as_ref()
                .map(|superclass| superclass.find_method(name))
                .flatten()
        })
    }
}

struct Instance {
    class: Rc<Class>,
    fields: RefCell<HashMap<Symbol, Value>>,
}

trait RcInstanceExt {
    fn get(&self, interner: &Interner, name: &Token) -> Result<Value, ErrCause>;
}

impl RcInstanceExt for Rc<Instance> {
    fn get(&self, interner: &Interner, name: &Token) -> Result<Value, ErrCause> {
        if let Some(value) = self.fields.borrow().get(&name.lexeme) {
            Ok(value.clone())
        } else if let Some(method) = self.class.find_method(name.lexeme) {
            if let Value::Callable(function) = method {
                if let Function::Declared(..) = &*Rc::borrow(&function) {
                    Ok(Value::Callable(Rc::new(
                        function.bind(interner, Rc::clone(self)),
                    )))
                } else {
                    unreachable!()
                }
            } else {
                unreachable!()
            }
        } else {
            Err(ErrCause::Error(
                name.clone(),
                format!("Undefined property '{}'.", interner.resolve(name.lexeme)),
            ))
        }
    }
}

impl Instance {
    fn new(class: Rc<Class>) -> Instance {
        Instance {
            class,
            fields: RefCell::new(HashMap::new()),
        }
    }

    fn find_method(&self, name: Symbol) -> Option<Value> {
        self.class.find_method(name)
    }

    fn set(&self, name: &Token, value: Value) {
        self.fields.borrow_mut().insert(name.lexeme.clone(), value);
    }
}
