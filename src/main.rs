mod codegen;
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
    println!("Generated AST {:?}", ast.statements);

    println!("\nGenerating IR...");
    let ir = ir::lower_ast(ast);
    println!("Generated IR {:?}", ir);
    // println!("Generated {} IR functions", ir.functions.len());

    // println!("\nOptimizing...");
    // let optimized_ir = optimizer::optimize(ir);

    // Choose between VM execution or native code generation
    if cfg!(feature = "x64") {
        println!("\nGenerating x64 assembly...");
        let assembly = codegen::generate_x64(ir);

        let output_path = if std::env::args().len() > 1 {
            Path::new(&std::env::args().nth(1).unwrap()).with_extension("s")
        } else {
            Path::new("output.s").to_path_buf()
        };

        fs::write(&output_path, assembly).expect("Failed to write assembly output");
        println!("Assembly written to: {}", output_path.display());
    } else {
        println!("\nExecuting in VM...");
        let mut vm = vm::VM::new(ir);
        let result = vm::Value::Number(0.0);
        match vm.execute_function("main", vec![]) {
            vm::Value::Number(n) => println!("Result: {}", n),
            vm::Value::String(s) => println!("Result: \"{}\"", s),
            vm::Value::Undefined => println!("Result: undefined"),
            _ => println!("Result: {:?}", result),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fibonacci() {
        let tokens = lexer::tokenize(EXAMPLE_JS);
        let ast = parser::parse(tokens);
        let ir = ir::lower_ast(ast);
        let mut vm = vm::VM::new(ir);

        if let vm::Value::Number(result) =
            vm.execute_function("fibonacci", vec![vm::Value::Number(10.0)])
        {
            assert!((result - 55.0).abs() < f64::EPSILON);
        } else {
            panic!("Expected number result from fibonacci(10)");
        }
    }

    #[test]
    fn test_is_even() {
        let tokens = lexer::tokenize(EXAMPLE_JS);
        let ast = parser::parse(tokens);
        let ir = ir::lower_ast(ast);
        let mut vm = vm::VM::new(ir);

        if let vm::Value::Boolean(result) =
            vm.execute_function("isEven", vec![vm::Value::Number(55.0)])
        {
            assert_eq!(result, false);
        } else {
            panic!("Expected boolean result from isEven(55)");
        }
    }
}
