use std::{collections::HashMap, mem};

use crate::{cluster_vector::*, response::{Response, ResponseSet}, state::State, word::Word, rank};

pub fn analyze(answers: &Vec<Word>, valid: &Vec<Word>, guesses_and_responses: Vec<(Word, Option<Response>)>) {
    let mut state = State::new(answers, valid);

    for (word, response) in guesses_and_responses {
        state.filter(word, response);
    }

    state.print();
}

pub fn stats(answers: &Vec<Word>, valid: &Vec<Word>, guesses_and_responses: Vec<(Word, Option<Response>)>) {
    let mut state = State::new(answers, valid);

    for (word, response) in guesses_and_responses {
        state.filter(word, response);
    }

    state.print_guesses();
    println!("  CV: {}", state.to_cluster_vector().to_string());
}

pub fn explain_hybrid(answers: Vec<Word>, guesses: Vec<Word>) {
    let mut turn = 0;
    let mut total_turns = 0;
    let total_words = answers.len();
    
    let mut map = HashMap::new();
    map.insert(ResponseSet::new(), answers);

    let mut inner_map = HashMap::new();

    for guess in guesses.iter() {
        turn += 1;
        rank::split_map(&map,*guess, 3, &mut inner_map);
        mem::swap(&mut map, &mut inner_map);

        let mut cv = ClusterVector::from_map(&map);
        println!("{}: {}", guess, cv.to_string());

        if turn < guesses.len() {
            cv.value.truncate(3);
        }

        let guessed_words = cv.word_count();
        let guessing_turns = cv.total_turns_pessimistic() + guessed_words * turn;

        if turn < guesses.len() {
            println!("   {} => {guessing_turns} turns ({:.1}%, {guessed_words} words)", cv.to_string(), (100.0 * guessed_words as f64) / total_words as f64);
            println!();
        }

        total_turns += guessing_turns;
    }

    let average_turns = total_turns as f64 / total_words as f64;
    println!("=> Average Turns: {average_turns:.4} ({total_turns} / {total_words})");
}

pub fn parse_into_guesses_and_responses<'a>(entries: Vec<String>) -> Vec<(Word, Option<Response>)> {
    let mut guess: Option<Word> = None;
    let mut result: Vec<(Word, Option<Response>)> = Vec::new();

    for entry in entries {
        // If there's a pending guess...
        let as_word = Word::new(&entry);
        let as_response = Response::from_str(&entry);
        
        if let Some(g) = guess {
            if let Some(r) = as_response {
                // If this is a response, add the pair
                result.push((g, Some(r)));
                guess = None;
            } else if let Some(_) = as_word {
                // If not, add the previous guess alone
                result.push((g, None));
                guess = as_word;
            } else {
                panic!("{:?} was not a valid guess or response.", &entry);
            }
        } else if let Some(_) = as_word {
            guess = as_word;
        } else {
            panic!("{:?} was not a valid guess or response.", &entry);
        }
    }

    // Add the last guess if there is one
    if let Some(g) = guess {
        result.push((g, None));
    }

    result
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn parse_into_guesses_and_responses() {
        let result = super::parse_into_guesses_and_responses(own(&["soare", "bbbbb", "clint"]));
        assert_eq!(vec![(w("soare"), Some(r("bbbbb"))), (w("clint"), None)], result);

        let result = super::parse_into_guesses_and_responses(own(&["yyyyy", "bbbbc"]));
        assert_eq!(vec![(w("yyyyy"), None), (w("bbbbc"), None)], result);
    }

    fn own(vec: &[&str]) -> Vec<String> {
        vec.iter().map(|s| String::from(*s)).collect()
    }
}