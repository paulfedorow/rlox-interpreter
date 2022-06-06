mod ast;
mod interner;
mod interpreter;
mod parser;
mod scanner;

use crate::interner::{Interner, Symbol};
use crate::interpreter::{Interpreter, Resolver};
use crate::parser::Parser;
use crate::scanner::{Scanner, Token, TokenType};
use std::cell::Cell;
use std::io::{BufRead, Write};
use std::{env, fs, io, str};

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

pub struct App {
    had_error: Cell<bool>,
    had_runtime_error: Cell<bool>,
    interner: interner::Interner,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use walkdir::WalkDir;

    #[test]
    fn test_compliance() {
        let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        let mut exe_path = root_dir.clone();
        exe_path.push(format!(
            "target/debug/rlox-interpreter{}",
            std::env::consts::EXE_SUFFIX
        ));

        assert!(
            exe_path.exists(),
            "rlox-interpreter executable not found. Run cargo build first."
        );

        let mut resources_dir = root_dir;
        resources_dir.push("resources/compliance_tests");

        for lox_file in WalkDir::new(resources_dir)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .map(|s| s.ends_with(".lox"))
                    .unwrap_or(false)
            })
        {
            let output = std::process::Command::new(exe_path.clone())
                .args([lox_file.path()])
                .output()
                .unwrap();

            let lox_file_path = lox_file.path().to_str().unwrap();

            let expected_out = fs::read_to_string(String::from(lox_file_path) + ".out").unwrap();
            let expected_err = fs::read_to_string(String::from(lox_file_path) + ".err").unwrap();

            assert_eq!(
                String::from_utf8(output.stdout).unwrap(),
                expected_out,
                "Unexpected stdin-output for {}.",
                lox_file_path
            );

            assert_eq!(
                String::from_utf8(output.stderr).unwrap(),
                expected_err,
                "Unexpected stderr-output for {}.",
                lox_file_path
            );
        }
    }
}
