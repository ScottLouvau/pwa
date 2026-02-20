use std::collections::HashMap;
use crate::{word::Word, rank, response::ResponseSet, clubs::Clubs, wordle_tree::{WordleTree, WordleGuess}, cluster_vector::ClusterVector};

const MIN_TOTAL_TURNS_IMPROVEMENT: f64 = 3.0;

// TODO:
//  - Show when specific guesses are needed after the third one.
//  - Savings from guessing after first standard?
//  - Outer Turns for a realistic simulation; guesses based on known letter count, not cluster count left.

// TODO: Take in a tree and figure out the next guess rather than assuming a standard one.

/// Evaluate specific guesses and emit a play strategy.
pub fn recommended_strategy(guesses: &Vec<Word>, next_guess: Word, answers: &Vec<Word>, valid: &Vec<Word>) -> String {
    let mut map = HashMap::new();
    rank::split_all(&answers, &guesses, ResponseSet::new(), &mut map);

    let mut clusters: Vec<(ResponseSet, Vec<Word>)> = map.into_iter().collect();
    clusters.sort_by(|l, r| r.1.len().cmp(&l.1.len()));

    let mut result = String::new();
    for (responses, cluster_words) in clusters.iter() {
        if cluster_words.len() < 4 { continue; }

        let first = cluster_words[0];
        let count = cluster_words.len();

        if count > 63 { 
            result += &format!("SKIP {first}, {count}\n");
            continue;
        }

        let next = Some(next_guess); // if responses.known_count() < 3 { Some(next_guess) } else { None };

        if let Some(choice) = run_best_turns(cluster_words, next, &valid) {
            result += &format!("{} ({first}, {count})\n    -> {choice}\n", responses.to_knowns(guesses));

            // for word in cluster_words {
            //     result += &format!("    {}\n", word);
            // }

            result += "\n";
        }
    }

    result
}

fn run_best_turns(words: &Vec<Word>, next_guess: Option<Word>, valid: &Vec<Word>) -> Option<String> {
    let clubs = Clubs::new(&words, valid);
    let within = clubs.all_vector();

    // Search for the best strategy (issue: best all the way down)
    let mut choices = HashMap::new();
    clubs.count_best_turns(clubs.all_vector(), &mut choices);
    let mut tree = WordleTree::new_sentinel();
    clubs.best_strategy(within, &choices, false, &mut tree);

    // If there was a best option, compare to the planned next option
    let next = tree.take_first_child();
    if let Some(next) = next {
        if let WordleGuess::Specific(guess) = next.next_guess {
            let cv = ClusterVector::from_bits(&clubs.split(guess, within));
            let outer_turns = clubs.count_random_turns_after(within, guess);

            if let Some(next_guess) = next_guess {
                let standard_outer_turns = clubs.count_random_turns_after(within, next_guess);
                let std_cv = ClusterVector::from_bits(&clubs.split(next_guess, within));

                if outer_turns <= standard_outer_turns - MIN_TOTAL_TURNS_IMPROVEMENT {
                    return Some(format!("{guess}  {outer_turns:2.1}  {cv}\n    vs {next_guess}  {standard_outer_turns:2.1}  {std_cv}"));
                } 
            } else {
                let standard_outer_turns = clubs.count_random_turns(within);
                
                if outer_turns <= standard_outer_turns - MIN_TOTAL_TURNS_IMPROVEMENT {
                    return Some(format!("{guess}  {outer_turns:2.1}  {cv}\n    vs *      {standard_outer_turns:2.1}"));
                }
            }
        }
    }
    
    //format!("*      {outer_turns:2}")
    None
}