use crate::ir::{BinaryOp, Constant, IRFunction, IRInstruction, IRModule, UnaryOp};
use std::collections::HashMap;

struct CodeGenerator {
    output: String,
    string_literals: Vec<String>,
    float_literals: Vec<f64>,
    local_offsets: HashMap<String, i32>,
    current_stack_size: i32,
}

impl CodeGenerator {
    fn new() -> Self {
        Self {
            output: String::new(),
            string_literals: Vec::new(),
            float_literals: Vec::new(),
            local_offsets: HashMap::new(),
            current_stack_size: 0,
        }
    }

    fn reset_frame(&mut self) {
        self.local_offsets.clear();
        self.current_stack_size = 0;
    }

    fn allocate_local(&mut self, name: &str) -> i32 {
        self.current_stack_size += 8;
        let offset = -self.current_stack_size;
        self.local_offsets.insert(name.to_string(), offset);
        offset
    }

    fn get_local_offset(&self, name: &str) -> i32 {
        *self
            .local_offsets
            .get(name)
            .unwrap_or_else(|| panic!("Local variable {} not found", name))
    }

    fn generate(&mut self, module: IRModule) -> String {
        // Data section
        self.output.push_str("    .data\n");

        // String literals
        for (i, s) in self.string_literals.iter().enumerate() {
            self.output
                .push_str(&format!("str_{}: .string \"{}\"\n", i, s));
        }

        // Float literals
        for (i, f) in self.float_literals.iter().enumerate() {
            self.output
                .push_str(&format!("float_{}: .double {}\n", i, f));
        }

        // Text section
        self.output.push_str("    .text\n");

        for function in module.functions {
            self.generate_function(&function);
        }

        std::mem::take(&mut self.output)
    }

    fn generate_function(&mut self, function: &IRFunction) {
        self.reset_frame();

        // Function header
        self.output
            .push_str(&format!("    .global {}\n", function.name));
        self.output.push_str(&format!("{}:\n", function.name));

        // Function prologue
        self.output.push_str("    push rbp\n");
        self.output.push_str("    mov rbp, rsp\n");

        // Pre-allocate space for all locals
        let frame_size = (function.max_locals as i32) * 8;
        if frame_size > 0 {
            self.output
                .push_str(&format!("    sub rsp, {}\n", frame_size));
        }

        // Save callee-saved registers
        self.output.push_str("    push rbx\n");
        self.output.push_str("    push r12\n");
        self.output.push_str("    push r13\n");
        self.output.push_str("    push r14\n");
        self.output.push_str("    push r15\n");

        // Initialize parameters as locals
        for (i, param) in function.params.iter().enumerate() {
            let offset = self.allocate_local(param);
            // Move parameter from register/stack to local variable
            match i {
                0 => self
                    .output
                    .push_str(&format!("    mov [rbp{}], rdi\n", offset)),
                1 => self
                    .output
                    .push_str(&format!("    mov [rbp{}], rsi\n", offset)),
                2 => self
                    .output
                    .push_str(&format!("    mov [rbp{}], rdx\n", offset)),
                3 => self
                    .output
                    .push_str(&format!("    mov [rbp{}], rcx\n", offset)),
                4 => self
                    .output
                    .push_str(&format!("    mov [rbp{}], r8\n", offset)),
                5 => self
                    .output
                    .push_str(&format!("    mov [rbp{}], r9\n", offset)),
                n => self.output.push_str(&format!(
                    "    mov rax, [rbp+{}]\n    mov [rbp{}], rax\n",
                    16 + (n - 6) * 8,
                    offset
                )),
            }
        }

        // Generate instructions
        for instruction in &function.instructions {
            self.generate_instruction(instruction);
        }
    }

    fn generate_instruction(&mut self, instruction: &IRInstruction) {
        match instruction {
            IRInstruction::Pop => {
                self.output.push_str("    add rsp, 8\n");
            }
            IRInstruction::Dup => {
                self.output.push_str("    mov rax, [rsp]\n");
                self.output.push_str("    push rax\n");
            }
            IRInstruction::PushConst(constant) => match constant {
                Constant::Number(n) => {
                    let idx = self.float_literals.len();
                    self.float_literals.push(*n);
                    self.output
                        .push_str(&format!("    movsd xmm0, [rip + float_{}]\n", idx));
                    self.output.push_str("    sub rsp, 8\n");
                    self.output.push_str("    movsd [rsp], xmm0\n");
                }
                Constant::String(s) => {
                    let idx = self.string_literals.len();
                    self.string_literals.push(s.clone());
                    self.output
                        .push_str(&format!("    lea rax, [rip + str_{}]\n", idx));
                    self.output.push_str("    push rax\n");
                }
                Constant::Boolean(b) => {
                    self.output
                        .push_str(&format!("    push {}\n", if *b { 1 } else { 0 }));
                }
                Constant::Null => {
                    self.output.push_str("    push 0\n");
                }
            },
            IRInstruction::Load(name) => {
                let offset = self.get_local_offset(name);
                self.output
                    .push_str(&format!("    mov rax, [rbp{}]\n", offset));
                self.output.push_str("    push rax\n");
            }
            IRInstruction::Store(name) => {
                let offset = match self.local_offsets.get(name) {
                    Some(off) => *off,
                    None => self.allocate_local(name),
                };
                self.output.push_str("    pop rax\n");
                self.output
                    .push_str(&format!("    mov [rbp{}], rax\n", offset));
            }
            IRInstruction::Binary(op) => self.generate_binary_op(op),
            IRInstruction::Unary(op) => self.generate_unary_op(op),
            IRInstruction::Call(name, argc) => {
                // Align stack for System V ABI
                if *argc % 2 == 1 {
                    self.output.push_str("    sub rsp, 8\n");
                }

                self.output.push_str(&format!("    call {}\n", name));

                // Clean up arguments
                let cleanup_size = if *argc % 2 == 0 {
                    argc * 8
                } else {
                    (argc + 1) * 8
                };
                if cleanup_size > 0 {
                    self.output
                        .push_str(&format!("    add rsp, {}\n", cleanup_size));
                }

                // Push return value
                self.output.push_str("    push rax\n");
            }
            IRInstruction::Return(has_value) => {
                if *has_value {
                    self.output.push_str("    pop rax\n");
                }

                self.generate_epilogue();
            }
            IRInstruction::Label(label) => {
                self.output.push_str(&format!(".{}:\n", label));
            }
            IRInstruction::Jump(label) => {
                self.output.push_str(&format!("    jmp .{}\n", label));
            }
            IRInstruction::JumpIf(label) => {
                self.output.push_str("    pop rax\n");
                self.output.push_str("    test rax, rax\n");
                self.output.push_str(&format!("    jnz .{}\n", label));
            }
        }
    }

    fn generate_binary_op(&mut self, op: &BinaryOp) {
        // Pop operands into XMM registers for floating-point operations
        self.output.push_str("    movsd xmm1, [rsp]\n");
        self.output.push_str("    add rsp, 8\n");
        self.output.push_str("    movsd xmm0, [rsp]\n");
        self.output.push_str("    add rsp, 8\n");

        match op {
            BinaryOp::Add => {
                self.output.push_str("    addsd xmm0, xmm1\n");
            }
            BinaryOp::Sub => {
                self.output.push_str("    subsd xmm0, xmm1\n");
            }
            BinaryOp::Mul => {
                self.output.push_str("    mulsd xmm0, xmm1\n");
            }
            BinaryOp::Div => {
                self.output.push_str("    divsd xmm0, xmm1\n");
            }
            BinaryOp::Eq => {
                self.output.push_str("    ucomisd xmm0, xmm1\n");
                self.output.push_str("    sete al\n");
                self.output.push_str("    movzx rax, al\n");
                self.output.push_str("    push rax\n");
                return;
            }
            BinaryOp::Lt => {
                self.output.push_str("    ucomisd xmm0, xmm1\n");
                self.output.push_str("    setb al\n");
                self.output.push_str("    movzx rax, al\n");
                self.output.push_str("    push rax\n");
                return;
            }
            BinaryOp::Gt => {
                self.output.push_str("    ucomisd xmm0, xmm1\n");
                self.output.push_str("    seta al\n");
                self.output.push_str("    movzx rax, al\n");
                self.output.push_str("    push rax\n");
                return;
            }
            BinaryOp::Ge => {
                self.output.push_str("    ucomisd xmm0, xmm1\n");
                self.output.push_str("    setae al\n");
                self.output.push_str("    movzx rax, al\n");
                self.output.push_str("    push rax\n");
                return;
            }
            BinaryOp::Le => {
                self.output.push_str("    ucomisd xmm0, xmm1\n");
                self.output.push_str("    setbe al\n");
                self.output.push_str("    movzx rax, al\n");
                self.output.push_str("    push rax\n");
                return;
            }
            BinaryOp::And | BinaryOp::Or => {
                // These are handled by the IR using JumpIf instructions
                panic!("Logical operators should be handled using jumps");
            }
        }

        // Push result back onto stack
        self.output.push_str("    sub rsp, 8\n");
        self.output.push_str("    movsd [rsp], xmm0\n");
    }

    fn generate_unary_op(&mut self, op: &UnaryOp) {
        match op {
            UnaryOp::Neg => {
                self.output.push_str("    movsd xmm0, [rsp]\n");
                self.output.push_str("    xorpd xmm1, xmm1\n");
                self.output.push_str("    subsd xmm1, xmm0\n");
                self.output.push_str("    movsd [rsp], xmm1\n");
            }
            UnaryOp::Not => {
                self.output.push_str("    pop rax\n");
                self.output.push_str("    xor rax, 1\n");
                self.output.push_str("    push rax\n");
            }
        }
    }

    fn generate_epilogue(&mut self) {
        // Restore callee-saved registers
        self.output.push_str("    pop r15\n");
        self.output.push_str("    pop r14\n");
        self.output.push_str("    pop r13\n");
        self.output.push_str("    pop r12\n");
        self.output.push_str("    pop rbx\n");

        // Restore stack frame
        self.output.push_str("    mov rsp, rbp\n");
        self.output.push_str("    pop rbp\n");
        self.output.push_str("    ret\n");
    }
}

pub fn generate_x64(module: IRModule) -> String {
    let mut generator = CodeGenerator::new();
    generator.generate(module)
}
