# Integration Tests Guide

Welcome! This guide will help you understand and add new integration tests for the compiler.

## Overview

The integration test system automatically discovers and runs tests in the `integration_tests/` directory. Each test is a separate folder containing a source file and an expected output file.

## Quick Start

### Running Tests

```bash
# Build the compiler first
make compiler

# Run all integration tests
./test_runner ./your_compiler

# Or if integrated into your build system
make test
```

### Test Results

- **[PASS]** - Test passed (green)
- **[FAIL]** - Test failed (red)
- **[ERROR]** - Test configuration error (yellow)

## Adding a New Test

### Step 1: Create a Test Folder

Create a new folder in `integration_tests/` with a descriptive name:

```bash
mkdir integration_tests/test_my_feature
```

**Naming Convention:** Use `test_` prefix followed by a descriptive name (e.g., `test_array_indexing`, `test_function_calls`)

### Step 2: Add Your Source File

Add the source code you want to test. The file can have any extension (`.c`, `.src`, `.txt`), but avoid naming it `expected*`:

```bash
# Create your test source
cat > integration_tests/test_my_feature/program.c << 'EOF'
int main() {
    printf("Hello, World!\n");
    return 0;
}
EOF
```

### Step 3: Choose Test Type and Add Expected File

The test system uses **convention over configuration**. The type of test is determined by which expected file you create:

## Test Types

### 1. Compile and Run Test (Most Common)

**Use when:** You want to compile the source, run the resulting executable, and verify its output.

**Expected file:** `expected_output.txt`

```bash
# Create expected output
cat > integration_tests/test_my_feature/expected_output.txt << 'EOF'
Hello, World!
EOF
```

**What the test does:**
1. ✓ Compiles your source file
2. ✓ Checks that compilation succeeds
3. ✓ Runs the compiled executable
4. ✓ Compares executable output with `expected_output.txt`

---

### 2. Compilation Output Test

**Use when:** You want to verify the compiler's own output (warnings, messages, etc.) but don't need to run the executable.

**Expected file:** `expected.txt`

```bash
# Create expected compiler output
cat > integration_tests/test_warnings/expected.txt << 'EOF'
Warning: unused variable 'x' on line 5
Compilation successful
EOF
```

**What the test does:**
1. ✓ Compiles your source file
2. ✓ Checks that compilation succeeds (exit code 0)
3. ✓ Compares compiler's stdout/stderr with `expected.txt`

---

### 3. Error Handling Test

**Use when:** You want to verify that the compiler correctly rejects invalid code with the right error messages.

**Expected file:** `expected_error.txt`

```bash
# Create expected error message
cat > integration_tests/test_syntax_error/expected_error.txt << 'EOF'
Error: unexpected token '}' on line 3
Error: expected ';' before '}'
Compilation failed
EOF
```

**What the test does:**
1. ✓ Compiles your source file
2. ✓ Checks that compilation **fails** (non-zero exit code)
3. ✓ Compares error messages with `expected_error.txt`

---

### 4. File Generation Test

**Use when:** You want to verify that the compiler creates specific output files (e.g., `.s` assembly files, `.o` object files).

**Expected file:** `expected_files.txt`

```bash
# List the files that should be created
cat > integration_tests/test_codegen/expected_files.txt << 'EOF'
output.s
output.o
test_executable
EOF
```

**What the test does:**
1. ✓ Compiles your source file
2. ✓ Checks that compilation succeeds
3. ✓ Verifies each listed file exists in the test folder

**Note:** List one filename per line. Comments (lines starting with `#`) and empty lines are ignored.

---

## Complete Example

Let's add a test for array operations:

```bash
# 1. Create test folder
mkdir integration_tests/test_array_sum

# 2. Create source file
cat > integration_tests/test_array_sum/source.c << 'EOF'
int main() {
    int arr[5] = {1, 2, 3, 4, 5};
    int sum = 0;
    
    for (int i = 0; i < 5; i++) {
        sum += arr[i];
    }
    
    printf("Sum: %d\n", sum);
    return 0;
}
EOF

# 3. Create expected output
cat > integration_tests/test_array_sum/expected_output.txt << 'EOF'
Sum: 15
EOF

# 4. Run tests
./test_runner ./your_compiler
```

You should see:
```
Running: test_array_sum
  Compiling: ./your_compiler integration_tests/test_array_sum/source.c -o ...
  Running: integration_tests/test_array_sum/test_executable > ...
[PASS] test_array_sum
```

## Tips and Best Practices

### ✅ DO

- **Use descriptive test names** - `test_nested_loops` not `test1`
- **One feature per test** - Keep tests focused and simple
- **Test edge cases** - Zero values, empty inputs, boundary conditions
- **Match output exactly** - Include newlines, spacing, and punctuation
- **Add comments** - Explain complex test scenarios in the source

### ❌ DON'T

- **Don't name files starting with `expected`** - These are reserved for test configuration
- **Don't use multiple expected files** - Only one type per test
- **Don't forget newlines** - Most programs end output with `\n`
- **Don't make tests interdependent** - Each test should run independently

## Common Issues

### Test Fails: "Output does not match"

**Cause:** The actual output differs from expected output, often by whitespace.

**Solution:**
```bash
# Check actual output
cat integration_tests/test_name/actual_output.txt

# Compare with expected
diff integration_tests/test_name/expected_output.txt \
     integration_tests/test_name/actual_output.txt
```

### Test Fails: "No source file found"

**Cause:** Your source file might be named `expected_something` or have an unrecognized extension.

**Solution:** Rename to something like `program.c`, `source.c`, or `test.c`

### Test Shows as "UNKNOWN"

**Cause:** No expected file was found in the test folder.

**Solution:** Add one of: `expected.txt`, `expected_error.txt`, `expected_output.txt`, or `expected_files.txt`

### Executable Not Created

**Cause:** Compilation failed but test expected it to succeed.

**Solution:** Check `compiler_output.txt` in the test folder for error messages.

## Test Organization

Organize tests by feature or category:

```
integration_tests/
├── test_arithmetic_basic/
├── test_arithmetic_overflow/
├── test_functions_simple/
├── test_functions_recursive/
├── test_loops_for/
├── test_loops_while/
├── test_errors_syntax/
├── test_errors_type/
└── test_codegen_optimization/
```

## Getting Help

If you encounter issues:

1. Check the test folder for generated files:
   - `compiler_output.txt` - Compiler's stdout/stderr
   - `actual_output.txt` - Executable's output (for RUN_OUTPUT tests)
   - `test_executable` - The compiled binary

2. Run the compiler manually:
   ```bash
   ./your_compiler integration_tests/test_name/source.c
   ```

3. Review test output for specific error messages

## Advanced: Test Priority

Tests are discovered in directory order. If you want specific tests to run first, use naming:

```
test_00_basic/
test_01_intermediate/
test_02_advanced/
```

---

Happy testing! 🚀