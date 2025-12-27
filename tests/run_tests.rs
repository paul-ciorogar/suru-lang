use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Find all test directories in tests/run/
fn find_run_tests() -> Vec<PathBuf> {
    let run_dir = Path::new("tests/run");
    let mut test_dirs = Vec::new();

    if let Ok(entries) = fs::read_dir(run_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Check if it has a main.suru file
                let main_file = path.join("main.suru");
                if main_file.exists() {
                    test_dirs.push(path);
                }
            }
        }
    }

    test_dirs.sort();
    test_dirs
}

/// Run a single test case
fn run_test_case(test_dir: &Path) -> Result<(), String> {
    let test_name = test_dir.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    let main_file = test_dir.join("main.suru");
    let expected_output_file = test_dir.join("expected_output.txt");

    // Check if expected output file exists
    if !expected_output_file.exists() {
        return Err(format!(
            "Test '{}': expected_output.txt not found",
            test_name
        ));
    }

    // Read expected output
    let expected_output = fs::read_to_string(&expected_output_file)
        .map_err(|e| format!("Test '{}': failed to read expected_output.txt: {}", test_name, e))?;

    // Run the suru run command
    // Note: We use cargo run -- run instead of calling suru directly
    let output = Command::new("cargo")
        .arg("run")
        .arg("--")
        .arg("run")
        .arg(&main_file)
        .output()
        .map_err(|e| format!("Test '{}': failed to execute suru run: {}", test_name, e))?;

    // Check if the command succeeded
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Test '{}': suru run failed with exit code {:?}\nStderr: {}",
            test_name,
            output.status.code(),
            stderr
        ));
    }

    // Compare output
    let actual_output = String::from_utf8_lossy(&output.stdout);
    if actual_output.trim() != expected_output.trim() {
        return Err(format!(
            "Test '{}': output mismatch\nExpected:\n{}\nActual:\n{}",
            test_name,
            expected_output.trim(),
            actual_output.trim()
        ));
    }

    Ok(())
}

#[test]
#[ignore] // Remove this when 'suru run' command is implemented
fn test_run_integration() {
    let test_dirs = find_run_tests();

    if test_dirs.is_empty() {
        panic!("No integration tests found in tests/run/");
    }

    let mut failures = Vec::new();

    for test_dir in &test_dirs {
        let test_name = test_dir.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        print!("Running test '{}' ... ", test_name);

        match run_test_case(test_dir) {
            Ok(_) => {
                println!("ok");
            }
            Err(e) => {
                println!("FAILED");
                failures.push(e);
            }
        }
    }

    if !failures.is_empty() {
        eprintln!("\nFailures:");
        for failure in &failures {
            eprintln!("  {}", failure);
        }
        panic!("{} test(s) failed", failures.len());
    }
}

// Individual test for each test case - makes it easier to run specific tests
#[test]
#[ignore] // Remove this when 'suru run' command is implemented
fn test_run_hello_world() {
    let test_dir = Path::new("tests/run/hello_world");
    if let Err(e) = run_test_case(test_dir) {
        panic!("{}", e);
    }
}
