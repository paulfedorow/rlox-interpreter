use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::os::linux::raw::stat;
use std::str::FromStr;
use std::{env, fs, io, str};

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut app = App::new();
    let mut interpreter = Interpreter::new();

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
    had_error: bool,
    had_runtime_error: bool,
}

impl App {
    fn new() -> App {
        return App {
            had_error: false,
            had_runtime_error: false,
        };
    }

    fn error(&mut self, line: u64, message: &str) {
        self.report(line, "", message);
    }

    fn error_token(&mut self, token: &Token, message: &str) {
        if token.token_type == TokenType::Eof {
            self.report(token.line, " at end", message);
        } else {
            self.report(token.line, &format!(" at '{}'", token.lexeme), message);
        }
    }

    fn runtime_error(&mut self, token: &Token, message: &str) {
        self.had_runtime_error = true;
        eprintln!("{}\n[line {}]", message, token.line);
    }

    fn report(&mut self, line: u64, origin: &str, message: &str) {
        self.had_error = true;
        eprintln!("[line {}] Error{}: {}", line, origin, message);
    }

    fn run_file(&mut self, interpreter: &mut Interpreter, path: &str) {
        match fs::read_to_string(path) {
            Ok(content) => {
                println!("{}", content.len());
                self.run(interpreter, &content);
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

    fn run_prompt(&mut self, interpreter: &mut Interpreter) {
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
                    self.had_error = false;
                }
                Err(error) => {
                    println!("Error: {}", error);
                    std::process::exit(70);
                }
            }
        }
    }

    fn run(&mut self, interpreter: &mut Interpreter, source: &str) {
        let mut scanner = Scanner::new(self, source.as_bytes());
        let tokens = scanner.scan_tokens();
        let mut parser = Parser::new(self, tokens);
        let statements = parser.parse();

        if self.had_error {
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
        self.add_token_with_literal(token_type, TokenLiteral::Nil);
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
    Bool(bool),
    Nil,
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

#[derive(Debug)]
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

#[derive(Debug)]
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
    app: &'a mut App,
}

impl Parser<'_> {
    fn new(app: &mut App, tokens: Vec<Token>) -> Parser {
        Parser {
            tokens,
            current: 0,
            app,
        }
    }

    fn expression(&mut self) -> Option<Expr> {
        self.assignment()
    }

    fn assignment(&mut self) -> Option<Expr> {
        let expr = self.equality()?;

        if self.match_one_of([TokenType::Equal]) {
            let value = self.assignment()?;
            let equals = self.previous_token();

            match expr {
                Expr::Variable(ExprVariable { name }) => {
                    return Some(Expr::Assign {
                        name,
                        value: Box::from(value),
                    });
                }
                _ => {
                    self.app
                        .error_token(&equals.clone(), "Invalid assignment target.");
                    return None;
                }
            }
        }

        Some(expr)
    }

    fn equality(&mut self) -> Option<Expr> {
        let mut expr = self.comparison();

        if expr.is_none() {
            return None;
        }

        while self.match_one_of([TokenType::BangEqual, TokenType::BangEqual]) {
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

        if expr.is_none() {
            return None;
        }

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

        if expr.is_none() {
            return None;
        }

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

        if expr.is_none() {
            return None;
        }

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
            self.primary()
        }
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
            Some(Expr::Variable(ExprVariable {
                name: self.previous_token().clone(),
            }))
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

    fn parse(&mut self) -> Vec<Stmt> {
        let mut statements = Vec::new();

        while !self.is_at_end() {
            match self.declaration() {
                Some(statement) => statements.push(statement),
                _ => {}
            }
        }

        statements
    }

    fn statement(&mut self) -> Option<Stmt> {
        if self.match_one_of([TokenType::Print]) {
            self.print_statement()
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

    fn print_statement(&mut self) -> Option<Stmt> {
        let expression = self.expression()?;
        self.consume(TokenType::Semicolon, "Expect ';' after value.")?;
        Some(Stmt::Print { expression })
    }

    fn expression_statement(&mut self) -> Option<Stmt> {
        let expression = self.expression()?;
        self.consume(TokenType::Semicolon, "Expect ';' after expression.")?;
        Some(Stmt::Expression(expression))
    }

    fn declaration(&mut self) -> Option<Stmt> {
        if self.match_one_of([TokenType::Var]) {
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

        return false;
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

#[derive(Clone, Debug, PartialEq)]
enum Value {
    String(String),
    Number(f64),
    Bool(bool),
    Nil,
}

struct Interpreter {
    environment: Environment,
}

impl Interpreter {
    fn new() -> Interpreter {
        Interpreter {
            environment: Environment::new(None),
        }
    }

    fn interpret(&mut self, app: &mut App, statements: &[Stmt]) {
        for statement in statements {
            match self.execute(statement) {
                Ok(_) => {}
                Err((token, message)) => {
                    app.runtime_error(&token, &message);
                    break;
                }
            }
        }
    }

    fn execute(&mut self, statement: &Stmt) -> Result<(), (Token, String)> {
        match statement {
            Stmt::Expression(expr) => {
                self.evaluate(expr)?;
            }
            Stmt::Print { expression } => {
                let value = self.evaluate(expression)?;
                println!("{}", stringify(&value));
            }
            Stmt::Var { name, initializer } => {
                let value = match initializer {
                    Some(expr) => self.evaluate(&expr)?,
                    _ => Value::Nil,
                };
                self.environment.define(name.lexeme.clone(), value.clone());
            }
            Stmt::Block { statements } => {
                let previous = self.environment.clone();
                self.environment = Environment::new(Some(self.environment.clone()));

                let mut ret = Ok(());
                for statement in statements {
                    ret = self.execute(statement);
                    if ret.is_err() {
                        break;
                    }
                }

                self.environment = previous;

                return ret;
            }
            _ => panic!("Statement node is not supported yet."),
        }
        Ok(())
    }

    fn evaluate(&mut self, expr: &Expr) -> Result<Value, (Token, String)> {
        match expr {
            Expr::Binary {
                left,
                operator,
                right,
            } => {
                let left = self.evaluate(left)?;
                let right = self.evaluate(right)?;

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
                            Ok(Value::String(left_str + &right_str))
                        }
                        _ => Err((
                            operator.clone(),
                            "Operands must be two numbers or two strings.".to_string(),
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
            Expr::Grouping { expression } => self.evaluate(expression),
            Expr::Literal { value } => match value {
                TokenLiteral::String(str) => Ok(Value::String(str.clone())),
                TokenLiteral::Number(num) => Ok(Value::Number(*num)),
                TokenLiteral::Bool(bool) => Ok(Value::Bool(*bool)),
                TokenLiteral::Nil => Ok(Value::Nil),
            },
            Expr::Unary { operator, right } => {
                let right = self.evaluate(right)?;

                match operator.token_type {
                    TokenType::Bang => Ok(Value::Bool(!is_truthy(&right))),
                    TokenType::Minus => {
                        let num = self.check_number_operand(operator, &right);
                        Ok(Value::Number(-(num?)))
                    }
                    _ => panic!("Unexpected unary operator token."),
                }
            }
            Expr::Variable(ExprVariable { name }) => {
                self.environment.get(name).map(|value| value.clone())
            }
            Expr::Assign { name, value } => {
                let value = self.evaluate(&value).map(|value| value.clone());
                self.environment.assign(&name, value.clone()?)?;
                value
            }
            _ => panic!("Expression node is not supported yet."),
        }
    }

    fn check_number_operand(
        &mut self,
        operator: &Token,
        operand: &Value,
    ) -> Result<f64, (Token, String)> {
        match operand {
            Value::Number(num) => Ok(*num),
            _ => Err((operator.clone(), String::from("Operand must be a number."))),
        }
    }

    fn check_number_operands(
        &mut self,
        operator: &Token,
        left: &Value,
        right: &Value,
    ) -> Result<(f64, f64), (Token, String)> {
        match (left, right) {
            (Value::Number(left_num), Value::Number(right_num)) => Ok((*left_num, *right_num)),
            _ => Err((operator.clone(), String::from("Operands must be a number."))),
        }
    }
}

fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Bool(bool) => bool.clone(),
        Value::Nil => false,
        _ => true,
    }
}

fn is_equal(left: &Value, right: &Value) -> bool {
    left == right
}

fn stringify(value: &Value) -> String {
    match value {
        Value::String(str) => str.to_string(),
        Value::Number(num) => format!("{}", num),
        Value::Bool(b) => {
            if *b {
                String::from("true")
            } else {
                String::from("false")
            }
        }
        Value::Nil => String::from("nil"),
    }
}

#[derive(Clone)]
struct Environment {
    values: HashMap<String, Value>,
    enclosing: Option<Box<Environment>>,
}

impl Environment {
    fn new(enclosing: Option<Environment>) -> Environment {
        Environment {
            values: HashMap::new(),
            enclosing: enclosing.map(|env| Box::from(env)),
        }
    }

    fn define(&mut self, name: String, value: Value) {
        self.values.insert(name, value);
    }

    fn assign(&mut self, name: &Token, value: Value) -> Result<(), (Token, String)> {
        match self.values.get(&name.lexeme) {
            Some(_) => {
                self.values.insert(name.lexeme.clone(), value);
                Ok(())
            }
            None => self.enclosing.as_mut().map_or(
                Err((
                    name.clone(),
                    format!("Undefined variable '{}'.", name.lexeme),
                )),
                |enclosing| enclosing.assign(name, value).clone(),
            ),
        }
    }

    fn get(&mut self, name: &Token) -> Result<&Value, (Token, String)> {
        match self.values.get(&name.lexeme) {
            Some(value) => Ok(value),
            None => self.enclosing.as_mut().map_or(
                Err((
                    name.clone(),
                    format!("Undefined variable '{}'.", name.lexeme),
                )),
                |enclosing| enclosing.get(name).clone(),
            ),
        }
    }
}
