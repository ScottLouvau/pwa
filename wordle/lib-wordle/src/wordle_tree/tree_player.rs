use std::{collections::HashMap, mem};
use crate::{wordle_tree::*, word::Word, response::Response};

pub struct TreePlayer<'a> {
    tree: &'a WordleTree,
    current: Option<&'a WordleTree>,

    game_count: usize,
    last_turn: usize,
    turn_counts: HashMap<Vec<usize>, (usize, usize)>,
    path: Vec<usize>,
}

impl TreePlayer<'_> {
    pub fn new<'a>(tree: &'a WordleTree) -> TreePlayer<'a> {
        TreePlayer {
            tree,
            current: None,

            game_count: 0,
            last_turn: 0,
            turn_counts: HashMap::new(),
            path: Vec::new(),
        }
    }

    /// Identify the next guess for this strategy in the current situation, or None if random guesses should be used.
    pub fn choose(&mut self, guesses: &Vec<Word>, turn: usize, answers_left: &Vec<Word>) -> Option<Word> {
        self.next_for_game(guesses, turn, answers_left);

        if let Some(c) = self.current {
            match c.next_guess {
                WordleGuess::Specific(word) => Some(word),
                WordleGuess::Random => None
            }
        } else {
            None
        }
    }

    /// Before a turn, use the current situation (answers_left) to figure out which node in the tree applies to this game.
    fn next_for_game(&mut self, guesses: &Vec<Word>, turn: usize, answers_left: &Vec<Word>) {
        // If a new game has started, mark down total turns until win in the previous one
        if turn <= 1 {
            self.current = Some(&self.tree);

            if self.last_turn > 0 {
                self.score();
            }

            self.last_turn = 1;
            self.game_count += 1;

            return;
        }

        let last_guess = guesses.get(turn - 1).cloned();
        let mut last_response = None; 
        if let Some(guess) = last_guess {
            if let Some(first_answer) = answers_left.get(0) { 
                last_response = Some(Response::score(guess, *first_answer));
            }
        }


        if let Some(c) = self.current {
            // Look for the most specific matching child (Cluster > Length > Any)
            let mut best: Option<(usize, &WordleTree)> = None;

            if let Some(subtree) = &c.subtree {
                for (i, child) in subtree.iter().enumerate() {
                    if child.identifier.matches(last_guess, last_response, answers_left) {
                        let can_stop = child.identifier.is_cluster();

                        if let Some(b) = best {
                            if child.identifier.is_more_specific(&b.1.identifier) {
                                best = Some((i, child));
                            }
                        } else {
                            best = Some((i, child));
                        }

                        if can_stop { break; }
                    }
                }
            }

            if let Some(best) = best {
                self.path.push(best.0);
                self.current = Some(best.1);
            } else {
                self.current = None;
            }
        }

        self.last_turn = turn;
    }

    /// Clear collected statistics about games simulated with this TreePlayer
    pub fn clear_play_stats(&mut self) {
        self.game_count = 0;
        self.last_turn = 0;
        self.turn_counts.clear();
    }

    /// Find the cluster containing 'word' after all specific guesses (before random guessing)
    pub fn cluster(&mut self, word: Word, answers: &Vec<Word>, at_turn: usize) -> Vec<Word> {
        let mut turn = 1;
        let mut answers_left = answers.clone();

        while let Some(guess) = self.choose(&Vec::new(), turn, &answers_left) {
            let response = Response::score(guess, word);
            answers_left.retain(|a| Response::score(guess, *a) == response);
            turn += 1;
            if turn >= at_turn { break; }
        }

        answers_left
    }

    /// After each game, track total turns and games played for the last node reached.
    fn score(&mut self) {
        if self.last_turn > 0 {
            // Swap the path for a new empty vector
            let mut path = Vec::new();
            mem::swap(&mut path,&mut self.path);

            // Add turns for this game
            let counts = self.turn_counts.entry(path).or_default();
            counts.0 += self.last_turn;
            counts.1 += 1;
            
            // Ensure calls from to_string don't "flush" the last game repeatedly
            self.last_turn = 0;
        }
    }

    /// Recursively add up total turns under a given WordleTree node
    pub fn total_turns(&mut self, node: &WordleTree, path: &mut Vec<usize>) -> (usize, usize) {
        // Ensure the last game is scored
        self.score();

        // Compute rolled up total turns
        let mut outer_total_turns = (0, 0);
        if let Some(subtree) = &node.subtree {
            for (i, child) in subtree.iter().enumerate() {
                path.push(i);

                let inner_turns = self.total_turns(child, path);
                outer_total_turns.0 += inner_turns.0;
                outer_total_turns.1 += inner_turns.1;

                path.pop();
            }
        }

        if let Some(turns) = self.turn_counts.get(path) {
            outer_total_turns.0 += turns.0;
            outer_total_turns.1 += turns.1;
        }

        outer_total_turns
    }

    /// Write out the WordleTree with the turn counts observed with each node.
    pub fn to_string(&mut self, answer_count: usize, options: &WordleTreeToStringOptions) -> String {
        let mut result = String::new();
        let mut path = Vec::new();
        self.add_with_scores(&self.tree, &mut path, answer_count, options, &mut result);
        result
    }

    fn add_with_scores(&mut self, node: &WordleTree, path: &mut Vec<usize>, answer_count: usize, options: &WordleTreeToStringOptions, result: &mut String) {
        // Compute total turns through this node
        let outer_total_turns = self.total_turns(node, path);

        // Hide this subtree if unvisited
        if outer_total_turns.0 == 0 && options.show_zero_turn_paths == false {
            return;
        }

        // Write this node (no more indent)
        pad_to_length(result.len() + 4 * path.len(), result);
        let start = result.len();

        if options.show_average_turns {
            // Average Turns is total_turns / total_games under this node.
            result.push_str(&write_turns(outer_total_turns.0 as f64, outer_total_turns.1 as f64, options.show_average_turns));
        } else {
            // Total Turns is the total number of turns under this node per "cycle" through every possible answer.
            //  Total Turns are compared to the computed values, and those are based on the answer count in the root.
            //  So, use cycle_count = total_games / root.answer_count.
            //  If no root cycle count was provided (0), just show the total turns through during this simulation as a whole.
            let cycle_count = if self.tree.answer_count > 0 { self.game_count as f64 / self.tree.answer_count as f64 } else { 1.0 };
            let total_turns = outer_total_turns.0 as f64 / cycle_count;
            result.push_str(&write_turns(total_turns, 1.0, options.show_average_turns));
        }
        result.push(' ');
        if node.outer_total_turns > 0.0 {
            pad_to_length(start + 6, result);
        }

        node.add_self_to_string(options, 0, result);

        // Write subtree
        if let Some(subtree) = &node.subtree {
            for (i, child) in subtree.iter().enumerate() {
                path.push(i);
                self.add_with_scores(child, path, answer_count, options, result);
                path.pop();
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use crate::wordle_tree::*;
    use super::*;

    const SAMPLE_TREE: &str = 
"(*, 0) -> parse
    (*, 0) -> clint
        (= 3, 5) -> first
        (fatal, 3) -> tally
            (*, 2) -> waltz";

    #[test]
    fn player_all() {
        let tree = WordleTree::parse(SAMPLE_TREE.lines()).unwrap();
        assert_eq!(smart_trim(&tree.to_string()), smart_trim(SAMPLE_TREE));
        let mut player = TreePlayer::new(&tree);

        // Simulate a game
        let guesses = Vec::new();
        let answers_left = vec![w("fatal"), w("tally"), w("waltz")];
        assert_eq!(player.choose(&guesses, 1, &answers_left), Some(w("parse")));

        // parse; (*) -> clint
        assert_eq!(player.choose(&guesses, 2, &answers_left), Some(w("clint")));

        // clint; (fatal, 3) -> tally
        assert_eq!(player.choose(&guesses, 3, &answers_left), Some(w("tally")));

        // tally; (*) -> waltz
        assert_eq!(player.choose(&guesses, 4, &answers_left), Some(w("waltz")));

        // waltz has no further children; random from here
        assert_eq!(player.choose(&guesses, 5, &answers_left), None);
        assert_eq!(player.choose(&guesses, 6, &answers_left), None);


        // Simulate where we don't match the "fatal" path
        let answers_left = vec![w("other"), w("tally"), w("waltz")];
        assert_eq!(player.choose(&guesses, 1, &answers_left), Some(w("parse")));
        assert_eq!(player.choose(&guesses, 2, &answers_left), Some(w("clint")));
        assert_eq!(player.choose(&guesses, 3, &answers_left), Some(w("first")));
        assert_eq!(player.choose(&guesses, 4, &answers_left), None);

        // Ask for the score. Confirm 6 turns down the "waltz" path and 4 turns down the "first" path, so 10 turns on common ancestors.
        let scored_text = player.to_string(answers_left.len(), &WordleTreeToStringOptions::default());
        let expected = 
"10 (*, 0) -> parse
    10 (*, 0) -> clint
        4 (= 3, 5) -> first
        6 (fatal, 3) -> tally
            6 (*, 2) -> waltz";
        assert_eq!(smart_trim(&scored_text), smart_trim(expected));

        // Clear states
        player.clear_play_stats();

        // Simulate only the 'first' path game
        let answers_left = vec![w("other"), w("tally"), w("waltz")];
        assert_eq!(player.choose(&guesses, 1, &answers_left), Some(w("parse")));
        assert_eq!(player.choose(&guesses, 2, &answers_left), Some(w("clint")));
        assert_eq!(player.choose(&guesses, 3, &answers_left), Some(w("first")));
        assert_eq!(player.choose(&guesses, 4, &answers_left), None);

        // Verify that the tally -> waltz path is hidden
        let mut options = WordleTreeToStringOptions::default();
        options.show_zero_turn_paths = false;
        let scored_text = player.to_string(answers_left.len(), &options);
        let expected = 
"4 (*, 0) -> parse
    4 (*, 0) -> clint
        4 (= 3, 5) -> first";
        assert_eq!(smart_trim(&scored_text), smart_trim(expected));

        // Ask for clusters for TALLY - should be alone, because it goes down to a specific guess for itself
        let answers_left = vec![w("fatal"), w("tally"), w("waltz")];
        assert_eq!(player.cluster(w("tally"), &answers_left, 4), vec![w("tally")]);

        // Ask for the clusters for a quad - should guess PARSE, CLINT and then not match for either child
        let answers_left = vec![w("odder"), w("order"), w("ruder"), w("udder")];
        assert_eq!(player.cluster(w("odder"), &answers_left, 4), answers_left);
    }
}