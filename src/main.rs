mod ast;
mod parser;

use parser::AsciiDocParser;
use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() != 2 {
        eprintln!("Usage: {} <input.adoc>", args[0]);
        process::exit(1);
    }
    
    let input_path = &args[1];
    
    let content = match fs::read_to_string(input_path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", input_path, e);
            process::exit(1);
        }
    };
    
    match AsciiDocParser::parse_document(&content) {
        Ok(document) => {
            let html = document.to_html();
            println!("{}", html);
        }
        Err(e) => {
            eprintln!("Parse error: {}", e);
            process::exit(1);
        }
    }
}
