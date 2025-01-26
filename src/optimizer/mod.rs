use crate::ir::{BinaryOp, Constant, IRFunction, IRInstruction, IRModule, UnaryOp};
use std::collections::{HashMap, HashSet};

struct Optimizer {
    module: IRModule,
}

impl Optimizer {
    fn new(module: IRModule) -> Self {
        Self { module }
    }

    fn constant_folding(&mut self) -> &mut Self {
        for function in &mut self.module.functions {
            let mut i = 0;
            while i < function.instructions.len() {
                let instructions = function.instructions[i..].to_vec();
                let folded = Self::try_fold_constants(&instructions);
                if let Some(folded) = folded {
                    // Replace the instruction(s) with the folded constant
                    function
                        .instructions
                        .splice(i..i + folded.len, folded.result);
                    i += 1;
                } else {
                    i += 1;
                }
            }
        }
        self
    }

    fn try_fold_constants(instructions: &[IRInstruction]) -> Option<FoldResult> {
        match &instructions[0] {
            IRInstruction::Binary(op) => {
                // Look for pattern: PushConst, PushConst, Binary
                if instructions.len() < 3 {
                    return None;
                }

                if let (
                    IRInstruction::PushConst(left),
                    IRInstruction::PushConst(right),
                    IRInstruction::Binary(bin_op),
                ) = (&instructions[0], &instructions[1], &instructions[2])
                {
                    let result = match (left, right, bin_op) {
                        (Constant::Number(a), Constant::Number(b), BinaryOp::Add) => {
                            Some(Constant::Number(a + b))
                        }
                        (Constant::Number(a), Constant::Number(b), BinaryOp::Sub) => {
                            Some(Constant::Number(a - b))
                        }
                        (Constant::Number(a), Constant::Number(b), BinaryOp::Mul) => {
                            Some(Constant::Number(a * b))
                        }
                        (Constant::Number(a), Constant::Number(b), BinaryOp::Div) if *b != 0.0 => {
                            Some(Constant::Number(a / b))
                        }
                        (Constant::String(a), Constant::String(b), BinaryOp::Add) => {
                            Some(Constant::String(a.clone() + b))
                        }
                        _ => None,
                    };

                    result.map(|const_result| FoldResult {
                        result: vec![IRInstruction::PushConst(const_result)],
                        len: 3,
                    })
                } else {
                    None
                }
            }
            IRInstruction::Unary(op) => {
                // Look for pattern: PushConst, Unary
                if instructions.len() < 2 {
                    return None;
                }

                if let IRInstruction::PushConst(constant) = &instructions[1] {
                    let result = match (op, constant) {
                        (UnaryOp::Neg, Constant::Number(n)) => Some(Constant::Number(-n)),
                        (UnaryOp::Not, Constant::Boolean(b)) => Some(Constant::Boolean(!b)),
                        _ => None,
                    };

                    result.map(|const_result| FoldResult {
                        result: vec![IRInstruction::PushConst(const_result)],
                        len: 2,
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn dead_code_elimination(&mut self) -> &mut Self {
        for function in &mut self.module.functions {
            // Find all reachable instructions
            let reachable = Self::find_reachable_instructions(function);

            // Remove unreachable instructions
            function.instructions = function
                .instructions
                .iter()
                .enumerate()
                .filter(|(i, _)| reachable.contains(i))
                .map(|(_, instr)| instr.clone())
                .collect();
        }
        self
    }

    fn find_reachable_instructions(function: &IRFunction) -> HashSet<usize> {
        let mut reachable = HashSet::new();
        let mut work_list = vec![0]; // Start from first instruction
        let mut label_positions = HashMap::new();

        // First pass: collect all label positions
        for (i, instr) in function.instructions.iter().enumerate() {
            if let IRInstruction::Label(label) = instr {
                label_positions.insert(label.clone(), i);
            }
        }

        // Second pass: find all reachable instructions
        while let Some(pos) = work_list.pop() {
            if pos >= function.instructions.len() || !reachable.insert(pos) {
                continue;
            }

            match &function.instructions[pos] {
                IRInstruction::Jump(label) => {
                    if let Some(&target) = label_positions.get(label) {
                        work_list.push(target);
                    }
                }
                IRInstruction::JumpIf(label) => {
                    if let Some(&target) = label_positions.get(label) {
                        work_list.push(target);
                    }
                    work_list.push(pos + 1); // Fall-through path
                }
                IRInstruction::Return(_) => {
                    // No more instructions after return
                }
                _ => {
                    work_list.push(pos + 1); // Sequential execution
                }
            }
        }

        reachable
    }

    fn run_all_passes(&mut self) -> &mut Self {
        self.constant_folding().dead_code_elimination()
    }
}

struct FoldResult {
    result: Vec<IRInstruction>,
    len: usize,
}

pub fn optimize(module: IRModule) -> IRModule {
    let mut optimizer = Optimizer::new(module);
    optimizer.run_all_passes();
    optimizer.module
}
