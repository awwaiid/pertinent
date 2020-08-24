fn main() {
    println!("{:?}", parser::parse_slides("hello"));
    println!("{:?}", parser::parse_slides("hello world"));
    println!("{:?}", parser::parse_slides("goodbye hello again"));
}
