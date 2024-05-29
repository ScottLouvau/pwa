use crate::{cluster_vector::ClusterVector, response::{Response, ResponseSet}, word::Word};
use std::collections::{HashMap, BinaryHeap};

const BEST_COUNT: usize = 20;

struct SearchState<'a> {
    answers: &'a Vec<Word>,                                     // In: Wordle answers for which to rank the guesses
    ranker: fn(&HashMap<ResponseSet, Vec<Word>>) -> usize,      // In: Ranking function for guesses; lower is better

    cluster_cutoff: f64,                                        // State: Only call ranking function for guesses creating at least this many distinct clusters of answers
    cluster_cutoff_ratio: f64,                                  // In: Raise cluster_cutoff to this percentage of the cluster count, if higher, as the search progresses
    
    count_ranked: usize,                                        // Out: How many guesses had the ranking function called?

    guesses: Vec<Word>,                                         // State: Current Set of guesses, initial plus current ones being considered
    responses: Vec<ResponseSet>,                                // State: The ResponseSet for each answer for the guesses so far, to avoid re-computing Response::score
    used_letters: u32,                                          // State: Bitmask of letters used in the guesses so far
    count_left: usize,                                          // State: Number of guesses left to add
    original_count: usize,                                      // In: Number of guesses to find

    best: BinaryHeap<(usize, Vec<Word>, ClusterVector)>,        // Out: Top N guess groups found so far; (score; guesses; cluster vector)
}

/// Search for the best guess(es) for a given set of answers, guess options, and initial guesses, according to a specific ranking function.
pub fn find_best(
    answers: &Vec<Word>,
    valid: &Vec<Word>,
    initial_guesses: Vec<Word>,
    count: usize,
    ranker: fn(&HashMap<ResponseSet, Vec<Word>>) -> usize,
    cluster_cutoff: f64,
    cluster_cutoff_ratio: f64,
) -> BinaryHeap<(usize, Vec<Word>, ClusterVector)> {

    // Exclude rare letters and those already guessed
    let mut used_letters = 0u32;
    for guess in initial_guesses.iter() {
        used_letters |= guess.letters_in_word();
    }

    if initial_guesses.len() < 2 {
        used_letters |= Word::new("vzxqj").unwrap().letters_in_word();
    }

    // Build a guess options list
    let mut options = Vec::new();

    // Remove options with used or repeat letters
    if count > 1 || initial_guesses.len() <= 2 {
        for option in valid.iter() {
            if option.letters_in_word() & used_letters != 0 {
                continue;
            }
            if option.has_repeat_letters() {
                continue;
            }

            options.push(*option);
        }
    } else {
        options = valid.clone();
    }

    // Build containers for a ResponseSet per answer
    let mut responses = Vec::new();
    for _ in 0..answers.len() {
        responses.push(ResponseSet::new());
    }

    // Add scores for initial guesses
    for guess in initial_guesses.iter() {
        for (i, answer) in answers.iter().enumerate() {
            let response = Response::score(*guess, *answer);
            responses[i].push(response);
        }
    }

    println!("Finding best {} guesses after {:?} having at least {:.0} clusters within {} / {} words with distinct letters...", count, initial_guesses, cluster_cutoff, options.len(), valid.len());

    // Build the state to search
    let mut state = SearchState {
        answers: answers,
        ranker: ranker,

        cluster_cutoff: cluster_cutoff,
        cluster_cutoff_ratio: cluster_cutoff_ratio,
        count_ranked: 0,

        guesses: initial_guesses,
        responses: responses,
        used_letters: used_letters,
        count_left: count,
        original_count: count,

        best: BinaryHeap::new(),
    };

    // Look recursively for the remaining guesses
    find_best_recurse(&mut state, &options);

    println!("Done. {} combinations scored.", state.count_ranked);
    state.best
}

fn find_best_recurse(state: &mut SearchState, guess_options: &Vec<Word>) {
    if state.count_left <= 1 {
        let mut cluster_sizes = HashMap::new();
        let mut clusters = HashMap::new();
        let mut cv = ClusterVector::new(Vec::new());

        for (_i, guess) in guess_options.iter().enumerate() {
            state.guesses.push(*guess);

            // Score guess against answers and determine how many distinct clusters there are
            cluster_sizes.clear();
            for (i, answer) in state.answers.iter().enumerate() {
                let responses = &mut state.responses[i];
                let response = Response::score(*guess, *answer);
                responses.push(response);

                *cluster_sizes.entry(*responses).or_insert(0usize) += 1;
            }

            // If the number of clusters is high enough, ...
            let cluster_count = cluster_sizes.len() as f64;
            if cluster_count >= state.cluster_cutoff {
                // Increase the cutoff, if this count is high enough
                let new_cutoff = cluster_count * state.cluster_cutoff_ratio;
                if new_cutoff > state.cluster_cutoff {
                    state.cluster_cutoff = new_cutoff;
                    println!("  CUTOFF -> {:.0}  ({:.0} x {:.2})", new_cutoff, cluster_count, state.cluster_cutoff_ratio);
                }

                // Build a map of the answers themselves
                clusters.clear();
                for (i, answer) in state.answers.iter().enumerate() {
                    let responses = &mut state.responses[i];
                    (*clusters.entry(*responses).or_insert(Vec::new())).push(*answer);
                }

                // Score the specific clusters
                state.count_ranked += 1;
                let score = (state.ranker)(&clusters);

                // If this ties or beats the current best, track it
                cv.clear();
                cv.add_map(&clusters);

                if state.best.len() < BEST_COUNT || state.best.peek().unwrap().0 > score {
                    if state.best.len() >= BEST_COUNT { state.best.pop(); }

                    println!("{}: {:?} {}", score, state.guesses, cv.to_string());
                    state.best.push((score, state.guesses.clone(), cv.clone()));
                }
            }

            state.guesses.pop();
            for response in state.responses.iter_mut() {
                response.pop();
            }
        }
    } else {
        let letters_before = state.used_letters;
        state.count_left -= 1;

        let mut inner_options = Vec::new();

        for (i, guess) in guess_options.iter().enumerate() {
            // Score this guess against all answers
            for (j, answer) in state.answers.iter().enumerate() {
                let response = Response::score(*guess, *answer);
                state.responses[j].push(response);
            }

            state.guesses.push(*guess);
            state.used_letters = letters_before | guess.letters_in_word();

            // Filter remaining guess options
            inner_options.clear();
            for other in guess_options[i+1..].iter() {
                let used_letters = other.letters_in_word();
                if used_letters & state.used_letters != 0 { continue; }

                inner_options.push(*other);
            }

            // Recurse to consider remaining options
            find_best_recurse(state, &inner_options);

            state.guesses.pop();
            for response in state.responses.iter_mut() {
                response.pop();
            }

            // Print progress and arguments to resume search here
            if state.count_left == state.original_count - 1 && i % 10 == 0 {
                println!(" --after {guess}  --cutoff {:.0}", state.cluster_cutoff);
            }
        }

        state.used_letters = letters_before;
        state.count_left += 1;
    }
}

pub fn score_cluster_count(cv: &Vec<usize>) -> usize {
    2500 - cv.iter().sum::<usize>()
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use crate::{word::Word, rank};

    #[test]
    fn search_scoring() {
        let answers = Word::parse_file(Path::new("../data/2315/answers.txt"));
        let guesses = vec![w("clint"), w("parse"), w("soare"), w("primy")];

        let results = super::find_best(&answers, &guesses, Vec::new(), 2, rank::total_turns_random_map, 0.0, 0.0);
        let best = results.iter().last().unwrap();
        assert_eq!(best.0, 3922);
        assert_eq!(best.1, vec![w("clint"), w("parse")]);
    }

    fn w(text: &str) -> Word {
        Word::new(text).unwrap()
    }
}