use crate::parser::{Expression, Statement, AST};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum IRInstruction {
    // Stack Operations
    Pop,
    Dup,

    // Constants
    PushConst(Constant), // Unified push constant instruction

    // Variables
    Load(String),  // Load from any scope (local/global)
    Store(String), // Store to any scope (local/global)

    // Arithmetic/Logic
    Binary(BinaryOp), // All binary operations
    Unary(UnaryOp),   // All unary operations

    // Control Flow
    Label(String),
    Jump(String),   // Unconditional jump
    JumpIf(String), // Conditional jump

    // Function Operations
    Call(String, u16), // Function name, argument count
    Return(bool),      // bool indicates if returning value
}

#[derive(Debug, Clone)]
pub enum BinaryOp {
    Add, // +
    Sub, // -
    Mul, // *
    Div, // /
    Eq,  // ==
    Lt,  // <
    Gt,  // >
    Ge,  // >=
    Le,  // <=
    And, // &&
    Or,  // ||
}

#[derive(Debug, Clone)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone)]
pub enum Constant {
    Null,
    Number(f64),
    String(String),
    Boolean(bool),
}

#[derive(Debug, Clone)]
pub struct IRFunction {
    pub name: String,
    pub params: Vec<String>,
    pub max_stack: u16,
    pub max_locals: u16,
    pub instructions: Vec<IRInstruction>,
    pub exception_table: Vec<ExceptionHandler>,
}

#[derive(Debug, Clone)]
pub struct ExceptionHandler {
    pub start_label: String,
    pub end_label: String,
    pub handler_label: String,
    pub exception_type: String,
}

#[derive(Debug)]
pub struct IRModule {
    pub functions: Vec<IRFunction>,
    pub constants: Vec<Constant>,
}

impl IRModule {
    fn new() -> Self {
        IRModule {
            functions: Vec::new(),
            constants: Vec::new(),
        }
    }

    fn add_function(&mut self, function: IRFunction) {
        self.functions.push(function);
    }

    fn add_constant(&mut self, constant: Constant) -> usize {
        self.constants.push(constant);
        self.constants.len() - 1
    }
}

struct IRBuilder {
    current_function: IRFunction,
    label_counter: usize,
    local_vars: HashMap<String, u16>,
    next_local: u16,
}

impl IRBuilder {
    fn new(name: String) -> Self {
        IRBuilder {
            current_function: IRFunction {
                name,
                params: Vec::new(),
                max_stack: 0,
                max_locals: 0,
                instructions: Vec::new(),
                exception_table: Vec::new(),
            },
            label_counter: 0,
            local_vars: HashMap::new(),
            next_local: 0,
        }
    }

    fn generate_label(&mut self) -> String {
        self.label_counter += 1;
        format!("L{}", self.label_counter)
    }

    fn allocate_local(&mut self, name: &str) -> u16 {
        let idx = self.next_local;
        self.local_vars.insert(name.to_string(), idx);
        self.next_local += 1;
        self.current_function.max_locals = self.next_local;
        idx
    }

    fn emit(&mut self, instruction: IRInstruction) {
        self.current_function.instructions.push(instruction);
    }

    fn get_or_create_local(&mut self, name: &str) -> u16 {
        if let Some(&idx) = self.local_vars.get(name) {
            idx
        } else {
            self.allocate_local(name)
        }
    }
}

pub fn lower_ast(ast: AST) -> IRModule {
    let mut module = IRModule::new();

    for statement in ast.statements {
        match statement {
            Statement::FunctionDeclaration { name, params, body } => {
                let mut builder = IRBuilder::new(name.clone());

                // Store params in the IRFunction
                builder.current_function.params = params.clone();

                // Allocate parameters as local variables
                for param in params {
                    let idx = builder.allocate_local(&param);
                    // Load parameter from the local variable
                    builder.emit(IRInstruction::Load(param.clone()));
                    builder.emit(IRInstruction::Store(param));
                }

                // Lower function body
                for stmt in body {
                    lower_statement(&mut builder, stmt);
                }

                // Add implicit return if needed
                if !matches!(
                    builder.current_function.instructions.last(),
                    Some(IRInstruction::Return(_))
                ) {
                    builder.emit(IRInstruction::Return(false));
                }

                module.add_function(builder.current_function);
            }
            _ => {}
        }
    }

    module
}

fn lower_statement(builder: &mut IRBuilder, stmt: Statement) {
    match stmt {
        Statement::Return(Some(expr)) => {
            lower_expression(builder, expr);
            builder.emit(IRInstruction::Return(true));
        }
        Statement::Return(None) => {
            builder.emit(IRInstruction::Return(false));
        }
        Statement::Let { name, initializer } => {
            lower_expression(builder, initializer);
            let local_idx = builder.get_or_create_local(&name);
            builder.emit(IRInstruction::Store(name));
        }
        Statement::ExpressionStatement(expr) => {
            lower_expression(builder, expr);
            builder.emit(IRInstruction::Pop);
        }
        Statement::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let else_label = builder.generate_label();
            let end_label = builder.generate_label();

            // Compile condition
            lower_expression(builder, condition);
            builder.emit(IRInstruction::JumpIf(else_label.clone()));

            // Compile then branch
            for stmt in then_branch {
                lower_statement(builder, stmt);
            }
            builder.emit(IRInstruction::Jump(end_label.clone()));

            // Compile else branch if it exists
            builder.emit(IRInstruction::Label(else_label));
            if let Some(else_stmts) = else_branch {
                for stmt in else_stmts {
                    lower_statement(builder, stmt);
                }
            }
            builder.emit(IRInstruction::Label(end_label));
        }
        Statement::While { condition, body } => {
            let start_label = builder.generate_label();
            let end_label = builder.generate_label();

            builder.emit(IRInstruction::Label(start_label.clone()));
            lower_expression(builder, condition);
            builder.emit(IRInstruction::JumpIf(end_label.clone()));

            for stmt in body {
                lower_statement(builder, stmt);
            }
            builder.emit(IRInstruction::Jump(start_label));
            builder.emit(IRInstruction::Label(end_label));
        }
        Statement::Block(statements) => {
            for stmt in statements {
                lower_statement(builder, stmt);
            }
        }
        Statement::FunctionDeclaration { name, .. } => {
            // Function declarations are handled at the module level
            builder.emit(IRInstruction::PushConst(Constant::String(name.clone())));
            builder.emit(IRInstruction::Store(name));
        }
    }
}

fn lower_expression(builder: &mut IRBuilder, expr: Expression) {
    match expr {
        Expression::Number(n) => {
            builder.emit(IRInstruction::PushConst(Constant::Number(n)));
        }
        Expression::String(s) => {
            builder.emit(IRInstruction::PushConst(Constant::String(s)));
        }
        Expression::Boolean(b) => {
            builder.emit(IRInstruction::PushConst(Constant::Boolean(b)));
        }
        Expression::Null => {
            builder.emit(IRInstruction::PushConst(Constant::Null));
        }
        Expression::Identifier(name) => {
            builder.emit(IRInstruction::Load(name));
        }
        Expression::FunctionCall { name, arguments } => {
            // Push arguments in reverse order
            let count = arguments.len();
            for arg in arguments.into_iter().rev() {
                lower_expression(builder, arg);
            }
            builder.emit(IRInstruction::Call(name, count as u16));
        }
        Expression::BinaryOp { op, left, right } => {
            lower_expression(builder, *left);
            lower_expression(builder, *right);

            let op = match op.as_str() {
                "+" => BinaryOp::Add,
                "-" => BinaryOp::Sub,
                "*" => BinaryOp::Mul,
                "/" => BinaryOp::Div,
                "==" => BinaryOp::Eq,
                "<" => BinaryOp::Lt,
                ">" => BinaryOp::Gt,
                "<=" => BinaryOp::Le,
                ">=" => BinaryOp::Ge,
                "&&" => {
                    // Short-circuit evaluation for &&
                    let end_label = builder.generate_label();
                    let false_label = builder.generate_label();
                    builder.emit(IRInstruction::Dup);
                    builder.emit(IRInstruction::JumpIf(false_label.clone()));
                    builder.emit(IRInstruction::Pop);
                    builder.emit(IRInstruction::Jump(end_label.clone()));
                    builder.emit(IRInstruction::Label(false_label));
                    builder.emit(IRInstruction::Label(end_label));
                    return;
                }
                "||" => {
                    // Short-circuit evaluation for ||
                    let end_label = builder.generate_label();
                    let true_label = builder.generate_label();
                    builder.emit(IRInstruction::Dup);
                    builder.emit(IRInstruction::JumpIf(true_label.clone()));
                    builder.emit(IRInstruction::Pop);
                    builder.emit(IRInstruction::Jump(end_label.clone()));
                    builder.emit(IRInstruction::Label(true_label));
                    builder.emit(IRInstruction::Label(end_label));
                    return;
                }
                _ => panic!("Unsupported binary operator: {}", op),
            };
            builder.emit(IRInstruction::Binary(op));
        }
        Expression::UnaryOp { op, expr } => {
            lower_expression(builder, *expr);
            let op = match op.as_str() {
                "-" => UnaryOp::Neg,
                "!" => UnaryOp::Not,
                _ => panic!("Unsupported unary operator: {}", op),
            };
            builder.emit(IRInstruction::Unary(op));
        }
        Expression::Conditional {
            condition,
            then_expr,
            else_expr,
        } => {
            let else_label = builder.generate_label();
            let end_label: String = builder.generate_label();

            lower_expression(builder, *condition);
            builder.emit(IRInstruction::JumpIf(else_label.clone()));

            lower_expression(builder, *then_expr);
            builder.emit(IRInstruction::Jump(end_label.clone()));

            builder.emit(IRInstruction::Label(else_label));
            lower_expression(builder, *else_expr);
            builder.emit(IRInstruction::Label(end_label));
        }
    }
}
