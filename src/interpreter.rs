use crate::ast::{Expr, ExprId, ExprVariable, Stmt, StmtFunction};
use crate::interner::{Interner, Symbol};
use crate::scanner::{Token, TokenLiteral, TokenType};
use crate::App;
use rustc_hash::FxHashMap;
use std::borrow::Borrow;
use std::cell::RefCell;
use std::rc::Rc;
use std::time;
use vec_map::VecMap;

pub struct Interpreter {
    global_environment: Rc<Environment>,
    environment: Rc<Environment>,
    locals: VecMap<usize>,
}

impl Interpreter {
    pub fn new(interner: &Interner) -> Interpreter {
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
            locals: VecMap::default(),
        }
    }

    pub fn interpret(&mut self, app: &App, statements: &[Stmt]) {
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
                self.environment.define(name.lexeme, value);
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

                self.environment.define(function_stmt.name.lexeme, function);
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
                    } else if let Expr::Variable(_, superclass) = superclass {
                        return Err(ErrCause::Error(
                            superclass.name.clone(),
                            String::from("Superclass must be a class."),
                        ));
                    } else {
                        unreachable!();
                    }
                } else {
                    None
                };

                self.environment.define(name.lexeme, Value::Nil);

                let environment = if let Some(superclass) = &superclass_value {
                    let environment = Rc::new(Environment::new(Some(Rc::clone(&self.environment))));
                    environment.define(interner.sym_super, superclass.clone());
                    environment
                } else {
                    Rc::clone(&self.environment)
                };

                let mut initializer_arity = None;
                let mut class_methods = FxHashMap::default();
                for method in methods {
                    let is_initializer = method.name.lexeme == interner.sym_init;
                    if is_initializer {
                        initializer_arity = Some(method.params.len());
                    }
                    let function = Value::Callable(Rc::new(Function::Declared(
                        method.clone(),
                        Rc::clone(&environment),
                        is_initializer,
                    )));
                    class_methods.insert(method.name.lexeme, function);
                }

                let superclass = superclass_value.and_then(|superclass| superclass.to_class());

                let initializer_arity = initializer_arity
                    .or_else(|| {
                        superclass.as_ref().and_then(|superclass| {
                            superclass.find_method(interner.sym_init).and_then(|init| {
                                if let Value::Callable(function) = init {
                                    if let Function::Declared(stmt_function, ..) =
                                        Rc::borrow(&function)
                                    {
                                        Some(stmt_function.params.len())
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            })
                        })
                    })
                    .unwrap_or(0);

                let class = Value::Callable(Rc::new(Function::Class(
                    initializer_arity,
                    Rc::new(Class {
                        name: name.lexeme,
                        methods: class_methods,
                        superclass,
                    }),
                )));

                self.environment.assign(interner, name, class)?;
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
                        (Value::InternedString(left_str), Value::String(right_str)) => Ok(
                            Value::String(Rc::from(interner.resolve(left_str) + &*right_str)),
                        ),
                        (Value::String(left_str), Value::InternedString(right_str)) => {
                            Ok(Value::String(Rc::from(
                                (&*left_str).clone() + &interner.resolve(right_str),
                            )))
                        }
                        (Value::InternedString(left_str), Value::InternedString(right_str)) => {
                            Ok(Value::String(Rc::from(
                                interner.resolve(left_str) + &interner.resolve(right_str),
                            )))
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
                    TokenType::BangEqual => Ok(Value::Bool(!is_equal(&left, &right, interner))),
                    TokenType::EqualEqual => Ok(Value::Bool(is_equal(&left, &right, interner))),
                    _ => panic!("Unexpected binary operator token."),
                }
            }
            Expr::Grouping { expression } => self.evaluate(interner, expression),
            Expr::Literal { value } => match value {
                TokenLiteral::String(sym) => Ok(Value::InternedString(*sym)),
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
                if let Some(distance) = self.locals.get(id.0).cloned() {
                    Environment::assign_at(
                        interner,
                        &self.environment,
                        distance,
                        name,
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
                            "Expected {} arguments but got {}.",
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
                let distance = self.locals.get(id.0).cloned().unwrap();
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
                        format!("Undefined property '{}'.", interner.resolve(method.lexeme)),
                    )),
                    _ => unreachable!(),
                }
            }
        }
    }

    fn resolve(&mut self, id: ExprId, depth: usize) {
        self.locals.insert(id.0, depth);
    }

    fn look_up_variable(
        &mut self,
        interner: &Interner,
        name: &Token,
        id: ExprId,
    ) -> Result<Value, ErrCause> {
        let distance = self.locals.get(id.0);
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
                String::from("Operands must be numbers."),
            )),
        }
    }
}

pub struct Resolver<'a> {
    app: &'a App,
    interpreter: &'a mut Interpreter,
    scopes: Vec<FxHashMap<Symbol, bool>>,
    current_function: FunctionType,
    current_class: ClassType,
}

impl Resolver<'_> {
    pub fn new<'a>(app: &'a App, interpreter: &'a mut Interpreter) -> Resolver<'a> {
        Resolver {
            app,
            interpreter,
            scopes: Vec::new(),
            current_function: FunctionType::None,
            current_class: ClassType::None,
        }
    }

    pub fn resolve(&mut self, statements: &[Stmt]) {
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

                self.resolve_function(function, FunctionType::Function);
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
                    self.resolve_function(method, declaration);
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
                if let Some(stmt) = else_branch.as_ref() {
                    self.resolve_stmt(stmt)
                }
            }
            Stmt::Print { expression } => self.resolve_expr(expression),
            Stmt::Return { keyword, value } => {
                if self.current_function == FunctionType::None {
                    self.app
                        .error_token(keyword, "Can't return from top-level code.")
                }

                if let Some(expr) = value.as_ref() {
                    if self.current_function == FunctionType::Initializer {
                        self.app
                            .error_token(keyword, "Can't return a value from an initializer.")
                    }

                    self.resolve_expr(expr)
                }
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
            self.declare(param);
            self.define(param);
        }
        self.resolve(&stmt_function.body);
        self.end_scope();

        self.current_function = enclosing_function;
    }

    fn resolve_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Assign { name, value, id } => {
                self.resolve_expr(value);
                self.resolve_local(*id, name);
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
                    .error_token(name, "Already a variable with this name in this scope.")
            }
            scope.insert(name.lexeme, false);
        }
    }

    fn define(&mut self, name: &Token) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.lexeme, true);
        }
    }

    fn begin_scope(&mut self) {
        self.scopes.push(FxHashMap::default());
    }

    fn end_scope(&mut self) {
        self.scopes.pop();
    }
}

#[derive(Clone)]
enum Value {
    String(Rc<String>),
    InternedString(Symbol),
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
            matches!(Rc::borrow(function), Function::Class(..))
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
    Declared(Rc<StmtFunction>, Rc<Environment>, bool),
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
            Function::Declared(stmt_function, closure, is_initializer) => {
                let StmtFunction { params, body, .. } = Rc::borrow(stmt_function);

                let environment = Environment::new(Some(Rc::clone(closure)));

                for i in 0..params.len() {
                    environment.define(params[i].lexeme, arguments[i].clone())
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
            Function::Declared(
                Rc::clone(stmt_function),
                Rc::new(environment),
                *is_initializer,
            )
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

enum ErrCause {
    Error(Token, String),
    Return(Value),
}

fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Bool(bool) => *bool,
        Value::Nil => false,
        _ => true,
    }
}

fn is_equal(left: &Value, right: &Value, interner: &Interner) -> bool {
    match (left, right) {
        (Value::String(l), Value::String(r)) => Rc::ptr_eq(l, r),
        (Value::InternedString(l), Value::String(r)) => interner.resolve(*l) == **r,
        (Value::String(l), Value::InternedString(r)) => **l == interner.resolve(*r),
        (Value::InternedString(l), Value::InternedString(r)) => l == r,
        (Value::Number(l), Value::Number(r)) => l == r,
        (Value::Bool(l), Value::Bool(r)) => l == r,
        (Value::Nil, Value::Nil) => true,
        (Value::Instance(l), Value::Instance(r)) => Rc::ptr_eq(l, r),
        (Value::Callable(l), Value::Callable(r)) => Rc::ptr_eq(l, r),
        (_, _) => false,
    }
}

fn stringify(interner: &Interner, value: &Value) -> String {
    match value {
        Value::String(str) => str.as_ref().clone(),
        Value::InternedString(sym) => interner.resolve(*sym),
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
            Function::Declared(stmt_function, ..) => {
                let StmtFunction { name, .. } = Rc::borrow(stmt_function);
                format!("<fn {}>", interner.resolve(name.lexeme))
            }
            Function::Class(_, class) => interner.resolve(class.name),
        },
        Value::Instance(instance) => {
            format!("{} instance", interner.resolve(instance.class.name))
        }
    }
}

#[derive(Clone)]
struct Environment {
    values: RefCell<FxHashMap<Symbol, Value>>,
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
            values: RefCell::new(FxHashMap::default()),
            enclosing,
        }
    }

    fn define(&self, name: Symbol, value: Value) {
        self.values.borrow_mut().insert(name, value);
    }

    fn assign(&self, interner: &Interner, name: &Token, value: Value) -> Result<(), ErrCause> {
        if self.values.borrow().get(&name.lexeme).is_some() {
            self.values.borrow_mut().insert(name.lexeme, value);
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

struct Class {
    name: Symbol,
    methods: FxHashMap<Symbol, Value>,
    superclass: Option<Rc<Class>>,
}

impl Class {
    fn find_method(&self, name: Symbol) -> Option<Value> {
        self.methods.get(&name).cloned().or_else(|| {
            self.superclass
                .as_ref()
                .and_then(|superclass| superclass.find_method(name))
        })
    }
}

struct Instance {
    class: Rc<Class>,
    fields: RefCell<FxHashMap<Symbol, Value>>,
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
            fields: RefCell::new(FxHashMap::default()),
        }
    }

    fn find_method(&self, name: Symbol) -> Option<Value> {
        self.class.find_method(name)
    }

    fn set(&self, name: &Token, value: Value) {
        self.fields.borrow_mut().insert(name.lexeme, value);
    }
}
