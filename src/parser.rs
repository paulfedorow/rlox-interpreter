use crate::ast::{Expr, ExprId, ExprVariable, Stmt, StmtFunction};
use crate::scanner::{Token, TokenLiteral, TokenType};
use crate::App;
use std::rc::Rc;

pub struct Parser<'a> {
    tokens: Vec<Token>,
    current: usize,
    app: &'a App,
    expr_id_count: usize,
}

impl Parser<'_> {
    pub fn new(app: &App, tokens: Vec<Token>) -> Parser {
        Parser {
            tokens,
            current: 0,
            app,
            expr_id_count: 0,
        }
    }

    pub fn parse(&mut self) -> Vec<Stmt> {
        let mut statements = Vec::new();

        while !self.is_at_end() {
            if let Some(statement) = self.declaration() {
                statements.push(statement);
            }
        }

        statements
    }

    fn expression(&mut self) -> Option<Expr> {
        self.assignment()
    }

    fn assignment(&mut self) -> Option<Expr> {
        let expr = self.or()?;

        if self.match_one_of([TokenType::Equal]) {
            let equals = self.previous_token().clone();
            let value = self.assignment()?;

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
        id
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
            if let Some(declaration) = self.declaration() {
                statements.push(declaration);
            }
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
        let declaration = if self.match_one_of([TokenType::Class]) {
            self.class_declaration()
        } else if self.match_one_of([TokenType::Fun]) {
            self.function("function")
                .map(|f| Stmt::Function(Rc::new(f)))
        } else if self.match_one_of([TokenType::Var]) {
            self.var_declaration()
        } else {
            self.statement()
        };

        if declaration.is_none() {
            self.synchronize()
        }

        declaration
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
            methods.push(Rc::new(self.function("method")?));
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
            initializer = Some(self.expression()?);
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
