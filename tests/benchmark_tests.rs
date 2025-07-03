use std::time::{Duration, Instant};
use crossterm::event::{KeyCode, KeyModifiers};
use chatshell::terminal::KeyInput;
use chatshell::hooks::{HookManager, create_default_hooks};

/// Test performance of key-to-bytes conversion
#[test]
fn benchmark_key_conversion() {
    let test_keys = vec![
        (KeyCode::Char('a'), KeyModifiers::empty()),
        (KeyCode::Char('a'), KeyModifiers::CONTROL),
        (KeyCode::Char('a'), KeyModifiers::ALT),
        (KeyCode::Up, KeyModifiers::empty()),
        (KeyCode::F(1), KeyModifiers::empty()),
        (KeyCode::F(12), KeyModifiers::empty()),
        (KeyCode::Enter, KeyModifiers::empty()),
        (KeyCode::Tab, KeyModifiers::empty()),
    ];
    
    const ITERATIONS: usize = 10_000;
    
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        for &(code, modifiers) in &test_keys {
            let _key_input = KeyInput::new(code, modifiers);
        }
    }
    let duration = start.elapsed();
    
    println!("Key conversion: {} iterations in {:?} ({:.2} µs per conversion)", 
             ITERATIONS * test_keys.len(), 
             duration,
             duration.as_micros() as f64 / (ITERATIONS * test_keys.len()) as f64);
    
    // Should be very fast - under 50ms for all iterations
    assert!(duration < Duration::from_millis(50));
}

/// Test performance of pattern matching
#[test]
fn benchmark_pattern_matching() {
    let patterns = vec![
        "ctrl+a", "ctrl+c", "ctrl+z", "alt+enter", "ctrl+shift+c",
        "f1", "f12", "up", "down", "left", "right", "home", "end"
    ];
    
    let keys = vec![
        KeyInput::new(KeyCode::Char('a'), KeyModifiers::CONTROL),
        KeyInput::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        KeyInput::new(KeyCode::Char('z'), KeyModifiers::CONTROL),
        KeyInput::new(KeyCode::Enter, KeyModifiers::ALT),
        KeyInput::new(KeyCode::Char('c'), KeyModifiers::CONTROL | KeyModifiers::SHIFT),
        KeyInput::new(KeyCode::F(1), KeyModifiers::empty()),
        KeyInput::new(KeyCode::F(12), KeyModifiers::empty()),
        KeyInput::new(KeyCode::Up, KeyModifiers::empty()),
        KeyInput::new(KeyCode::Down, KeyModifiers::empty()),
        KeyInput::new(KeyCode::Left, KeyModifiers::empty()),
        KeyInput::new(KeyCode::Right, KeyModifiers::empty()),
        KeyInput::new(KeyCode::Home, KeyModifiers::empty()),
        KeyInput::new(KeyCode::End, KeyModifiers::empty()),
    ];
    
    const ITERATIONS: usize = 10_000;
    
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        for key in &keys {
            for pattern in &patterns {
                let _matches = key.matches_pattern(pattern);
            }
        }
    }
    let duration = start.elapsed();
    
    println!("Pattern matching: {} comparisons in {:?} ({:.2} µs per comparison)", 
             ITERATIONS * keys.len() * patterns.len(), 
             duration,
             duration.as_micros() as f64 / (ITERATIONS * keys.len() * patterns.len()) as f64);
    
    // Should be fast - under 1000ms for all iterations
    assert!(duration < Duration::from_millis(1000));
}

/// Test performance of hook processing
#[test]
fn benchmark_hook_processing() {
    let hooks = create_default_hooks();
    let hook_manager = HookManager::from_configs(hooks);
    
    let test_keys = vec![
        KeyInput::new(KeyCode::Char(';'), KeyModifiers::CONTROL), // Will match help hook
        KeyInput::new(KeyCode::Char('a'), KeyModifiers::empty()), // Won't match
        KeyInput::new(KeyCode::Char('x'), KeyModifiers::CONTROL), // Won't match
        KeyInput::new(KeyCode::Enter, KeyModifiers::empty()), // Won't match
    ];
    
    const ITERATIONS: usize = 1_000;
    
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        for key in &test_keys {
            let _ = hook_manager.process_key(key);
        }
    }
    let duration = start.elapsed();
    
    println!("Hook processing: {} keys in {:?} ({:.2} µs per key)", 
             ITERATIONS * test_keys.len(), 
             duration,
             duration.as_micros() as f64 / (ITERATIONS * test_keys.len()) as f64);
    
    // Should be reasonably fast - under 100ms
    assert!(duration < Duration::from_millis(100));
}

/// Test memory usage by creating many key inputs
#[test]
fn test_memory_usage() {
    let mut keys = Vec::new();
    
    // Create a large number of different key inputs
    for c in 'a'..='z' {
        for &ctrl in &[false, true] {
            for &alt in &[false, true] {
                for &shift in &[false, true] {
                    let mut modifiers = KeyModifiers::empty();
                    if ctrl { modifiers |= KeyModifiers::CONTROL; }
                    if alt { modifiers |= KeyModifiers::ALT; }
                    if shift { modifiers |= KeyModifiers::SHIFT; }
                    
                    keys.push(KeyInput::new(KeyCode::Char(c), modifiers));
                }
            }
        }
    }
    
    // Add special keys
    for f_num in 1..=12 {
        keys.push(KeyInput::new(KeyCode::F(f_num), KeyModifiers::empty()));
    }
    
    println!("Created {} key inputs", keys.len());
    
    // Verify they all work
    let mut total_bytes = 0;
    for key in &keys {
        assert!(!key.raw_bytes.is_empty());
        total_bytes += key.raw_bytes.len();
    }
    
    println!("Total raw bytes: {}", total_bytes);
    
    // Basic sanity check - shouldn't be empty
    assert!(!keys.is_empty());
}

/// Test rapid sequential key processing
#[test]
fn test_rapid_sequential_processing() {
    let hook_manager = HookManager::new(); // Empty hook manager for speed
    
    // Simulate typing a long document
    let text = "The quick brown fox jumps over the lazy dog. ".repeat(100);
    let keys: Vec<KeyInput> = text.chars()
        .map(|c| KeyInput::new(KeyCode::Char(c), KeyModifiers::empty()))
        .collect();
    
    const ITERATIONS: usize = 10;
    
    let start = Instant::now();
    for _ in 0..ITERATIONS {
        for key in &keys {
            let _ = hook_manager.process_key(key);
        }
    }
    let duration = start.elapsed();
    
    println!("Processed {} keys in {:?} ({:.2} keys/sec)", 
             ITERATIONS * keys.len(), 
             duration,
             (ITERATIONS * keys.len()) as f64 / duration.as_secs_f64());
    
    // Should be able to handle thousands of keys per second
    let keys_per_second = (ITERATIONS * keys.len()) as f64 / duration.as_secs_f64();
    assert!(keys_per_second > 1000.0);
}