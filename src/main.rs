mod codegen;
mod debug;
mod ir;
mod lexer;
mod optimizer;
mod parser;
mod vm;

use std::fs;
use std::path::Path;

const EXAMPLE_JS: &str = r#"
// Simple function to calculate fibonacci number
function fibonacci(n) {
    if (n <= 1) {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

// Main function that uses both functions
function main() {
    let n = 10;
    let fib = fibonacci(n);
    
    // Print results using our primitive print function
    print(n);          // 10
    print(fib);        // 55
    return fib;
}
"#;

fn main() {
    // If no arguments provided, use the example
    let source = if std::env::args().len() > 1 {
        let args: Vec<String> = std::env::args().collect();
        fs::read_to_string(&args[1]).expect("Failed to read source file")
    } else {
        String::from(EXAMPLE_JS)
    };

    println!("Compiling JavaScript:");
    println!("{}", source);
    println!("\nTokenizing...");
    let tokens = lexer::tokenize(&source);
    println!("Generated {} tokens", tokens.len());

    println!("\nParsing...");
    let ast = parser::parse(tokens);
    // println!("Generated AST {:?}", ast.statements);

    println!("\nGenerating IR...");
    let ir = ir::lower_ast(ast);
    // println!("Generated IR {:?}", ir);
    // println!("Generated {} IR functions", ir.functions.len());

    // println!("\nOptimizing...");
    // let optimized_ir = optimizer::optimize(ir);

    // Choose between targets based on features
    let target = if cfg!(feature = "x64") {
        codegen::Target::X64
    } else if cfg!(feature = "arm64") {
        codegen::Target::ARM64
    } else if cfg!(feature = "wasm") {
        codegen::Target::Wasm
    } else {
        codegen::Target::None
    };

    match target {
        codegen::Target::None => {
            println!("Running in VM mode (no native code generation)");
            let mut vm = vm::VM::new(ir);
            vm.enable_debugging();
            let result = vm.execute_function("main", vec![]);

            if let Some(debug_trace) = vm.get_debug_trace() {
                let html = debug_trace.generate_html();
                fs::write("debug_output.html", html).expect("Failed to write debug output");
                println!("Debug visualization written to debug_output.html");
            }

            match result {
                vm::Value::Number(n) => println!("Result: {}", n),
                vm::Value::String(s) => println!("Result: \"{}\"", s),
                vm::Value::Undefined => println!("Result: undefined"),
                _ => println!("Result: {:?}", result),
            }
        }
        _ => {
            println!("\nGenerating code for target {:?}...", target);
            if let Some(output) = codegen::generate_code(ir, target.clone()) {
                let extension = match target {
                    codegen::Target::X64 | codegen::Target::ARM64 => "s",
                    codegen::Target::Wasm => "wat",
                    _ => unreachable!(),
                };

                let output_path = if std::env::args().len() > 1 {
                    Path::new(&std::env::args().nth(1).unwrap()).with_extension(extension)
                } else {
                    Path::new(&format!("output.{}", extension)).to_path_buf()
                };

                fs::write(&output_path, output).expect("Failed to write output");
                println!("Output written to: {}", output_path.display());
            }
        }
    }
}
