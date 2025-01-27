use crate::ir::{IRFunction, IRInstruction};
use crate::vm::Value;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize, Clone)]
pub struct DebugFrame {
    pub instruction: String,
    pub stack: Vec<String>,
    pub locals: HashMap<String, String>,
    pub ip: usize,
    pub function_name: String,
}

#[derive(Serialize)]
pub struct DebugTrace {
    pub frames: Vec<DebugFrame>,
    pub breakpoints: Vec<usize>,
}

impl DebugTrace {
    pub fn new() -> Self {
        DebugTrace {
            frames: Vec::new(),
            breakpoints: Vec::new(),
        }
    }

    pub fn add_frame(
        &mut self,
        instruction: &IRInstruction,
        stack: &[Value],
        locals: &HashMap<String, Value>,
        ip: usize,
        function_name: &str,
    ) {
        let frame = DebugFrame {
            instruction: format!("{:?}", instruction),
            stack: stack.iter().map(|v| format!("{:?}", v)).collect(),
            locals: locals
                .iter()
                .map(|(k, v)| (k.clone(), format!("{:?}", v)))
                .collect(),
            ip,
            function_name: function_name.to_string(),
        };
        self.frames.push(frame);
    }

    pub fn generate_html(&self) -> String {
        include_str!("debug.template")
            .replace("{{TRACE_DATA}}", &serde_json::to_string(self).unwrap())
    }
}
