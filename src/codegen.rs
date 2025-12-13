use inkwell::context::Context;
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode,
    Target, TargetMachine,
};
use inkwell::OptimizationLevel;
use inkwell::AddressSpace;
use std::path::Path;

pub fn generate_hello_world() -> Result<(), Box<dyn std::error::Error>> {
    // A. Setup and Initialization
    // Initialize LLVM target for native code generation
    Target::initialize_native(&InitializationConfig::default())?;

    // Create LLVM context (container for all LLVM entities)
    let context = Context::create();
    let module = context.create_module("hello_world");
    let builder = context.create_builder();

    // B. Define Types
    let i32_type = context.i32_type();
    let i8_ptr_type = context.ptr_type(AddressSpace::default());

    // C. Declare External printf Function
    // Declare printf: int printf(const char* format, ...)
    // The 'true' parameter indicates it's variadic
    let printf_type = i32_type.fn_type(&[i8_ptr_type.into()], true);
    let printf_fn = module.add_function("printf", printf_type, None);

    // D. Create Main Function
    let main_type = i32_type.fn_type(&[], false);
    let main_fn = module.add_function("main", main_type, None);
    let entry_block = context.append_basic_block(main_fn, "entry");

    builder.position_at_end(entry_block);

    // E. Create Global String Constant and Call printf
    let hello_string = "Hello, world!\n";
    // Use the builder's build_global_string_ptr for easier string creation
    let global_string = builder.build_global_string_ptr(hello_string, ".str")?;

    // Call printf with the string pointer
    let string_ptr = global_string.as_pointer_value();
    builder.build_call(printf_fn, &[string_ptr.into()], "printf_call")?;

    // F. Return from Main
    let return_value = i32_type.const_int(0, false);
    builder.build_return(Some(&return_value))?;

    // G. Verify and Print Module
    if let Err(err) = module.verify() {
        eprintln!("Module verification failed: {}", err);
        return Err("Invalid LLVM module".into());
    }

    println!("Generated LLVM IR:");
    println!("{}", module.print_to_string().to_string());
    println!();

    // H. Compile to Object File
    let target_triple = TargetMachine::get_default_triple();
    let target = Target::from_triple(&target_triple)
        .map_err(|e| format!("Failed to get target: {}", e))?;

    let target_machine = target
        .create_target_machine(
            &target_triple,
            "generic",
            "",
            OptimizationLevel::Default,
            RelocMode::Default,
            CodeModel::Default,
        )
        .ok_or("Failed to create target machine")?;

    let output_path = Path::new("hello.o");
    target_machine.write_to_file(&module, FileType::Object, output_path)
        .map_err(|e| format!("Failed to write object file: {}", e))?;

    println!("Object file written to: {}", output_path.display());

    // I. Link to Executable
    let link_status = std::process::Command::new("clang-18")
        .args(&["hello.o", "-o", "hello", "-no-pie"])
        .status()?;

    if !link_status.success() {
        return Err("Linking failed".into());
    }

    println!("Executable created: ./hello");
    println!("\nRun with: ./hello");

    Ok(())
}
