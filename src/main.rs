fn main() {
    println!("{:?}", parser::parse_deck("hello"));
    println!("{:?}", parser::parse_deck("hello world"));
    println!("{:?}", parser::parse_deck("goodbye hello again"));
}
