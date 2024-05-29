use response::Response;
use word::Word;

pub mod analyze;
pub mod check;
pub mod bit_vector_slice;
pub mod clubs;
pub mod cluster_vector;
pub mod letter_orders;
pub mod parser;
pub mod rank;
pub mod response;
pub mod scrappy;
pub mod search;
pub mod single_use;
pub mod state;
pub mod word;
pub mod wordle_tree;

pub fn wv(words: &str) -> Vec<Word> {
    words.split(",").map(|word| w(word.trim())).collect()
}

pub fn wv_safe(words: &str) -> Result<Vec<Word>, String> {
    let mut result = Vec::new();

    for word in words.split(',') {
        if let Some(word) = Word::new(word.trim()) {
            result.push(word);
        } else {
            return Err(format!("'{word}' was not a valid Wordle word."));
        }
    }

    Ok(result)
}

pub fn w(text: &str) -> Word {
    Word::new(text).unwrap()
}

pub fn r(text: &str) -> Response {
    Response::from_str(text).unwrap()
}

pub fn write_turns(outer_total_turns: f64, situation_count: f64, write_average: bool) -> String {
    if write_average {
        // Average is always written with three decimals
        let average = outer_total_turns / situation_count;
        format!("{average:.3}")
    } else {
        // Write one decimal place if the value has partial turns and is under 1,000 turns
        let tenths = outer_total_turns - ((outer_total_turns as usize) as f64);
        if tenths < 0.05 || tenths >= 0.949 || outer_total_turns >= 99.9 {
            format!("{outer_total_turns:.0}")
        } else {
            format!("{outer_total_turns:.1}")
        }
    }
}

pub fn pad_to_length(length: usize, result: &mut String) {
    if result.len() < length {
        let pad_count = length - result.len();
        for _ in 0..pad_count {
            result.push(' ');
        }
    }
}

pub fn smart_trim(text: &str) -> String {
    let mut result = String::new();

    for line in text.lines() {
        let mut saw_non_space = false;
        let mut last = '\n';

        for c in line.chars() {
            if c != ' ' {
                saw_non_space = true;
                result.push(c);
            } else if saw_non_space == true && last == ' ' {
                // Trim spaces *after the initial indent* down to a single space
            } else {
                result.push(c);
            }

            last = c;
        }

        // No empty lines
        if saw_non_space { result.push('\n'); }
    }

    // No trailing newline
    if result.ends_with("\n") {
        result.pop();
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_turns() {
        // Total Turns, no fractional part
        assert_eq!(write_turns(15.0, 3.0, false), "15");

        // Total Turns w/fractional part
        assert_eq!(write_turns(15.4, 3.0, false), "15.4");

        // Total Turns, rounds (down/up) to no fraction
        assert_eq!(write_turns(15.0499, 3.0, false), "15");
        assert_eq!(write_turns(15.95, 3.0, false), "16");

        // Total turns, too big to show fraction
        assert_eq!(write_turns(100.4, 3.0, false), "100");

        // Average Turns (three decimal places)
        assert_eq!(write_turns(12.3456, 10.0, true), "1.235");
    }

    #[test]
    fn test_smart_trim() {
        // 1. Keep leading spaces.
        // 2. Remove multiple spaces after initial indent.
        // 3. Remove empty lines
        // 4. No trailing newline
        assert_eq!(smart_trim("    100  (nice, 34)  ->  cool\n\n"), "    100 (nice, 34) -> cool");
    }
}