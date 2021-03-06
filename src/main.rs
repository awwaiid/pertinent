use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();

    let filename = &args[1];
    let content = fs::read_to_string(filename)
        .expect("Error reading deck file");

    println!("{:?}", parser::parse_deck(&content));
}
