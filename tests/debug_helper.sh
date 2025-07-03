#!/bin/bash

# ChatShell Debug Helper
# This script helps diagnose issues with ChatShell

echo "=== ChatShell Debug Information ==="
echo

# System information
echo "--- System Information ---"
echo "OS: $(uname -a)"
echo "Shell: $SHELL"
echo "Terminal: $TERM"
echo "TERM_PROGRAM: $TERM_PROGRAM"
echo "Locale: $LANG"
echo

# Check dependencies
echo "--- Dependency Check ---"
echo "Bash: $(which bash || echo 'Not found')"
echo "Vi/Vim: $(which vi || echo 'Not found')"
echo "Rust: $(rustc --version 2>/dev/null || echo 'Not found')"
echo "Cargo: $(cargo --version 2>/dev/null || echo 'Not found')"
echo

# Build information
echo "--- Build Information ---"
if [ -f "../Cargo.toml" ]; then
    echo "Cargo.toml found"
    echo "Package name: $(grep '^name' ../Cargo.toml | cut -d'"' -f2)"
    echo "Version: $(grep '^version' ../Cargo.toml | cut -d'"' -f2)"
else
    echo "Cargo.toml not found"
fi

if [ -f "../target/debug/chatshell" ]; then
    echo "Debug binary: Present"
    echo "Debug binary size: $(ls -lh ../target/debug/chatshell | awk '{print $5}')"
else
    echo "Debug binary: Not found"
fi

if [ -f "../target/release/chatshell" ]; then
    echo "Release binary: Present"
    echo "Release binary size: $(ls -lh ../target/release/chatshell | awk '{print $5}')"
else
    echo "Release binary: Not found"
fi
echo

# Configuration
echo "--- Configuration ---"
if [ -f ~/.config/chatshell/config.toml ]; then
    echo "Config file: Present at ~/.config/chatshell/config.toml"
    echo "Config file size: $(ls -lh ~/.config/chatshell/config.toml | awk '{print $5}')"
    echo "Config content preview:"
    head -10 ~/.config/chatshell/config.toml
else
    echo "Config file: Not found"
fi
echo

# Test compilation
echo "--- Test Compilation Check ---"
cd ..
if cargo check --tests 2>/dev/null; then
    echo "✓ Tests compile successfully"
else
    echo "✗ Tests have compilation errors"
    echo "Running cargo check for details:"
    cargo check --tests
fi
echo

# Runtime test
echo "--- Quick Runtime Test ---"
if [ -f "target/debug/chatshell" ]; then
    echo "Testing basic functionality..."
    echo 'echo "Debug test" | exit' | timeout 5s ./target/debug/chatshell 2>/dev/null && echo "✓ Basic runtime test passed" || echo "✗ Basic runtime test failed"
else
    echo "No debug binary available for testing"
fi
echo

echo "=== Debug Information Complete ==="
echo "Use this information when reporting issues or troubleshooting."