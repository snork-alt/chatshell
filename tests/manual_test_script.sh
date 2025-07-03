#!/bin/bash

# ChatShell Manual Test Script
# This script provides guided manual testing for ChatShell functionality

set -e

echo "=== ChatShell Manual Test Suite ==="
echo "This script will guide you through testing ChatShell functionality"
echo

# Check if chatshell is built
if [ ! -f "../target/debug/chatshell" ] && [ ! -f "../target/release/chatshell" ]; then
    echo "Building ChatShell..."
    cargo build
fi

CHATSHELL_BIN="../target/debug/chatshell"
if [ ! -f "$CHATSHELL_BIN" ]; then
    CHATSHELL_BIN="../target/release/chatshell"
fi

echo "Using ChatShell binary: $CHATSHELL_BIN"
echo

# Test 1: Basic shell functionality
echo "=== Test 1: Basic Shell Functionality ==="
echo "This will start ChatShell. Test the following:"
echo "1. Type 'echo Hello World' and press Enter"
echo "2. Try some basic commands like 'ls', 'pwd', 'date'"
echo "3. Press Ctrl+C to exit when done"
echo
read -p "Press Enter to start test 1..."
$CHATSHELL_BIN
echo "Test 1 completed."
echo

# Test 2: Special keys test
echo "=== Test 2: Special Keys Test ==="
echo "This will start ChatShell. Test the following special keys:"
echo "1. Arrow keys (Up/Down/Left/Right) for command history and navigation"
echo "2. Home/End keys for line beginning/end"
echo "3. Tab key for command completion"
echo "4. Ctrl+A (beginning of line), Ctrl+E (end of line)"
echo "5. Ctrl+L (clear screen)"
echo "6. Press Ctrl+C to exit when done"
echo
read -p "Press Enter to start test 2..."
$CHATSHELL_BIN
echo "Test 2 completed."
echo

# Test 3: Vi editor test
echo "=== Test 3: Vi Editor Test ==="
echo "This will test complex program interaction through ChatShell:"
echo "1. Type 'vi test_file.txt' and press Enter"
echo "2. Press 'i' to enter insert mode"
echo "3. Type some text"
echo "4. Press Esc to exit insert mode"
echo "5. Type ':wq' and press Enter to save and quit"
echo "6. Type 'cat test_file.txt' to verify the content"
echo "7. Press Ctrl+C to exit when done"
echo
read -p "Press Enter to start test 3..."
$CHATSHELL_BIN
echo "Test 3 completed."
echo

# Test 4: Hook functionality test
echo "=== Test 4: Hook Functionality Test ==="
echo "This will test the hook system:"
echo "1. Press Ctrl+; (Ctrl and semicolon) - should show help"
echo "2. Try other configured hooks if any"
echo "3. Verify that regular keys still work normally"
echo "4. Press Ctrl+C to exit when done"
echo
read -p "Press Enter to start test 4..."
$CHATSHELL_BIN
echo "Test 4 completed."
echo

# Test 5: Stress test
echo "=== Test 5: Stress Test ==="
echo "This will test rapid input handling:"
echo "1. Type rapidly and continuously for 10 seconds"
echo "2. Try holding down keys"
echo "3. Mix regular typing with special keys"
echo "4. Verify all input is processed correctly"
echo "5. Press Ctrl+C to exit when done"
echo
read -p "Press Enter to start test 5..."
$CHATSHELL_BIN
echo "Test 5 completed."
echo

# Test 6: Configuration test
echo "=== Test 6: Configuration Test ==="
echo "Testing configuration functionality:"
echo "1. Check if config file exists"
if [ -f ~/.config/chatshell/config.toml ]; then
    echo "   ✓ Config file found at ~/.config/chatshell/config.toml"
    echo "   Current config:"
    cat ~/.config/chatshell/config.toml
else
    echo "   ✗ Config file not found"
    echo "   Creating default config..."
    $CHATSHELL_BIN --create-config
fi
echo

echo "=== All Manual Tests Completed ==="
echo "If you encountered any issues, please report them with:"
echo "- Steps to reproduce"
echo "- Expected vs actual behavior"
echo "- Terminal environment details"
echo

# Cleanup
if [ -f "test_file.txt" ]; then
    rm test_file.txt
    echo "Cleaned up test_file.txt"
fi