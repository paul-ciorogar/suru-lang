use clap::Parser;
use suru_lang::cli::{Cli, Commands};
use suru_lang::{lexer, limits, parser, semantic};

fn main() {
    std::process::exit(match run() {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("Error: {}", e);
            1
        }
    });
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Parse(args) => parse_command(args)?,
        Commands::Check(args) => check_command(args)?,
    }

    Ok(())
}

fn check_command(args: suru_lang::cli::CheckArgs) -> Result<(), Box<dyn std::error::Error>> {
    let limits = match limits::CompilerLimits::from_project_toml("project.toml") {
        Ok(l) => {
            l.validate()?;
            l
        }
        Err(_) => limits::CompilerLimits::default(),
    };

    let source = std::fs::read_to_string(&args.file)
        .map_err(|e| format!("Failed to read '{}': {}", args.file, e))?;

    if source.len() > limits.max_input_size {
        return Err(format!(
            "Input too large: {} bytes (max: {})",
            source.len(),
            limits.max_input_size
        )
        .into());
    }

    let tokens = lexer::lex(&source, &limits)?;
    let ast = parser::parse(tokens, &limits)?;
    let analyzer = semantic::SemanticAnalyzer::new(ast);

    match analyzer.analyze() {
        Ok(_) => {
            println!("No errors found.");
            Ok(())
        }
        Err(errors) => {
            for error in &errors {
                eprintln!("{error}");
            }
            std::process::exit(1);
        }
    }
}

fn parse_command(args: suru_lang::cli::ParseArgs) -> Result<(), Box<dyn std::error::Error>> {
    // Load compiler limits from project.toml or use defaults
    let limits = match limits::CompilerLimits::from_project_toml("project.toml") {
        Ok(l) => {
            l.validate()?;
            l
        }
        Err(_) => {
            // Silently use defaults
            limits::CompilerLimits::default()
        }
    };

    // Read source file
    let source = std::fs::read_to_string(&args.file)
        .map_err(|e| format!("Failed to read '{}': {}", args.file, e))?;

    // Check input size limit
    if source.len() > limits.max_input_size {
        return Err(format!(
            "Input too large: {} bytes (max: {})",
            source.len(),
            limits.max_input_size
        )
        .into());
    }

    // Lex and parse
    let tokens = lexer::lex(&source, &limits)?;
    let ast = parser::parse(tokens, &limits)?;

    // Print AST tree
    print!("{}", ast.to_string());

    Ok(())
}
