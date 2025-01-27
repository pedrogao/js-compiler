use super::CodeGenerator;
use crate::ir::{BinaryOp, Constant, IRFunction, IRInstruction, IRModule, UnaryOp};
use std::collections::HashMap;
use std::fmt::Write;

pub struct ARM64Generator {
    output: String,
    string_literals: Vec<String>,
    float_literals: Vec<f64>,
    local_offsets: HashMap<String, i32>,
    current_stack_size: i32,
    label_counter: usize,
}

impl ARM64Generator {
    pub fn new() -> Self {
        Self {
            output: String::new(),
            string_literals: Vec::new(),
            float_literals: Vec::new(),
            local_offsets: HashMap::new(),
            current_stack_size: 0,
            label_counter: 0,
        }
    }

    fn reset_state(&mut self) {
        self.local_offsets.clear();
        self.current_stack_size = 0;
    }

    fn next_label(&mut self) -> String {
        self.label_counter += 1;
        format!(".L{}", self.label_counter)
    }

    fn generate_function(&mut self, function: &IRFunction) {
        self.reset_state();

        // Function header
        writeln!(self.output, "\t.global _{}", function.name).unwrap();
        writeln!(self.output, "\t.p2align 2").unwrap();
        writeln!(self.output, "_{}:", function.name).unwrap();

        // Function prologue
        writeln!(self.output, "\tstp fp, lr, [sp, #-16]!").unwrap();
        writeln!(self.output, "\tmov fp, sp").unwrap();

        // Allocate stack frame
        let frame_size = ((function.max_locals * 8 + 15) / 16) * 16;
        if frame_size > 0 {
            writeln!(self.output, "\tsub sp, sp, #{}", frame_size).unwrap();
        }

        // Save callee-saved registers
        writeln!(self.output, "\tstp x19, x20, [sp, #-16]!").unwrap();
        writeln!(self.output, "\tstp x21, x22, [sp, #-16]!").unwrap();
        writeln!(self.output, "\tstp x23, x24, [sp, #-16]!").unwrap();
        writeln!(self.output, "\tstp x25, x26, [sp, #-16]!").unwrap();
        writeln!(self.output, "\tstp x27, x28, [sp, #-16]!").unwrap();

        // Store parameters in their slots
        for (i, param) in function.params.iter().enumerate() {
            let param_reg = match i {
                0 => "x0",
                1 => "x1",
                2 => "x2",
                3 => "x3",
                4 => "x4",
                5 => "x5",
                6 => "x6",
                7 => "x7",
                _ => panic!("Too many parameters"),
            };
            let offset = self.allocate_local(param);
            writeln!(self.output, "\tstr {}, [fp, #{}]", param_reg, offset).unwrap();
        }

        // Generate code for instructions
        for instruction in &function.instructions {
            self.generate_instruction(instruction);
        }
    }

    fn generate_epilogue(&mut self) {
        // Restore callee-saved registers
        writeln!(self.output, "\tldp x27, x28, [sp], #16").unwrap();
        writeln!(self.output, "\tldp x25, x26, [sp], #16").unwrap();
        writeln!(self.output, "\tldp x23, x24, [sp], #16").unwrap();
        writeln!(self.output, "\tldp x21, x22, [sp], #16").unwrap();
        writeln!(self.output, "\tldp x19, x20, [sp], #16").unwrap();
        writeln!(self.output, "\tmov sp, fp").unwrap();
        writeln!(self.output, "\tldp fp, lr, [sp], #16").unwrap();
        writeln!(self.output, "\tret").unwrap();
    }

    fn allocate_local(&mut self, name: &str) -> i32 {
        let offset = self.current_stack_size - 8;
        self.local_offsets.insert(name.to_string(), offset);
        self.current_stack_size = offset;
        offset
    }

    fn generate_instruction(&mut self, instruction: &IRInstruction) {
        match instruction {
            IRInstruction::PushConst(constant) => self.generate_push_const(constant),
            IRInstruction::Load(name) => self.generate_load(name),
            IRInstruction::Store(name) => self.generate_store(name),
            IRInstruction::Binary(op) => self.generate_binary_op(op),
            IRInstruction::Unary(op) => self.generate_unary_op(op),
            IRInstruction::Call(name, argc) => self.generate_call(name, *argc),
            IRInstruction::Return(has_value) => self.generate_return(*has_value),
            IRInstruction::Jump(label) => self.generate_jump(label),
            IRInstruction::JumpIf(label) => self.generate_jump_if(label),
            IRInstruction::Label(label) => writeln!(self.output, "{}:", label).unwrap(),
            IRInstruction::Pop => writeln!(self.output, "\tadd sp, sp, #8").unwrap(),
            IRInstruction::Dup => {
                writeln!(self.output, "\tldr x0, [sp]").unwrap();
                writeln!(self.output, "\tstr x0, [sp, #-8]!").unwrap();
            }
        }
    }

    fn generate_push_const(&mut self, constant: &Constant) {
        match constant {
            Constant::Number(n) => {
                let idx = self.float_literals.len();
                self.float_literals.push(*n);
                writeln!(self.output, "\tadrp x0, .LCD{}@PAGE", idx).unwrap();
                writeln!(self.output, "\tldr d0, [x0, .LCD{}@PAGEOFF]", idx).unwrap();
                writeln!(self.output, "\tstr d0, [sp, #-8]!").unwrap();
            }
            Constant::String(s) => {
                let idx = self.string_literals.len();
                self.string_literals.push(s.clone());
                writeln!(self.output, "\tadrp x0, .LC{}@PAGE", idx).unwrap();
                writeln!(self.output, "\tadd x0, x0, .LC{}@PAGEOFF", idx).unwrap();
                writeln!(self.output, "\tstr x0, [sp, #-8]!").unwrap();
            }
            Constant::Boolean(b) => {
                writeln!(self.output, "\tmov x0, #{}", if *b { 1 } else { 0 }).unwrap();
                writeln!(self.output, "\tstr x0, [sp, #-8]!").unwrap();
            }
            Constant::Null => {
                writeln!(self.output, "\tstr xzr, [sp, #-8]!").unwrap();
            }
        }
    }

    fn generate_load(&mut self, name: &str) {
        if let Some(&offset) = self.local_offsets.get(name) {
            writeln!(self.output, "\tldr x0, [fp, #{}]", offset).unwrap();
            writeln!(self.output, "\tstr x0, [sp, #-8]!").unwrap();
        } else {
            panic!("Undefined variable: {}", name);
        }
    }

    fn generate_store(&mut self, name: &str) {
        let offset = self.local_offsets.get(name).cloned().unwrap_or_else(|| {
            let offset = self.allocate_local(name);
            offset
        });
        writeln!(self.output, "\tldr x0, [sp], #8").unwrap();
        writeln!(self.output, "\tstr x0, [fp, #{}]", offset).unwrap();
    }

    fn generate_binary_op(&mut self, op: &BinaryOp) {
        writeln!(self.output, "\tldr x1, [sp], #8").unwrap(); // right operand
        writeln!(self.output, "\tldr x0, [sp], #8").unwrap(); // left operand

        match op {
            BinaryOp::Add => writeln!(self.output, "\tadd x0, x0, x1").unwrap(),
            BinaryOp::Sub => writeln!(self.output, "\tsub x0, x0, x1").unwrap(),
            BinaryOp::Mul => writeln!(self.output, "\tmul x0, x0, x1").unwrap(),
            BinaryOp::Div => {
                writeln!(self.output, "\tsdiv x0, x0, x1").unwrap();
            }
            BinaryOp::Eq => {
                writeln!(self.output, "\tcmp x0, x1").unwrap();
                writeln!(self.output, "\tcset x0, eq").unwrap();
            }
            BinaryOp::Lt => {
                writeln!(self.output, "\tcmp x0, x1").unwrap();
                writeln!(self.output, "\tcset x0, lt").unwrap();
            }
            BinaryOp::Gt => {
                writeln!(self.output, "\tcmp x0, x1").unwrap();
                writeln!(self.output, "\tcset x0, gt").unwrap();
            }
            BinaryOp::Le => {
                writeln!(self.output, "\tcmp x0, x1").unwrap();
                writeln!(self.output, "\tcset x0, le").unwrap();
            }
            BinaryOp::Ge => {
                writeln!(self.output, "\tcmp x0, x1").unwrap();
                writeln!(self.output, "\tcset x0, ge").unwrap();
            }
            BinaryOp::And => writeln!(self.output, "\tand x0, x0, x1").unwrap(),
            BinaryOp::Or => writeln!(self.output, "\torr x0, x0, x1").unwrap(),
        }
        writeln!(self.output, "\tstr x0, [sp, #-8]!").unwrap();
    }

    fn generate_unary_op(&mut self, op: &UnaryOp) {
        writeln!(self.output, "\tldr x0, [sp], #8").unwrap();
        match op {
            UnaryOp::Neg => {
                writeln!(self.output, "\tneg x0, x0").unwrap();
            }
            UnaryOp::Not => {
                writeln!(self.output, "\tcmp x0, #0").unwrap();
                writeln!(self.output, "\tcset x0, eq").unwrap();
            }
        }
        writeln!(self.output, "\tstr x0, [sp, #-8]!").unwrap();
    }

    fn generate_call(&mut self, name: &str, argc: u16) {
        // Set up arguments
        for i in (0..argc).rev() {
            let reg = match i {
                0 => "x0",
                1 => "x1",
                2 => "x2",
                3 => "x3",
                4 => "x4",
                5 => "x5",
                6 => "x6",
                7 => "x7",
                _ => panic!("Too many arguments in call to {}", name),
            };
            writeln!(self.output, "\tldr {}, [sp], #8", reg).unwrap();
        }

        writeln!(self.output, "\tbl _{}", name).unwrap();
        writeln!(self.output, "\tstr x0, [sp, #-8]!").unwrap();
    }

    fn generate_return(&mut self, has_value: bool) {
        if has_value {
            writeln!(self.output, "\tldr x0, [sp], #8").unwrap();
        }
        self.generate_epilogue();
    }

    fn generate_jump(&mut self, label: &str) {
        writeln!(self.output, "\tb {}", label).unwrap();
    }

    fn generate_jump_if(&mut self, label: &str) {
        writeln!(self.output, "\tldr x0, [sp], #8").unwrap();
        writeln!(self.output, "\tcmp x0, #0").unwrap();
        writeln!(self.output, "\tb.ne {}", label).unwrap();
    }
}

impl CodeGenerator for ARM64Generator {
    fn generate(&mut self, module: IRModule) -> String {
        // Data section for constants
        writeln!(self.output, "\t.section __DATA,__data").unwrap();

        // Add string literals
        for (i, s) in self.string_literals.iter().enumerate() {
            writeln!(self.output, ".LC{}:", i).unwrap();
            writeln!(self.output, "\t.asciz \"{}\"", s).unwrap();
        }

        // Add float literals
        for (i, f) in self.float_literals.iter().enumerate() {
            writeln!(self.output, ".LCD{}:", i).unwrap();
            writeln!(self.output, "\t.double {}", f).unwrap();
        }

        // Text section for code
        writeln!(self.output, "\t.section __TEXT,__text").unwrap();

        // Generate code for each function
        for function in module.functions {
            self.generate_function(&function);
        }

        self.output.clone()
    }
}
