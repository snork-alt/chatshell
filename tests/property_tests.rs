use proptest::prelude::*;
use crossterm::event::{KeyCode, KeyModifiers};
use chatshell::terminal::KeyInput;

// Simple property test for basic key functionality
proptest! {
    #[test]
    fn test_key_bytes_not_empty(
        ctrl in any::<bool>(),
        alt in any::<bool>(),
    ) {
        let key_input = KeyInput::new(KeyCode::Char('a'), {
            let mut modifiers = KeyModifiers::empty();
            if ctrl { modifiers |= KeyModifiers::CONTROL; }
            if alt { modifiers |= KeyModifiers::ALT; }
            modifiers
        });
        
        // Basic properties that should always hold
        prop_assert!(!key_input.raw_bytes.is_empty());
    }
}

// Property test for special keys
proptest! {
    #[test]
    fn test_special_keys_have_sequences(
        key in prop_oneof![
            Just(KeyCode::Up),
            Just(KeyCode::Down),
            Just(KeyCode::Left),
            Just(KeyCode::Right),
            Just(KeyCode::Home),
            Just(KeyCode::End),
            Just(KeyCode::PageUp),
            Just(KeyCode::PageDown),
            Just(KeyCode::Delete),
            Just(KeyCode::Insert),
            Just(KeyCode::Enter),
            Just(KeyCode::Tab),
            Just(KeyCode::Backspace),
            Just(KeyCode::Esc),
        ]
    ) {
        let key_input = KeyInput::new(key, KeyModifiers::empty());
        
        // All special keys should produce byte sequences
        prop_assert!(!key_input.raw_bytes.is_empty());
        
        // Most navigation keys should start with ESC (27)
        match key {
            KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right |
            KeyCode::Home | KeyCode::End | KeyCode::PageUp | KeyCode::PageDown |
            KeyCode::Delete | KeyCode::Insert => {
                prop_assert_eq!(key_input.raw_bytes[0], 27);
            }
            KeyCode::Enter => prop_assert_eq!(key_input.raw_bytes[0], 13), // \r
            KeyCode::Tab => prop_assert_eq!(key_input.raw_bytes[0], 9),   // \t
            KeyCode::Backspace => prop_assert_eq!(key_input.raw_bytes[0], 127),
            KeyCode::Esc => prop_assert_eq!(key_input.raw_bytes[0], 27),
            _ => {}
        }
    }
}

// Property test for function keys
proptest! {
    #[test]
    fn test_function_keys(f_num in 1u8..=12) {
        let key_input = KeyInput::new(KeyCode::F(f_num), KeyModifiers::empty());
        
        // All function keys should produce sequences
        prop_assert!(!key_input.raw_bytes.is_empty());
        
        // All function keys should start with ESC
        prop_assert_eq!(key_input.raw_bytes[0], 27);
        
        // F1-F4 should be shorter sequences than F5-F12
        if f_num <= 4 {
            prop_assert!(key_input.raw_bytes.len() <= 3);
        } else {
            prop_assert!(key_input.raw_bytes.len() > 3);
        }
    }
}