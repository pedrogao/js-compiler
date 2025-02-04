use crate::debug::DebugTrace;
use crate::ir::{BinaryOp, Constant, IRFunction, IRInstruction, IRModule, UnaryOp};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Number(f64),
    String(String),
    Boolean(bool),
    Object(HashMap<String, Value>),
    Undefined,
}

impl Value {
    fn from_constant(constant: &Constant) -> Self {
        match constant {
            Constant::Null => Value::Null,
            Constant::Number(n) => Value::Number(*n),
            Constant::String(s) => Value::String(s.clone()),
            Constant::Boolean(b) => Value::Boolean(*b),
        }
    }
}

type NativeFunction = fn(Vec<Value>) -> Value;

pub struct VMContext {
    stack: Vec<Value>,
    locals: HashMap<String, Value>, // Change from Vec to HashMap for better scoping
    globals: HashMap<String, Value>,
    functions: HashMap<String, Function>,
    frames: Vec<CallFrame>,
}

#[derive(Clone)]
enum Function {
    IR(IRFunction),
    Native(NativeFunction),
}

struct CallFrame {
    function: IRFunction,
    ip: usize,
    locals: HashMap<String, Value>, // Local variables for this frame
    stack_base: usize,              // Stack pointer at frame start
}

impl CallFrame {
    fn new(function: IRFunction, stack_base: usize) -> Self {
        Self {
            function,
            ip: 0,
            locals: HashMap::new(),
            stack_base,
        }
    }
}

impl VMContext {
    fn new(module: &IRModule) -> Self {
        let mut functions = HashMap::new();

        // Add built-in functions
        functions.insert("print".to_string(), Function::Native(native_print));

        // Add user-defined functions
        for func in &module.functions {
            functions.insert(func.name.clone(), Function::IR(func.clone()));
        }

        VMContext {
            stack: Vec::with_capacity(1024),
            locals: HashMap::new(),
            globals: HashMap::new(),
            functions,
            frames: Vec::new(),
        }
    }

    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    fn pop(&mut self) -> Value {
        self.stack.pop().unwrap_or(Value::Undefined)
    }

    fn get_local(&self, name: &str) -> Value {
        // First check current frame's locals
        if let Some(frame) = self.frames.last() {
            if let Some(value) = frame.locals.get(name) {
                return value.clone();
            }
        }
        // Then check globals
        self.globals.get(name).cloned().unwrap_or(Value::Undefined)
    }

    fn set_local(&mut self, name: String, value: Value) {
        if let Some(frame) = self.frames.last_mut() {
            // First try to update existing local
            if frame.locals.contains_key(&name) {
                frame.locals.insert(name, value);
            } else {
                // If not found in current frame, set as global
                self.globals.insert(name, value);
            }
        } else {
            // No active frame, set as global
            self.globals.insert(name, value);
        }
    }
}

pub struct VM {
    context: VMContext,
    debug_trace: Option<DebugTrace>,
}

impl VM {
    pub fn new(module: IRModule) -> Self {
        VM {
            context: VMContext::new(&module),
            debug_trace: None,
        }
    }

    pub fn enable_debugging(&mut self) {
        self.debug_trace = Some(DebugTrace::new());
    }

    pub fn execute_function(&mut self, name: &str, args: Vec<Value>) -> Value {
        match self.context.functions.get(name).cloned() {
            Some(Function::IR(function)) => {
                let stack_base = self.context.stack.len();
                let mut frame = CallFrame::new(function, stack_base);
                let mut return_value = Value::Undefined;

                // Set up parameters as locals
                for (param, arg) in frame.function.params.iter().zip(args) {
                    frame.locals.insert(param.clone(), arg);
                }

                self.context.frames.push(frame);

                // Execute until frame returns
                loop {
                    let current_frame = self.context.frames.last_mut().unwrap();
                    if current_frame.ip >= current_frame.function.instructions.len() {
                        let stack_base = current_frame.stack_base;
                        // Get any value left on the stack as implicit return
                        if self.context.stack.len() > stack_base {
                            return_value = self.context.pop();
                        }
                        self.context.frames.pop();
                        self.context.stack.truncate(stack_base);
                        break;
                    }

                    let instruction = current_frame.function.instructions[current_frame.ip].clone();
                    current_frame.ip += 1;

                    // Handle explicit returns
                    if let IRInstruction::Return(has_value) = &instruction {
                        let stack_base = current_frame.stack_base;
                        if *has_value {
                            return_value = self.context.pop();
                        }
                        self.context.frames.pop();
                        self.context.stack.truncate(stack_base);
                        break;
                    }

                    self.execute_instruction(instruction);
                }

                return_value
            }
            Some(Function::Native(func)) => func(args),
            None => panic!("Function {} not found", name),
        }
    }

    fn execute_instruction(&mut self, instruction: IRInstruction) {
        // Record debug info before execution
        if let Some(debug_trace) = &mut self.debug_trace {
            if let Some(frame) = self.context.frames.last() {
                debug_trace.add_frame(
                    &instruction,
                    &self.context.stack,
                    &frame.locals,
                    frame.ip - 1,
                    &frame.function.name,
                );
            }
        }

        match instruction {
            IRInstruction::Pop => {
                self.context.pop();
            }
            IRInstruction::Dup => {
                let value = self
                    .context
                    .stack
                    .last()
                    .cloned()
                    .unwrap_or(Value::Undefined);
                self.context.push(value);
            }
            IRInstruction::PushConst(constant) => {
                self.context.push(Value::from_constant(&constant));
            }
            IRInstruction::Load(name) => {
                let value = self.context.get_local(&name);
                self.context.push(value);
            }
            IRInstruction::Store(name) => {
                let value = self.context.pop();
                self.context.set_local(name, value);
            }
            IRInstruction::Binary(op) => {
                let right = self.context.pop();
                let left = self.context.pop();
                let result = match op {
                    BinaryOp::Add => self.binary_add(left, right),
                    BinaryOp::Sub => self.binary_sub(left, right),
                    BinaryOp::Mul => self.binary_mul(left, right),
                    BinaryOp::Div => self.binary_div(left, right),
                    BinaryOp::Eq => self.binary_eq(left, right),
                    BinaryOp::Lt => self.binary_lt(left, right),
                    BinaryOp::Gt => self.binary_gt(left, right),
                    BinaryOp::And => self.binary_and(left, right),
                    BinaryOp::Or => self.binary_or(left, right),
                    BinaryOp::Ge => self.binary_ge(right, left),
                    BinaryOp::Le => self.binary_le(right, left),
                };
                self.context.push(result);
            }
            IRInstruction::Unary(op) => {
                let operand = self.context.pop();
                let result = match op {
                    UnaryOp::Neg => self.unary_neg(operand),
                    UnaryOp::Not => self.unary_not(operand),
                };
                self.context.push(result);
            }
            IRInstruction::Call(name, argc) => {
                let stack_base = self.context.stack.len() - argc as usize;
                let args: Vec<Value> = self.context.stack.drain(stack_base..).collect();
                let result = self.execute_function(&name, args);
                self.context.push(result);
            }
            IRInstruction::Return(has_value) => {
                let return_value = if has_value {
                    Some(self.context.pop())
                } else {
                    None
                };

                if let Some(frame) = self.context.frames.pop() {
                    self.context.stack.truncate(frame.stack_base);
                    if let Some(value) = return_value {
                        self.context.push(value);
                    }
                }
            }
            IRInstruction::Label(_) => {} // Labels are no-ops in VM
            IRInstruction::Jump(label) => {
                if let Some(frame) = self.context.frames.last_mut() {
                    if let Some(pos) = Self::find_label(&frame.function, &label) {
                        frame.ip = pos;
                    }
                }
            }
            IRInstruction::JumpIf(label) => {
                let condition = matches!(self.context.pop(), Value::Boolean(true));
                if condition {
                    if let Some(frame) = self.context.frames.last_mut() {
                        if let Some(pos) = Self::find_label(&frame.function, &label) {
                            frame.ip = pos;
                        }
                    }
                }
            }
        }
    }

    pub fn get_debug_trace(&self) -> Option<&DebugTrace> {
        self.debug_trace.as_ref()
    }

    // Helper methods for binary operations
    fn binary_add(&self, left: Value, right: Value) -> Value {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Value::Number(a + b),
            (Value::String(a), Value::String(b)) => Value::String(a + &b),
            (Value::String(a), b) => Value::String(format!("{}{}", a, Self::to_string(&b))),
            (a, Value::String(b)) => Value::String(format!("{}{}", Self::to_string(&a), b)),
            _ => Value::Undefined,
        }
    }

    fn binary_sub(&self, left: Value, right: Value) -> Value {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Value::Number(a - b),
            _ => Value::Undefined,
        }
    }

    fn binary_mul(&self, left: Value, right: Value) -> Value {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Value::Number(a * b),
            _ => Value::Undefined,
        }
    }

    fn binary_div(&self, left: Value, right: Value) -> Value {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => {
                if b == 0.0 {
                    Value::Number(f64::NAN)
                } else {
                    Value::Number(a / b)
                }
            }
            _ => Value::Undefined,
        }
    }

    fn binary_eq(&self, left: Value, right: Value) -> Value {
        Value::Boolean(match (left, right) {
            (Value::Number(a), Value::Number(b)) => (a - b).abs() < f64::EPSILON,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Null, Value::Null) => true,
            (Value::Undefined, Value::Undefined) => true,
            _ => false,
        })
    }

    fn binary_lt(&self, left: Value, right: Value) -> Value {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Value::Boolean(a < b),
            (Value::String(a), Value::String(b)) => Value::Boolean(a < b),
            _ => Value::Undefined,
        }
    }

    fn binary_gt(&self, left: Value, right: Value) -> Value {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Value::Boolean(a > b),
            (Value::String(a), Value::String(b)) => Value::Boolean(a > b),
            _ => Value::Undefined,
        }
    }

    fn binary_ge(&self, right: Value, left: Value) -> Value {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Value::Boolean(a >= b),
            (Value::String(a), Value::String(b)) => Value::Boolean(a >= b),
            _ => Value::Undefined,
        }
    }

    fn binary_le(&self, right: Value, left: Value) -> Value {
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => Value::Boolean(a <= b),
            (Value::String(a), Value::String(b)) => Value::Boolean(a <= b),
            _ => Value::Undefined,
        }
    }

    fn binary_and(&self, left: Value, right: Value) -> Value {
        // First evaluate left operand
        let left_bool = Self::to_boolean(&left);
        if !left_bool {
            // Short-circuit: if left is false, return false
            Value::Boolean(false)
        } else {
            // Left is true, evaluate and return right operand as boolean
            Value::Boolean(Self::to_boolean(&right))
        }
    }

    fn binary_or(&self, left: Value, right: Value) -> Value {
        // First evaluate left operand
        let left_bool = Self::to_boolean(&left);
        if left_bool {
            // Short-circuit: if left is true, return true
            Value::Boolean(true)
        } else {
            // Left is false, evaluate and return right operand as boolean
            Value::Boolean(Self::to_boolean(&right))
        }
    }

    fn unary_neg(&self, operand: Value) -> Value {
        match operand {
            Value::Number(n) => Value::Number(-n),
            _ => Value::Undefined,
        }
    }

    fn unary_not(&self, operand: Value) -> Value {
        Value::Boolean(!Self::to_boolean(&operand))
    }

    // Helper methods for type conversion (JavaScript-like behavior)
    fn to_boolean(value: &Value) -> bool {
        match value {
            Value::Boolean(b) => *b,
            Value::Number(n) => *n != 0.0 && !n.is_nan(),
            Value::String(s) => !s.is_empty(),
            Value::Null => false,
            Value::Undefined => false,
            Value::Object(_) => true,
        }
    }

    fn to_number(value: &Value) -> f64 {
        match value {
            Value::Number(n) => *n,
            Value::Boolean(true) => 1.0,
            Value::Boolean(false) => 0.0,
            Value::String(s) => s.parse().unwrap_or(f64::NAN),
            Value::Null => 0.0,
            Value::Undefined => f64::NAN,
            Value::Object(_) => f64::NAN,
        }
    }

    fn to_string(value: &Value) -> String {
        match value {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Boolean(b) => b.to_string(),
            Value::Null => "null".to_string(),
            Value::Undefined => "undefined".to_string(),
            Value::Object(_) => "[object Object]".to_string(),
        }
    }

    fn find_label(function: &IRFunction, label: &str) -> Option<usize> {
        function
            .instructions
            .iter()
            .position(|inst| matches!(inst, IRInstruction::Label(l) if l == label))
    }
}

// Native function implementations
fn native_print(args: Vec<Value>) -> Value {
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            print!(" ");
        }
        match arg {
            Value::Number(n) => print!("{}", n),
            Value::String(s) => print!("{}", s),
            Value::Boolean(b) => print!("{}", b),
            Value::Null => print!("null"),
            Value::Undefined => print!("undefined"),
            Value::Object(_) => print!("[object Object]"),
        }
    }
    println!();
    Value::Undefined
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse;

    fn setup_vm(source: &str) -> VM {
        let tokens = tokenize(source);
        let ast = parse(tokens);
        let ir_module = crate::ir::lower_ast(ast);
        VM::new(ir_module)
    }

    #[test]
    fn test_arithmetic_operations() {
        let mut vm = setup_vm("function test() { return 5 + 3; }");
        let result = vm.execute_function("test", vec![]);
        match result {
            Value::Number(n) => assert_eq!(n, 8.0),
            _ => panic!("Expected number result"),
        }
    }

    #[test]
    fn test_comparison_operations() {
        let mut vm = setup_vm("function test(x, y) { return x > y; }");
        let result = vm.execute_function("test", vec![Value::Number(5.0), Value::Number(3.0)]);
        assert_eq!(result, Value::Boolean(true));
    }

    #[test]
    fn test_function_calls() {
        let mut vm = setup_vm(
            "function add(x, y) { return x + y; }
             function test() { return add(5, 3); }",
        );
        let result = vm.execute_function("test", vec![]);
        match result {
            Value::Number(n) => assert_eq!(n, 8.0),
            _ => panic!("Expected number result"),
        }
    }

    #[test]
    fn test_conditional_execution() {
        let mut vm = setup_vm(
            "function test(x) { 
                if (x > 0) { 
                    return true; 
                } else { 
                    return false; 
                }
             }",
        );

        let result_positive = vm.execute_function("test", vec![Value::Number(1.0)]);
        assert_eq!(result_positive, Value::Boolean(true));

        let result_negative = vm.execute_function("test", vec![Value::Number(-1.0)]);
        assert_eq!(result_negative, Value::Boolean(false));
    }

    #[test]
    fn test_variable_scoping() {
        let mut vm = setup_vm(
            "let global = 10;
             function test() { 
                let local = 20;
                let result = local + global;
                return result;
             }",
        );

        // First set the global variable
        vm.context
            .globals
            .insert("global".to_string(), Value::Number(10.0));

        let result = vm.execute_function("test", vec![]);
        match result {
            Value::Number(n) => assert_eq!(n, 30.0),
            _ => panic!("Expected number result"),
        }
    }
}
