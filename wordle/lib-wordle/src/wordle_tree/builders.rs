use std::{collections::{HashMap, VecDeque}, mem};
use crate::{word::Word, wordle_tree::{*, self}, rank, response::ResponseSet};

// FUTURE:
//  Currently, builders which split the tree (make different specific choices per situation) can't recurse by using the overall construction strategy; they don't get a handle to it.
//  The strategy would have to be a member in BuilderState to allow it to be passed itself.
//  I *think* Rust may be unwilling to do this because a closure that changes state is an FnMut, and must be mutable, and can't be borrowed mutably while it's being borrowed already (which would be the case recursively).

// Strategy Situations:
//  - Pass:       Take nothing off the map, don't create any new nodes, and return None to tell the next strategy to try.
//  - Summarize:  Create one (*, count) node for the remainder of the game. Compute the expected turns.
//  - Filter:     Remove *some* clusters from the map and add nodes for them. Return None if any clusters are left to have the next strategy finish.
//  - Branch:     Create multiple nodes and potentially recurse to finish the game. 

// TODO:
//  - guess_best_until_done should consider out-of-cluster options when appropriate.
//     - Only for 4+ clusters. Only when best in-cluster is more than two additional turns on average.
//     - May need to set maximum cluster size. Hundreds of cluster words would leave too much recursive rework.
//     - Sort valid by ideal turns; stop when remaining option ideal turns are worse than current best.
//     - Stop early when any valid option has fully split the remaining answers.
//     - DO NOT recurse when option only produces one cluster (infinite recursion).
//     - Put 'valid' on BuilderState to get it through.

//  - Print options on WordleTree? (Hide answers. Hide CVs. Total/Avg Turns)
//  - Triples: guess_safe_under_length only? Interesting but not playable unless the other clusters are all handled individually, which is guess_best.

// How would "assess" mode look on a tree like this?
//  -> Filter the tree to show only the paths taken for the answer.
//    -> Write the expected turns (the last ancestor total_turns / answer_count)
//    -> Can also create a tree for the actual guesses (need to know if standard, random, or specific)


/// Build constructs a WordleTree for a given strategy and set of answers and guesses.
///  It uses composable strategy parts to choose the next guess for each situation.
pub fn build(strategy: &str, answers: &Vec<Word>, guesses: &Vec<Word>) -> WordleTree {
    match strategy {
        "standard" => build_standard(answers, guesses),
        "hybrid"   => build_hybrid(answers, guesses),
        "best"     => build_best(answers, guesses),
        "first"    => build_first(answers, guesses),
        "v11"      => build_v11(answers, guesses),
        _          => panic!("Unknown strategy: {}", strategy)
    }
}

struct BuilderState {
    pub map: HashMap<ResponseSet, Vec<Word>>,
    pub turns_before: usize
}

impl BuilderState {
    pub fn new(answers: &Vec<Word>) -> BuilderState {
        let mut map: HashMap<ResponseSet, Vec<Word>> = HashMap::new();
        map.insert(ResponseSet::new(), answers.clone());

        BuilderState { map, turns_before: 0, }
    }
}

// ---- Assembled Strategy Chains ----

// - Guess SOARE, CLINT.
// - If fewer than three letters are known:
//   - guess DUMPY.
// - Otherwise, guess randomly in-cluster.
fn build_v11(answers: &Vec<Word>, _guesses: &Vec<Word>) -> WordleTree {
    let mut guesses = VecDeque::from(wv("soare, clint"));
    let third = w("dumpy");

    let mut strategy: &mut dyn FnMut(&mut BuilderState, &mut WordleTree) -> Option<WordleTree> = &mut |mut s, mut p| 
        guess_next_standard(&mut s, &mut p, &mut guesses)
            .or_else(|| guess_specific_under_letter_count(&mut s, &mut p, third, 3))
            .or_else(|| guess_random_up_to_length(&mut s, &mut p, 2))
            .or_else(|| guess_random_all_merged(&mut s, &mut p));

    build_tree(answers, &mut strategy)
}

// Play by guessing all provided guesses and then random in-cluster choices.
fn build_standard(answers: &Vec<Word>, guesses: &Vec<Word>) -> WordleTree {
    let mut guesses = VecDeque::from(guesses.clone());

    let mut strategy: &mut dyn FnMut(&mut BuilderState, &mut WordleTree) -> Option<WordleTree> = &mut |mut s, mut p| 
        guess_next_standard(&mut s, &mut p, &mut guesses)
            .or_else(|| guess_random_up_to_length(&mut s, &mut p, 2))
            .or_else(|| guess_random_all_merged(&mut s, &mut p));

    build_tree(answers, &mut strategy)
}

// Play by guessing < 4, otherwise standard. Break out random other guesses to show clusters left.
fn build_hybrid(answers: &Vec<Word>, guesses: &Vec<Word>) -> WordleTree {
    let mut guesses = VecDeque::from(guesses.clone());

    let mut strategy: &mut dyn FnMut(&mut BuilderState, &mut WordleTree) -> Option<WordleTree> = &mut |mut s, mut p| 
        guess_random_up_to_length(&mut s, &mut p, 3)
            .or_else(|| guess_next_standard(&mut s, &mut p, &mut guesses))
            .or_else(|| guess_random_separate(&mut s, &mut p));

    build_tree(answers, &mut strategy)
}

// Play by guessing tiny clusters, or playing the next standard guess, or the in-cluster guess using the fewest total turns for across all answers.
fn build_best(answers: &Vec<Word>, guesses: &Vec<Word>) -> WordleTree {
    let mut guesses = VecDeque::from(guesses.clone());

    let mut strategy: &mut dyn FnMut(&mut BuilderState, &mut WordleTree) -> Option<WordleTree> = &mut |mut s, mut p| 
        guess_random_up_to_length(&mut s, &mut p, 3)
            .or_else(|| guess_next_standard(&mut s, &mut p, &mut guesses))
            .or_else(|| guess_best_until_done(&mut s, &mut p));

    build_tree(answers, &mut strategy)
}

// Play by guessing tiny clusters, or the next standard guesses, or the alphabetically first possible answer each time
fn build_first(answers: &Vec<Word>, guesses: &Vec<Word>) -> WordleTree {
    let mut guesses = VecDeque::from(guesses.clone());

    let mut strategy: &mut dyn FnMut(&mut BuilderState, &mut WordleTree) -> Option<WordleTree> = &mut |mut s, mut p| 
    guess_random_up_to_length(&mut s, &mut p, 3)
        .or_else(|| guess_next_standard(&mut s, &mut p, &mut guesses))
        .or_else(|| guess_first_until_done(&mut s, &mut p));

    build_tree(answers, &mut strategy)
}

// ---- Main Recursive builder function to turn a chain of strategy options into a built tree  ----

/// Build a WordleTree for a given set of answers, taking a closure which returns the next guess to try.
///  Pass an "or_else" chain of strategy options to describe a full strategy
///  ex: build_tree(&answers, &mut |s, p| next_standard(s, p, &mut guesses).or_else(|| random_guess_each(s, p))));
fn build_tree(answers: &Vec<Word>, next_guess: &mut dyn FnMut(&mut BuilderState, &mut WordleTree) -> Option<WordleTree>) -> WordleTree {
    let mut root = WordleTree::new_sentinel();
    let mut state = BuilderState::new(answers);

    if state.map.len() > 0 {
        build_tree_recurse(&mut state, &mut root, next_guess);
    }

    root.take_first_child().unwrap()
}

fn build_tree_recurse(state: &mut BuilderState, current: &mut WordleTree, next_guess: &mut dyn FnMut(&mut BuilderState, &mut WordleTree) -> Option<WordleTree>) {
    let mut next = next_guess(state, current).or_else(|| guess_random_all_merged(state, current)).unwrap();

    if state.map.len() > 0 {
        state.turns_before += 1;
        build_tree_recurse(state, &mut next, next_guess);
        state.turns_before -= 1;
    }

    current.add_child(next);
}

/// ---- Composable Strategies for building a WordleTree ----

/// For each cluster, choose the in-cluster guess which minimizes total turns. Recurse until all clusters are solved.
fn guess_best_until_done(state: &mut BuilderState, parent: &mut WordleTree) -> Option<WordleTree> {
    let mut last = None;

    // Guess all < 4; always better for 1-, 2-, never worse for 3-.
    //  May miss up to 0.67 turns for triples with one safe guess and two unsafe (1 + 2 + 2) vs (1 + 2 + 3)
    let node = guess_random_up_to_length(state, parent, 4);
    if let Some(node) = node { add_except_last(node, parent, &mut last); }

    let outer_map = mem::replace(&mut state.map, HashMap::new());
    let mut inner_map = HashMap::new();

    for (_, cluster) in outer_map.iter() {
        let identifier = WordleTreeIdentifier::Cluster(cluster[0]);

        let mut best: Option<WordleTree> = None;
        let mut worst_turns: Option<f64> = None;

        for word in cluster {
            let mut candidate = WordleTree::new(identifier, WordleGuess::Specific(word.clone()));

            inner_map.clear();
            let excluded_count = rank::split_as_set(cluster, *word, &mut inner_map);
            candidate.cluster_vector = Some(ClusterVector::from_map(&inner_map));

            mem::swap(&mut state.map, &mut inner_map);
            state.turns_before += 1;
            guess_best_until_done(state, &mut candidate).map(|c| candidate.add_child(c));
            state.turns_before -= 1;

            if excluded_count > 0 {
                candidate.add_child(WordleTree::new_single_leaf(*word, state.turns_before));
            }

            if let Some(w) = worst_turns {
                if w < candidate.outer_total_turns {
                    worst_turns = Some(candidate.outer_total_turns);
                }
            } else {
                worst_turns = Some(candidate.outer_total_turns);
            }

            if let Some(b) = &best {
                if b.outer_total_turns > candidate.outer_total_turns {
                    best = Some(candidate);
                }
            } else {
                best = Some(candidate);
            }
        }

        if let Some(mut best) = best {
            if let Some(worst_turns) = worst_turns {
                if best.outer_total_turns < worst_turns {
                    // Ensure node knows the cluster answers; correct count if doubled by add_answers
                    best.add_answers(cluster);
                    best.answer_count = cluster.len();

                    // Don't show subtree for 1- and 2- clusters
                    if best.answer_count <= 3 { 
                        best.subtree = None; 
                    } 
                    
                    // Don't show the subtree for "safe" clusters (all words within distinguished by guess)
                    if let Some(cv) = &best.cluster_vector {
                        if cv.biggest_cluster() <= 1 {
                            best.subtree = None;
                        }
                    }

                    add_except_last(best, parent, &mut last);
                } else {
                    let any_guess_leaf = WordleTree::new_leaf(cluster.clone(), worst_turns);
                    add_except_last(any_guess_leaf, parent, &mut last);
                }
            }
        }
    }

    Some(last.unwrap())
}

/// For each cluster, guess the first word in the cluster. Recurse until all clusters are solved.
fn guess_first_until_done(state: &mut BuilderState, parent: &mut WordleTree) -> Option<WordleTree> {
    let mut last = None;

    let map = mem::replace(&mut state.map, HashMap::new());
    for (_, cluster) in map.into_iter() {
        // Guess the first word in the cluster
        let guess = cluster[0];
        let mut node = WordleTree::new(WordleTreeIdentifier::Cluster(guess), WordleGuess::Specific(guess));

        // Add the subnode for the guess having been the answer
        if cluster.len() > 1 {
            let single = WordleTree::new_single_leaf(guess, state.turns_before);
            node.add_child(single);
        } else {
            node.next_guess = WordleGuess::Random;
            node.answer_count = 1;
            node.outer_total_turns = (state.turns_before + 1) as f64;
        }
        
        // Split remaining answers for this guess
        let mut inner_map = HashMap::new();
        rank::split_as_set(&cluster, guess, &mut inner_map);

        // Add CV and answers
        node.cluster_vector = Some(ClusterVector::from_map(&inner_map));
        if cluster.len() <= wordle_tree::LIST_ANSWERS_MAX_COUNT {
            node.answers = Some(cluster);
        }

        // Recurse for each sub-cluster
        mem::swap(&mut state.map, &mut inner_map);
        state.turns_before += 1;
        let last_child = guess_first_until_done(state, &mut node);
        state.turns_before -= 1;

        if let Some(last_child) = last_child {
            node.add_child(last_child);
        }

        // Add each node except the last one 
        add_except_last(node, parent, &mut last);
    }

    last
}

/// Guess the next standard guess, if any are left.
fn guess_next_standard(state: &mut BuilderState, _parent: &mut WordleTree, guesses: &mut VecDeque<Word>) -> Option<WordleTree> {
    guess_specific(state, &mut guesses.pop_front())
}

/// Guess a specific word next, if provided.
fn guess_specific(state: &mut BuilderState, specific: &mut Option<Word>) -> Option<WordleTree> {
    if let Some(guess) = specific.take() {
        // Make a node for this guess
        let mut current = WordleTree::new(WordleTreeIdentifier::Any, WordleGuess::Specific(guess));

        // Re-split answers for this guess
        let mut inner_map = HashMap::new();
        let excluded_count = rank::split_map(&state.map, guess, 0, &mut inner_map);
        current.cluster_vector = Some(ClusterVector::from_map(&inner_map));

        // Replace map for next guess for the new one
        mem::swap(&mut state.map, &mut inner_map);

        // If the guess itself was in the answers, create a single node for it
        if excluded_count > 0 {
            let single = WordleTree::new_leaf(vec![guess], (state.turns_before + 1) as f64);
            current.add_child(single);
        }

        Some(current)
    } else {
        None
    }
}

/// Guess a specific word next, if provided.
fn guess_specific_under_letter_count(state: &mut BuilderState, parent: &mut WordleTree, specific: Word, known_letters_under: u8) -> Option<WordleTree> {
    let mut last = None;

    // Clear and take the map
    let mut map = HashMap::new();
    mem::swap(&mut state.map, &mut map);

    let mut inner_map = HashMap::new();

    // Add the specific guess for each cluster under the target known letter count
    for (responses, cluster) in map.iter() {
        if responses.known_count() < known_letters_under {
            let cluster_word = cluster[0];
            let mut current = WordleTree::new(WordleTreeIdentifier::Cluster(cluster_word), WordleGuess::Specific(specific));
            current.answer_count = cluster.len();

            rank::split(cluster, specific, &mut inner_map);
            current.cluster_vector = Some(ClusterVector::from_map(&inner_map));

            let inner_turns_under_here = rank::total_turns_random_map(&inner_map) as f64;
            let outer_turns_here = (cluster.len() * (state.turns_before + 1)) as f64 + inner_turns_under_here;
            current.outer_total_turns = outer_turns_here;
            
            if let Some(previous) = last {
                parent.add_child(previous);
            }

            last = Some(current);
        }
    }

    // Put back the clusters we didn't use the guess for
    map.retain(|r, _| r.known_count() >= known_letters_under);
    mem::swap(&mut state.map, &mut map);

    if state.map.len() > 0 {
        // If there are words left, add the last child and request the next strategy option
        if let Some(last) = last {
            parent.add_child(last);
        }

        // Return None to tell the next strategy option to handle the remainder
        return None;
    } else {
        // Return one of the nodes to stop other strategy attempts
        last
    }
}

/// Guess each small cluster randomly. Merge them into a node for each "EqualsLength". Leave remaining clusters for the next strategy option.
fn guess_random_up_to_length(state: &mut BuilderState, parent: &mut WordleTree, length: usize) -> Option<WordleTree> {
    let mut nodes = Vec::new();
    for i in 0..length {
        let mut current = WordleTree::new(WordleTreeIdentifier::EqualsLength(i + 1), WordleGuess::Random);
        current.answers = Some(Vec::new());
        nodes.push(current);
    }

    // Add small clusters to the per-length leaves
    for (_, cluster) in state.map.iter() {
        let answer_count = cluster.len();
        if answer_count <= length {
            let target = &mut nodes[answer_count - 1];
            target.add_answers(cluster);

            let turns = (state.turns_before * answer_count) as f64 + rank::total_turns_random(cluster);            
            target.outer_total_turns += turns as f64;
        }
    }

    // Remove those clusters from the map
    state.map.retain(|_, v| v.len() > length);

    // Add each child but the last one
    let mut last = None;
    for current in nodes {
        if current.answer_count > 0 {
            add_except_last(current, parent, &mut last);
        }
    }

    if state.map.len() > 0 {
        // If there are words left, add the last child and request the next strategy option
        if let Some(last) = last {
            parent.add_child(last);
        }

        // Return None to tell the next strategy option to handle the remainder
        return None;
    } else {
        // Otherwise, return the last node to indicate that we're done
        return last;
    }
}

/// Guess every remaining cluster randomly. Create a node for each cluster to them and their per-cluster costs.
fn guess_random_separate(state: &mut BuilderState, parent: &mut WordleTree) -> Option<WordleTree> {
    // Summarize singles and pairs
    let mut last = guess_random_up_to_length(state, parent, 2);

    // Clear and take the map
    let mut map = HashMap::new();
    mem::swap(&mut state.map, &mut map);

    // Add a 'random leaf' for each cluster
    for (_, cluster) in map {
        let inner_turns_under_here = rank::total_turns_random(&cluster);
        let outer_turns_here = (cluster.len() * state.turns_before) as f64 + inner_turns_under_here;

        let current = WordleTree::new_leaf(cluster, outer_turns_here);
        if let Some(previous) = last {
            parent.add_child(previous);
        }

        last = Some(current);
    }

    // Return one of the nodes to stop other strategy attempts
    last
}

/// Guess all remaining words randomly, represented with a single node with the total guess turn cost.
fn guess_random_all_merged(state: &mut BuilderState, _parent: &mut WordleTree) -> Option<WordleTree> {
    // Add a single leaf to summarize all remaining answers
    let mut current = WordleTree::new(WordleTreeIdentifier::Any, WordleGuess::Random);
    let answer_count = state.map.iter().map(|(_, v)| v.len()).sum::<usize>();
    current.answer_count = answer_count;

    // Compute total turns with random guessing across all answers
    let inner_turns_under_here = rank::total_turns_random_map_exact(&state.map);
    let outer_turns_here = (answer_count * state.turns_before) as f64 + inner_turns_under_here;
    current.outer_total_turns = outer_turns_here;

    // Include the answers themselves if there are few enough
    if answer_count <= wordle_tree::LIST_ANSWERS_MAX_COUNT {
        let answers: Vec<Word> = state.map.iter().flat_map(|(_, v)| v).map(|w| w.clone()).collect();
        current.answers = Some(answers);
    }

    // Clear the map
    state.map.clear();

    Some(current)
}

/// Helper to add a node under parent in the Wordle tree but save a referece to the last one to return.
///  It's weird, but it's how each chained strategy piece communicates whether it has handled everything or not.
fn add_except_last(node: WordleTree, parent: &mut WordleTree, last: &mut Option<WordleTree>) {
    if let Some(l) = last.take() {
        parent.add_child(l);
    }

    *last = Some(node);
}

#[cfg(test)]
mod tests {
    use crate::smart_trim;
    use super::*;

    #[test]
    fn test_build_standard() {
        let answers = vec![w("fatal"), w("tally"), w("waltz")];
        let guesses = vec![w("parse"), w("fatal")];

        // Should guess PARSE, FATAL before anything.
        //  - FATAL solved in two guesses.
        //  - TALLY, WALTZ left which take 3 + 4 guesses. (7 total)
        let tree = build_standard(&answers, &guesses);
        assert_eq!(smart_trim(&tree.to_string()), 
"9 (*, 3) -> parse [0, 0, 1]
    9 (*, 3) -> fatal [0, 1]
        7 (= 2, 2) -> * {tally, waltz}
        2 {fatal}");

        // Should guess PARSE, WALTZ before anything.
        //  - WALTZ solved in two guesses.
        //  - FATAL, TALLY are singles which take 3 + 3 guesses (6 total)
        let guesses = vec![w("parse"), w("waltz")]; 
        let tree = build_standard(&answers, &guesses);
        assert_eq!(smart_trim(&tree.to_string()), 
"8 (*, 3) -> parse [0, 0, 1]
    8 (*, 3) -> waltz [2]
        6 (= 1, 2) -> * {fatal, tally}
        2 {waltz}");
    }

    fn w(text: &str) -> Word {
        Word::new(text).unwrap()
    }
}