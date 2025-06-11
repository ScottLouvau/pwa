use crate::{word::Word, response::Response, rank, cluster_vector::ClusterVector};
use num_format::{Locale, ToFormattedString};
use std::{fs, path::Path, collections::HashMap, ops::Range};

/// One-Time: Merge answers and Guesses to emit 'Valid.txt'
pub fn merge_answers_into_guesses(base_folder: &str) -> () {
    let answers_path = format!("{}/{}", base_folder, "answers.txt");
    let answers = Word::parse_file(Path::new(&answers_path));

    let guesses_path = format!("{}/{}", base_folder, "guesses.txt");
    let guesses = Word::parse_file(&Path::new(&guesses_path));

    // Merge and sort answers and guesses
    let mut merged: Vec<String> = answers
        .iter()
        .chain(guesses.iter())
        .cloned()
        .map(|w| w.to_string())
        .collect();
    merged.sort();

    // Write merged 'valid.txt'
    let valid_path = format!("{}/{}", base_folder, "valid.txt");
    let contents = merged.join("\n");
    fs::write(&valid_path, contents).expect(&format!("Unable to write merged '{}'", &valid_path));

    let answers_count = answers.len().to_formatted_string(&Locale::en);
    let guesses_count = guesses.len().to_formatted_string(&Locale::en);
    println!("Merged {answers_count} answers with {guesses_count} guesses.");
}

#[derive(Copy, Clone)]
pub struct LetterNeighbors {
    letter: u8,
    count: u16,
    after: [u16; 26],
    before: [u16; 26]
}

impl LetterNeighbors {
    pub fn new() -> LetterNeighbors {
        LetterNeighbors {
            letter: 0,
            count: 0,
            after: [0; 26],
            before: [0; 26]
        }
    }
}

/// Show which letters appear right before or after each given letter across answers.
pub fn letter_neighbors(answers: &Vec<Word>, position_range: Range<usize>) -> [LetterNeighbors; 26] {
    let mut results = [LetterNeighbors::new(); 26];

    for (i, neighbors) in results.iter_mut().enumerate() {
        neighbors.letter = i as u8;
    }

    // Count occurrences of each letter (total and per position)
    for answer in answers.iter() {
        let mut last = 0;

        for (i, letter_index) in answer.iter_index().enumerate() {
            if position_range.contains(&i) {
                results[letter_index as usize].count += 1;
            }

            if i > 0 {
                // Add 'last' before this letter
                if position_range.contains(&i) {
                    results[letter_index as usize].before[last as usize] += 1;
                }

                // Add this letter after 'last'
                if position_range.contains(&(i - 1)) {
                    results[last as usize].after[letter_index as usize] += 1;
                }
            }

            last = letter_index;
        }
    }

    // Sort by letter frequency in tracked positions
    results.sort_by(|l, r| r.count.cmp(&l.count));

    results
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum NeighborSide {
    Before,
    After
}

pub fn print_letter_neighbors(neighbors: &[LetterNeighbors; 26], side: NeighborSide) -> String {
    let mut result = String::new();

    for record in neighbors {
        let set_array = if side == NeighborSide::Before { &record.before } else { &record.after };

        // Sort neighbors by frequency descending
        let mut set = Vec::new();
        for (other_letter, count) in set_array.iter().enumerate() {
            set.push((other_letter as u8, *count));
        }
        set.sort_by(|l, r| r.1.cmp(&l.1));

        result += &format!("{} ", i2c(record.letter));
        result += &format!("{:4}", record.count);

        // Write each >1% found neighbor around this letter, with the percentage frequency
        for (other_letter, count) in set {
            let first_letter = if side == NeighborSide::Before { other_letter } else { record.letter };
            let last_letter = if side == NeighborSide::Before { record.letter } else { other_letter };
            let percentage = 100.0 * count as f64 / record.count as f64;

            if percentage > 1.0 {
                result += &format!(" | {}{} {:2.0}", i2c(first_letter), i2c(last_letter), percentage);
            }
        }

        result += "\n";
    }

    result
}

pub fn consonant_pairs(answers: &Vec<Word>) -> HashMap<(u8, u8), u16> {
    let mut result = HashMap::new();

    for answer in answers.iter() {
        let mut last = None;

        for letter in answer.iter_index() {
            if let Some(l) = last {
                if is_consonant(l) && is_consonant(letter) {
                    *result.entry((l, letter)).or_insert(0) += 1;
                }
            }

            last = Some(letter);
        }
    }

    result
}

pub fn print_consonant_pairs(pairs: &HashMap<(u8, u8), u16>) -> String {
    let mut pair_vec = Vec::new();

    for pair in pairs.iter() {
        pair_vec.push((*pair.1, i2c(pair.0.0), i2c(pair.0.1)));
    }

    // Sort by frequency descending
    pair_vec.sort_by(|l, r| r.0.cmp(&l.0));

    let mut result = String::new();
    for pair in pair_vec.iter() {
        result += &format!("{} | {}{}\n", pair.0, pair.1, pair.2); 
    }

    result
}

fn is_consonant(index: u8) -> bool {
    // Not 'A', 'E', 'I', 'O', 'U'
    index != 0 && index != 4 && index != 8 && index != 14 && index != 20
}

fn i2c(index: u8) -> char {
    (b'A' + index) as char
}

pub struct LetterCounts {
    letter: char,
    total: u16,
    per_position: [u16; 5],
}

/// Show how many times each letter appears across answers, and how often per position
pub fn letter_frequency(answers: &Vec<Word>) -> Vec<LetterCounts> {
    let mut counts = Vec::new();

    for i in 0..26 {
        let letter = (b'a' + i as u8) as char;
        counts.push(LetterCounts { letter: letter, total: 0u16, per_position: [0u16; 5] });
    }

    // Count occurrences of each letter (total and per position)
    for answer in answers.iter() {
        for (i, letter_index) in answer.iter_index().enumerate() {
            counts[letter_index as usize].total += 1;
            counts[letter_index as usize].per_position[i] += 1;
        }
    }

    // Sort by total per letter descending
    counts.sort_by(|l, r| r.total.cmp(&l.total));

    // Show the counts
    println!();
    println!("Frequency of Letters in Answers:");
    for count in counts.iter() {
        println!("{}: {} {:?}", count.letter, count.total, count.per_position);
    }

    counts
}

/// Compute the odds each letter is first
pub fn letter_first_odds(counts: &Vec<LetterCounts>) -> Vec<(char, f32)> {
    let mut first_odds = Vec::new();

    for letter in counts {
        let odds = if letter.total > 0 { letter.per_position[0] as f32 / letter.total as f32 } else { 0.0 };
        first_odds.push((letter.letter, odds));
    }

    first_odds.sort_by(|l, r| r.1.total_cmp(&l.1).then_with(|| l.0.cmp(&r.0)));

    println!();
    println!("Odds Letter is First if Present:");
    for (letter, odds) in first_odds.iter() {
        println!("{}: {:.1}%", letter, 100.0 * odds);
    }

    first_odds
}

// Compute how often answers have a repeated letter
pub fn repeat_letter_odds(answers: &Vec<Word>) -> f64 {
    let mut total = 0;
    let mut dupes = 0;

    for answer in answers.iter() {
        total += 1;
        if answer.has_repeat_letters() {
            dupes += 1;
        }
    }

    let odds = dupes as f64 / total as f64;
    
    println!();
    println!("Odds of Repeat Letter: {:.2}%", 100.0 * odds);
    
    odds
}

pub fn score_answers_by_turns(answers: &Vec<Word>, guesses: &Vec<Word>) -> Vec<(usize, Word, ClusterVector)> {
    let mut answer_cluster_sizes: Vec<Vec<usize>> = Vec::new();
    for _ in answers.iter() {
        answer_cluster_sizes.push(Vec::new());
    }

    let mut map = HashMap::new();
    for guess in guesses.iter() {
        // Find count per cluster for this guess
        rank::counts(&answers, *guess, &mut map);

        // Add the size of cluster each answer ended up in for this guess
        for (i, answer) in answers.iter().enumerate() {
            let resp = Response::score(*guess, *answer);
            let size_for_guess = *map.get(&resp).unwrap_or(&1);

            let answer_clusters = &mut answer_cluster_sizes[i];
            while answer_clusters.len() < size_for_guess {
                answer_clusters.push(0);
            }
            answer_clusters[size_for_guess - 1] += 1;
        }
    }

    let mut results = Vec::new();
    for (i, answer) in answers.iter().enumerate() {
        let cv = ClusterVector::new(answer_cluster_sizes[i].clone());
        let score = cv.total_turns_pessimistic();
        let score = score / guesses.len();
        results.push((score, *answer, cv));
    }

    results.sort_by(|l, r| r.0.cmp(&l.0));
    results
}

pub fn score_guesses_by_responses(answers: &Vec<Word>, guesses: &Vec<Word>) -> Vec<(usize, Word)> {
    let mut results = Vec::new();

    let mut map = HashMap::new();
    for guess in guesses.iter() {
        // Find count per cluster for this guess
        rank::counts(answers, *guess, &mut map);
        results.push((map.len(), *guess));
    }

    results.sort_by(|l, r| r.0.cmp(&l.0));
    results
}

#[cfg(test)]
mod tests {
    use crate::*;
    //use std::{fs, path::Path};

    // Useful Test, but unreliable.
    //  Need to refactor method to return new file contents instead to avoid inconsistency depending on deletion vs. creation time.
    // #[test]
    // fn merge_answers_into_guesses() {
    //     // Delete valid.txt
    //     let base_path = "./tst/_o_er";
    //     let merged_path = format!("{}/{}", &base_path, "valid.txt");
    //     fs::remove_file(&merged_path).ok();

    //     // Request merge.
    //     super::merge_answers_into_guesses(&base_path);

    //     // Verify valid.txt exists, has correct word count, and has merged words in sorted order
    //     assert!(Path::new(&merged_path).exists());
    //     let valid = Word::parse_file(Path::new(&merged_path));
    //     assert_eq!(valid.len(), 76);
    //     assert_eq!(
    //         valid[0..5]
    //             .iter()
    //             .map(|w| w.to_string())
    //             .collect::<Vec<String>>(),
    //         ["boner", "borer", "bower", "boxer", "coder"]
    //     );

    //     fs::remove_file(&merged_path).ok();
    // }

    #[test]
    fn letter_stats() {
        let words = vec![w("every"), w("words"), w("there")];

        // Test counting letter position
        let counts = super::letter_frequency(&words);

        // Verify highest totals first (e = 4, r = 3), count per position correct
        assert_eq!(counts[0].letter, 'e');
        assert_eq!(counts[0].total, 4);
        assert_eq!(counts[0].per_position, [1, 0, 2, 0, 1]);

        assert_eq!(counts[1].letter, 'r');
        assert_eq!(counts[1].total, 3);
        assert_eq!(counts[1].per_position, [0, 0, 1, 2, 0]);

        // Test finding letter by position odds
        let odds = super::letter_first_odds(&counts);

        // 't' and 'w' were only first; sort is by odds and then letter, so 't' should sort first.
        assert_eq!(odds[0], ('t', 1.0));
        assert_eq!(odds[1], ('w', 1.0));

        // 'e' was first once out of 4 appearances.
        assert_eq!(odds[2], ('e', 1.0 / 4.0));

        // No other letters appeared first
        assert_eq!(odds[3].1, 0.0);

        // Test repeat letter odds; 2/3 have a repeat
        let repeat_odds = super::repeat_letter_odds(&words);
        assert_eq!(repeat_odds, 2.0 / 3.0);
    }

    #[test]
    fn is_consonant() {
        assert_eq!(false, super::is_consonant(b'A' - b'A'));
        assert_eq!(false, super::is_consonant(b'E' - b'A'));
        assert_eq!(false, super::is_consonant(b'I' - b'A'));
        assert_eq!(false, super::is_consonant(b'O' - b'A'));
        assert_eq!(false, super::is_consonant(b'U' - b'A'));

        assert_eq!(true, super::is_consonant(b'B' - b'A'));
        assert_eq!(true, super::is_consonant(b'F' - b'A'));
        assert_eq!(true, super::is_consonant(b'N' - b'A'));
        assert_eq!(true, super::is_consonant(b'V' - b'A'));
    }
}
