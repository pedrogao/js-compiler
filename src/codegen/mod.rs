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
