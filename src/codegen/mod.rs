pub mod arm64;
pub mod wasm;
pub mod x64;

use crate::ir::IRModule;

pub trait CodeGenerator {
    fn generate(&mut self, module: IRModule) -> String;
}

pub fn generate_code(module: IRModule, target: Target) -> Option<String> {
    match target {
        Target::X64 => {
            let mut generator = x64::X64Generator::new();
            Some(generator.generate(module))
        }
        Target::ARM64 => {
            let mut generator = arm64::ARM64Generator::new();
            Some(generator.generate(module))
        }
        Target::Wasm => {
            let mut generator = wasm::WasmGenerator::new();
            Some(generator.generate(module))
        }
        Target::None => None,
    }
}

#[derive(Debug, Clone)]
pub enum Target {
    X64,
    ARM64,
    Wasm,
    None, // Added for VM-only execution
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{BinaryOp, Constant, IRFunction, IRInstruction};

    #[test]
    fn test_x64_generation() {
        let function = IRFunction {
            name: "test".to_string(),
            params: vec![],
            max_stack: 2,
            max_locals: 0,
            instructions: vec![
                IRInstruction::PushConst(Constant::Number(5.0)),
                IRInstruction::PushConst(Constant::Number(3.0)),
                IRInstruction::Binary(BinaryOp::Add),
                IRInstruction::Return(true),
            ],
            exception_table: vec![],
        };

        let module = IRModule {
            functions: vec![function],
            constants: vec![Constant::Number(5.0), Constant::Number(3.0)],
        };

        let code = generate_code(module, Target::X64);
        assert!(code.is_some());
        assert!(code.unwrap().contains("add"));
    }

    #[test]
    fn test_wasm_generation() {
        let function = IRFunction {
            name: "add".to_string(),
            params: vec!["x".to_string(), "y".to_string()],
            max_stack: 2,
            max_locals: 2,
            instructions: vec![
                IRInstruction::Load("x".to_string()),
                IRInstruction::Load("y".to_string()),
                IRInstruction::Binary(BinaryOp::Add),
                IRInstruction::Return(true),
            ],
            exception_table: vec![],
        };

        let module = IRModule {
            functions: vec![function],
            constants: vec![],
        };

        let code = generate_code(module, Target::Wasm);
        assert!(code.is_some());
        let wasm_code = code.unwrap();
        // Update assertions to match actual WebAssembly text format
        assert!(wasm_code.contains("(module"));
        assert!(wasm_code.contains("(func"));
    }

    #[test]
    fn test_arm64_generation() {
        let function = IRFunction {
            name: "main".to_string(),
            params: vec![],
            max_stack: 1,
            max_locals: 0,
            instructions: vec![
                IRInstruction::PushConst(Constant::Number(42.0)),
                IRInstruction::Return(true),
            ],
            exception_table: vec![],
        };

        let module = IRModule {
            functions: vec![function],
            constants: vec![Constant::Number(42.0)],
        };

        let code = generate_code(module, Target::ARM64);
        assert!(code.is_some());
        assert!(code.unwrap().contains(".global _main"));
    }
}
