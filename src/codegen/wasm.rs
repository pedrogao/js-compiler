use super::CodeGenerator;
use crate::ir::{BinaryOp, Constant, IRFunction, IRInstruction, IRModule, UnaryOp};
use std::collections::HashMap;

pub struct WasmGenerator {
    output: String,
    locals: HashMap<String, u32>,
    local_count: u32,
    string_data: Vec<String>,
    float_data: Vec<f64>,
}

impl WasmGenerator {
    pub fn new() -> Self {
        Self {
            output: String::new(),
            locals: HashMap::new(),
            local_count: 0,
            string_data: Vec::new(),
            float_data: Vec::new(),
        }
    }

    fn reset_state(&mut self) {
        self.locals.clear();
        self.local_count = 0;
    }

    fn allocate_local(&mut self, name: &str) -> u32 {
        let idx = self.local_count;
        self.locals.insert(name.to_string(), idx);
        self.local_count += 1;
        idx
    }

    fn generate_function(&mut self, function: &IRFunction) {
        self.reset_state();

        // Function header
        self.output.push_str(&format!("(func ${} ", function.name));

        // Parameters
        for _ in &function.params {
            self.output.push_str("(param i64) ");
        }
        self.output.push_str("(result i64)\n");

        // Local variables
        if function.max_locals > 0 {
            self.output
                .push_str(&format!("(local ${} i64)\n", self.local_count));
        }

        // Generate instructions
        for instruction in &function.instructions {
            self.generate_instruction(instruction);
        }

        self.output.push_str(")\n");
    }

    fn generate_instruction(&mut self, instruction: &IRInstruction) {
        match instruction {
            IRInstruction::PushConst(constant) => self.generate_const(constant),
            IRInstruction::Load(name) => {
                let local_idx = self.locals.get(name).cloned().unwrap_or_else(|| {
                    let idx = self.allocate_local(name);
                    idx
                });
                self.output.push_str(&format!("local.get {}\n", local_idx));
            }
            IRInstruction::Store(name) => {
                let local_idx = self.locals.get(name).cloned().unwrap_or_else(|| {
                    let idx = self.allocate_local(name);
                    idx
                });
                self.output.push_str(&format!("local.set {}\n", local_idx));
            }
            IRInstruction::Binary(op) => self.generate_binary_op(op),
            IRInstruction::Unary(op) => self.generate_unary_op(op),
            IRInstruction::Call(name, argc) => {
                self.output
                    .push_str(&format!("call ${} ;; args: {}\n", name, argc));
            }
            IRInstruction::Return(has_value) => {
                if (!has_value) {
                    self.output.push_str("i64.const 0\n");
                }
                self.output.push_str("return\n");
            }
            IRInstruction::Jump(label) => {
                self.output.push_str(&format!("br {}\n", label));
            }
            IRInstruction::JumpIf(label) => {
                self.output.push_str(&format!("br_if {}\n", label));
            }
            IRInstruction::Label(label) => {
                self.output.push_str(&format!("(block ${}\n", label));
            }
            IRInstruction::Pop => {
                self.output.push_str("drop\n");
            }
            IRInstruction::Dup => {
                self.output.push_str("local.tee $tmp\n");
                self.output.push_str("local.get $tmp\n");
            }
        }
    }

    fn generate_const(&mut self, constant: &Constant) {
        match constant {
            Constant::Number(n) => {
                self.output.push_str(&format!("f64.const {}\n", n));
                self.output.push_str("i64.reinterpret_f64\n");
            }
            Constant::String(s) => {
                let index = self.string_data.len();
                self.string_data.push(s.clone());
                self.output.push_str(&format!("i64.const {}\n", index));
            }
            Constant::Boolean(b) => {
                self.output
                    .push_str(&format!("i64.const {}\n", if *b { 1 } else { 0 }));
            }
            Constant::Null => {
                self.output.push_str("i64.const 0\n");
            }
        }
    }

    fn generate_binary_op(&mut self, op: &BinaryOp) {
        match op {
            BinaryOp::Add => self.output.push_str("i64.add\n"),
            BinaryOp::Sub => self.output.push_str("i64.sub\n"),
            BinaryOp::Mul => self.output.push_str("i64.mul\n"),
            BinaryOp::Div => {
                self.output.push_str("f64.reinterpret_i64\n");
                self.output.push_str("f64.div\n");
                self.output.push_str("i64.reinterpret_f64\n");
            }
            BinaryOp::Eq => self.output.push_str("i64.eq\n"),
            BinaryOp::Lt => self.output.push_str("i64.lt_s\n"),
            BinaryOp::Gt => self.output.push_str("i64.gt_s\n"),
            BinaryOp::Le => self.output.push_str("i64.le_s\n"),
            BinaryOp::Ge => self.output.push_str("i64.ge_s\n"),
            BinaryOp::And => self.output.push_str("i64.and\n"),
            BinaryOp::Or => self.output.push_str("i64.or\n"),
        }
    }

    fn generate_unary_op(&mut self, op: &UnaryOp) {
        match op {
            UnaryOp::Neg => {
                self.output.push_str("i64.const -1\n");
                self.output.push_str("i64.mul\n");
            }
            UnaryOp::Not => {
                self.output.push_str("i64.eqz\n");
                self.output.push_str("i64.extend_i32_u\n");
            }
        }
    }
}

impl CodeGenerator for WasmGenerator {
    fn generate(&mut self, module: IRModule) -> String {
        // Module header
        self.output.push_str("(module\n");

        // Memory section for string data
        self.output.push_str("(memory 1)\n");

        // Import JavaScript console.log
        self.output
            .push_str("(import \"console\" \"log\" (func $log (param i64)))\n");

        // Generate data sections for strings
        for (i, string) in self.string_data.iter().enumerate() {
            self.output.push_str(&format!(
                "(data (i32.const {}) \"{}\")\n",
                i * 8,
                string.escape_default()
            ));
        }

        // Check for main function
        let has_main = module.functions.iter().any(|f| f.name == "main");

        // Generate functions
        for function in module.functions {
            self.generate_function(&function);
        }

        // Export main function if it exists
        if has_main {
            self.output.push_str("(export \"main\" (func $main))\n");
        }

        // Close module
        self.output.push_str(")\n");

        self.output.clone()
    }
}
