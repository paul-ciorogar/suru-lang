pub mod codegen;
pub mod lexer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    codegen::generate_hello_world()?;
    Ok(())
}
