use std::{collections::BinaryHeap, str::Lines};
use crate::{cluster_vector::ClusterVector, parser::Parser, word::Word, *};

pub mod builders;
pub mod tree_player;

pub const LIST_ANSWERS_MAX_COUNT: usize = 16;

/* WordleTree describes a play strategy in Wordle.
    It can be a full decision tree (a specific next word in every situation),
    but it also allows describing a next guess to use in many situations and 
    marking "don't care" situations in which the player should randomly guess.

    WordleTree has a text syntax designed to be human readable and easy to parse,
    so that *both* humans and computers can easily play the strategy described.
 */
pub struct WordleTree {
    // The total number of turns estimated to solve every answer in this subtree, including initial guesses to get to this subtree.
    pub outer_total_turns: f64,

    // Describes the situation(s) for this subtree - a specific cluster identified by the alphabetically first word, all clusters of a length, all clusters under a length, or all clusters not handled by a sibling of this node.
    pub identifier: WordleTreeIdentifier,

    // How many answers are left under this subtree.
    pub answer_count: usize,

    // The next guess to use in this situation.
    pub next_guess: WordleGuess,

    // The specific answers in this subtree. Included so that the specific words here can be output in the text form.
    pub answers: Option<Vec<Word>>,

    // The cluster vector after next_guess, for large trees.
    pub cluster_vector: Option<ClusterVector>,

    // Further strategy to handle situations after next_guess.
    // Never populated if this subtree is two answers or less. Not needed if all subtrees will be guessed randomly.
    pub subtree: Option<Vec<WordleTree>>
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum WordleGuess {
    Specific(Word),
    Random
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum WordleTreeIdentifier {
    Any,                            // Applies to all games at the given turn
    EqualsLength(usize),            // Applies when the number of remaining possible answers matches (usize)
    Cluster(Word)                   // Applies when the alphabetically first remaining possible answer is (Word)
}

impl WordleTreeIdentifier {
    pub fn is_cluster(&self) -> bool {
        match self {
            WordleTreeIdentifier::Cluster(_) => true,
            _ => false
        }
    }

    /// Return whether this identifier is a match for the given cluster.
    pub fn matches(&self, cluster: &Vec<Word>) -> bool {
        match self {
            WordleTreeIdentifier::Any => true,
            WordleTreeIdentifier::EqualsLength(length) => cluster.len() == *length,
            WordleTreeIdentifier::Cluster(cluster_word) => cluster.first().map(|w| w == cluster_word).unwrap_or(false)
        }
    }

    /// Cluster > EqualsLength > Any; same as sort order.
    pub fn is_more_specific(&self, other: &WordleTreeIdentifier) -> bool {
        self > other
    }
}

pub struct WordleTreeToStringOptions {
    pub show_average_turns: bool,
    pub show_cluster_vectors: bool,
    pub show_answers: bool,
    pub always_show_identifiers: bool,

    pub show_zero_turn_paths: bool,
}

impl WordleTreeToStringOptions {
    pub fn default() -> WordleTreeToStringOptions {
        WordleTreeToStringOptions { 
            show_average_turns: false, 
            show_cluster_vectors: true, 
            show_answers: true, 
            always_show_identifiers: false,
            show_zero_turn_paths: false
        }
    }
}

impl WordleTree {
    pub fn new(identifier: WordleTreeIdentifier, next_guess: WordleGuess) -> WordleTree {
        WordleTree { 
            outer_total_turns: 0.0, 
            identifier: identifier, 
            answer_count: 0, 
            next_guess: next_guess, 
            answers: None, 
            cluster_vector: None,
            subtree: None
        }
    }

    pub fn new_single_leaf(word: Word, turns_before: usize) -> WordleTree {
        WordleTree {
            outer_total_turns: (turns_before + 1) as f64,
            identifier: WordleTreeIdentifier::Cluster(word),
            answer_count: 1,
            next_guess: WordleGuess::Random,
            answers: Some(vec![word]),
            cluster_vector: None,
            subtree: None
        }
    }

    pub fn new_leaf(answers: Vec<Word>, total_turns: f64) -> WordleTree {
        WordleTree {
            outer_total_turns: total_turns,
            identifier: WordleTreeIdentifier::Cluster(*answers.first().unwrap()),
            answer_count: answers.len(),
            next_guess: WordleGuess::Random,
            answers: Some(answers),
            cluster_vector: None,
            subtree: None
        }
    }

    pub fn new_sentinel() -> WordleTree {
        WordleTree {
            outer_total_turns: 0.0,
            identifier: WordleTreeIdentifier::Any,
            answer_count: 0,
            next_guess: WordleGuess::Random,
            answers: None,
            cluster_vector: None,
            subtree: None
        }
    }

    pub fn has_children(&self) -> bool {
        if let Some(subtree) = &self.subtree {
            return !subtree.is_empty();
        } else {
            return false;
        }
    }

    pub fn add_child(&mut self, child: WordleTree) {
        self.outer_total_turns += child.outer_total_turns;
        self.answer_count += child.answer_count;

        self.add_child_without_rollup(child);
    }

    pub fn add_child_without_rollup(&mut self, child: WordleTree) {
        if self.subtree.is_none() {
            self.subtree = Some(Vec::new());
        }

        let children = self.subtree.as_mut().unwrap();
        children.push(child);
    }

    pub fn add_answers(&mut self, cluster: &Vec<Word>) {
        if self.answers.is_none() {
            self.answers = Some(Vec::new());
        }

        self.answer_count += cluster.len();
        if let Some(answers) = &mut self.answers {
            for answer in cluster {
                answers.push(answer.clone());
            }
        }
    }

    pub fn take_first_child(&mut self) -> Option<WordleTree> {
        if let Some(subtree) = &mut self.subtree {
            return subtree.pop();
        } else {
            return None;
        }
    }

    pub fn to_string(&self) -> String {
        let mut result = String::new();
        self.add_to_string(&WordleTreeToStringOptions::default(), 0, &mut result);
        result
    }

    pub fn add_to_string(&self, options: &WordleTreeToStringOptions, indent: usize, result: &mut String) {
        // Add this node to the output string
        self.add_self_to_string(options, indent, result);

        // Further indented subtree, if included, 
        // *ordered by* : answer count desc, then identifier asc, then next_guess asc
        if let Some(subtree) = &self.subtree {
            let mut ordered = subtree.iter().collect::<BinaryHeap<_>>();
            while let Some(child) = ordered.pop() {
                child.add_to_string(options, indent + 1, result);
            }
        }
    }

    pub fn add_self_to_string(&self, options: &WordleTreeToStringOptions, indent: usize, result: &mut String) {
        // Indent
        let line_start = result.len();
        pad_to_length(result.len() + 4 * indent, result);
        let start_length = result.len();

        // Total or Average Turns
        if self.outer_total_turns != 0.0 {
            result.push_str(&write_turns(self.outer_total_turns, self.answer_count as f64, options.show_average_turns));

            // Align at 5
            pad_to_length(start_length + 5, result);
        }

        // Identifier: Show unless this is a specific, small cluster with a random guess
        let skip_identifier = self.identifier.is_cluster() && self.next_guess == WordleGuess::Random && self.answer_count <= LIST_ANSWERS_MAX_COUNT && options.always_show_identifiers == false;
        if skip_identifier == false {
            if result.len() > start_length { result.push(' '); }

            // Subtree (identifier, answer_count)
            match self.identifier {
                WordleTreeIdentifier::Any => {
                    result.push_str(&format!("(*, {})", self.answer_count));
                },
                WordleTreeIdentifier::Cluster(ref word) => {
                    result.push_str(&format!("({}, {})", word.to_string(), self.answer_count));
                },
                WordleTreeIdentifier::EqualsLength(length) => {
                    result.push_str(&format!("(= {}, {})", length, self.answer_count));
                }
            }

            // Align next guesses (follow indent of tree)
            pad_to_length(start_length + 17, result);

            // -> next_guess
            result.push_str(&" -> ");
            match self.next_guess {
                WordleGuess::Random => {
                    result.push_str("*");
                },
                WordleGuess::Specific(ref word) => {
                    result.push_str(&format!("{}", word.to_string()));
                }
            }
        }

        // {cluster_vector} => cluster_count (if included)
        if options.show_cluster_vectors {
            if let Some(cluster_vector) = &self.cluster_vector {
                let cv = cluster_vector.to_string();
                if cv.len() > 0 {
                    if result.len() > start_length { result.push_str("  "); }
                    result.push_str(&cv);
                }
            }
        }

        // [answers] (if small enough)
        if skip_identifier || options.show_answers {
            if let Some(answers) = &self.answers {
                if answers.len() > 0 && answers.len() <= LIST_ANSWERS_MAX_COUNT {
                    if skip_identifier == false {
                        pad_to_length(line_start + 64, result);
                    }

                    if result.len() > start_length { result.push(' '); }
                    result.push_str("{");

                    let mut sorted_answers = answers.clone();
                    sorted_answers.sort();

                    for (i, answer) in sorted_answers.iter().enumerate() {
                        if i > 0 { result.push_str(", "); }
                        result.push_str(&answer.to_string());
                    }
                    result.push('}');
                }
            }
        }

        result.push('\n');
    }


    /// Parse a full WordleTree from text
    pub fn parse(text: Lines) -> Result<WordleTree, String> {
        let mut parser = Parser::new(text);
        let mut parent_stack: Vec<(usize, WordleTree)> = Vec::new();
        
        loop {
            let (indent, tree) = WordleTree::parse_single(&mut parser)?;

            // Pop and add nodes which don't have a smaller indent than this node
            let mut last = None;
            while let Some((p_indent, mut parent)) = parent_stack.pop() {
                if let Some((_, leaf)) = last {
                    parent.add_child_without_rollup(leaf);
                }
    
                last = Some((p_indent, parent));
                if p_indent < indent { break; }
            }

            // Put the last removed node back
            if let Some((p_indent, leaf)) = last {
                parent_stack.push((p_indent, leaf));
            }
            
            // Add the newly parsed node
            parent_stack.push((indent, tree));

            // If this is the last line, break
            if parser.next_line().is_err() { break; }
        }

        // Pop and add nodes which don't have a smaller indent than this node
        let mut last = None;
        while let Some((_, mut parent)) = parent_stack.pop() {
            if let Some(leaf) = last {
                parent.add_child_without_rollup(leaf);
            }

            last = Some(parent);
        }

        Ok(last.unwrap())
    }

    /// Parse a single WordleTree node from the current line of text being parsed
    pub fn parse_single(parser: &mut Parser) -> Result<(usize, WordleTree), String> {
        // ex: 43 (foyer, 8) -> rover  [2, 1, 1 .. +1 [8] ∑14]  {foyer, hover, joker, offer, roger, rover, rower, wooer}
        let mut result = WordleTree::new_sentinel();
        let indent = parser.char_in_line - 1;

        // Outer Total Turns?
        if let Ok(outer_total_turns) = parser.as_f64() {
            result.outer_total_turns = outer_total_turns;
            parser.next()?;
        }
        
        // Identifier? (foyer, 8) | (*, 2315) | (= 1, 12)
        let mut had_identifier = false;
        if parser.current == "(" {
            parser.next()?;
            had_identifier = true;

            // Identifier: Equals Length, Word, or Any
            if parser.current == "=" {
                parser.next()?;
                result.identifier = WordleTreeIdentifier::EqualsLength(parser.as_usize()?);
            } else if let Some(word) = parser.as_word()? {
                result.identifier = WordleTreeIdentifier::Cluster(word);
            } else {
                result.identifier = WordleTreeIdentifier::Any;
            }

            parser.next()?;
            parser.require(",")?;

            // Answer Count
            result.answer_count = parser.as_usize()?;
            parser.next()?;
            parser.require(")")?;

            // Next Guess
            parser.require("->")?;

            if let Some(word) = parser.as_word()? { 
                result.next_guess = WordleGuess::Specific(word);
            } else {
                result.next_guess = WordleGuess::Random;
            }
            
            parser.next()?;
        }

        // Cluster Vector?
        if parser.current == "[" {
            result.cluster_vector = ClusterVector::parse(parser)?;
        }
        
        // [answers]
        if parser.current == "{" {
            parser.next()?;

            let mut answers = Vec::new();

            while parser.current != "}" {
                if let Some(word) = parser.as_word()? {
                    answers.push(word);
                } else {
                    return Err(parser.error("Specific words required in '{answers}'"));
                }

                parser.next()?;

                if parser.current != "}" {
                    parser.require(",")?;
                }
            }

            // If there wasn't an identifier, fill in from answers
            if had_identifier == false {
                if let Some(word) = answers.first() {
                    result.identifier = WordleTreeIdentifier::Cluster(*word);
                }
            }

            result.answer_count = answers.len();
            result.answers = Some(answers);

            parser.next()?;
        }
        
        // Verify nothing else is on this line
        if parser.next().is_err() {
            Ok((indent, result))
        } else {
            Err(parser.error("Unexpected content on line after end of WordleTree"))
        }
    }

}

impl PartialEq for WordleTree {
    fn eq(&self, other: &Self) -> bool {
        self.answer_count == other.answer_count 
            && self.identifier == other.identifier 
            && self.next_guess == other.next_guess
    }
}

impl Eq for WordleTree {}

impl PartialOrd for WordleTree {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // Sort by answer count descending, then guess (all random before specific), then cluster identifier (first word)
        Some(
            self.answer_count.cmp(&other.answer_count)
                .then_with(|| other.next_guess.cmp(&self.next_guess))
                .then_with(|| other.identifier.cmp(&self.identifier))
                
        )
    }
}

impl Ord for WordleTree {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

// impl std::fmt::Debug for WordleTree {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.write_str(&self.to_string())
//     }
// }

// impl std::fmt::Display for WordleTree {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.write_str(&self.to_string())
//     }
// }

#[cfg(test)]
mod tests {
    use crate::*;
    use super::*;

    #[test]
    fn test_to_string() {
        // Verify simple leaf nodes are single-line, with identifier omitted
        //  For any two-cluster, guessing either word takes 1 + 2 turns (1 if the guess was right, 2 otherwise)
        let tree = WordleTree::new_leaf(vec![w("parka"), w("parry")], 3.0);
        assert_eq!(smart_trim(&tree.to_string()), "3 {parka, parry}");

        // Verify total turns show tenths when it's not whole
        //  For a 3-cluster, next guesses can take (1 + 2 + 2) turns or (1 + 2 + 3) turns depending on whether the guess distinguishes the other two if wrong
        // 2/3 * (1 + 2 + 2) + 1/3 * (1 + 2 + 3) = 5.3333
        let tree = WordleTree::new_leaf(vec![w("fatal"), w("tally"), w("waltz")], (2.0 / 3.0) * (1.0 + 2.0 + 2.0) + (1.0 / 3.0) * (1.0 + 2.0 + 3.0));
        assert_eq!(smart_trim(&tree.to_string()), "5.3 {fatal, tally, waltz}");

        // Verify guess is shown when there is one
        //  Show a 3-cluster where a specific next guess is specified to save 1/3 of a turn.
        let mut tree = WordleTree::new_leaf(vec![w("fatal"), w("tally"), w("waltz")], 5.0);
        tree.identifier = WordleTreeIdentifier::Cluster(w("fatal"));
        tree.next_guess = WordleGuess::Specific(w("tally"));
        assert_eq!(smart_trim(&tree.to_string()), "5 (fatal, 3) -> tally {fatal, tally, waltz}");

        // Verify subtree shown when included
        let mut tree = WordleTree::new(WordleTreeIdentifier::Cluster(w("hardy")), WordleGuess::Specific(w("harry")));
        tree.add_child(WordleTree::new_leaf(vec![w("karma"), w("hardy"), w("marry")], 6.0));
        tree.add_child(WordleTree::new_leaf(vec![w("harry")], 1.0));
        assert_eq!(smart_trim(&tree.to_string()), 
"7 (hardy, 4) -> harry
    6 {hardy, karma, marry}
    1 {harry}");

        // Verify cluster vector shown when included
        let mut tree = WordleTree::new(WordleTreeIdentifier::Any, WordleGuess::Specific(w("parse")));
        tree.cluster_vector = Some(ClusterVector::new(vec![36, 15, 17, 11, 3, 7, 3, 5, 3, 3, 3, 1]));
        tree.answer_count = 2315;
        tree.outer_total_turns = 8552.0;
        assert_eq!(smart_trim(&tree.to_string()), format!("8552 (*, 2315) -> parse {}", tree.cluster_vector.unwrap().to_string()));

        // *No extra spaces* after next_guess if CV or answers are present but empty
        let mut tree = WordleTree::new(WordleTreeIdentifier::Cluster(w("parse")), WordleGuess::Specific(w("parse")));
        tree.outer_total_turns = 5.0;
        tree.answer_count = 1;
        tree.cluster_vector = Some(ClusterVector::new(Vec::new()));
        tree.answers = Some(Vec::new());

        // (verify text ends with a newline, and when trimmed nothing but the newline is removed)
        let text = tree.to_string();
        assert!(text.ends_with('\n'));
        assert_eq!(text.trim_end(), &text[0..text.len()-1]);
    }

    #[test]
    fn sort() {
        // Random guesses before any specific ones
        assert!(WordleGuess::Random > WordleGuess::Specific(w("waltz")));

        let mut left = WordleTree::new(WordleTreeIdentifier::Any, WordleGuess::Random);
        let mut right = WordleTree::new(WordleTreeIdentifier::Any, WordleGuess::Random);

        // Bigger before smaller (after in sort order because they're popped off a BinaryHeap)
        left.answer_count = 1;
        right.answer_count = 2;
        assert!(right > left);
        right.answer_count = 1;

        // Specific guesses before random
        right.next_guess = WordleGuess::Specific(w("waltz"));
        assert!(right > left);
        right.next_guess = WordleGuess::Random;

        // Equals Length before a single, even if only one in equals length 
        left = WordleTree::new(WordleTreeIdentifier::EqualsLength(1), WordleGuess::Specific(w("zooey")));
        right = WordleTree::new_single_leaf(w("zoomy"), 4);
        assert!(right > left);
    }

    #[test]
    fn test_identifier_all() {
        // Sort
        // Any < Length
        assert!(WordleTreeIdentifier::Any < WordleTreeIdentifier::EqualsLength(1));

        // Length < Cluster
        assert!(WordleTreeIdentifier::EqualsLength(1) < WordleTreeIdentifier::Cluster(w("fatal")));

        // Clusters sorted by first word
        assert!(WordleTreeIdentifier::Cluster(w("fatal")) < WordleTreeIdentifier::Cluster(w("waltz")));

        // is_cluster()
        assert_eq!(WordleTreeIdentifier::Any.is_cluster(), false);
        assert_eq!(WordleTreeIdentifier::EqualsLength(2).is_cluster(), false);
        assert_eq!(WordleTreeIdentifier::Cluster(w("match")).is_cluster(), true);

        // is_most_specific: Cluster > EqualsLength > Any. Ties are "don't care".
        assert_eq!(WordleTreeIdentifier::Any.is_more_specific(&WordleTreeIdentifier::EqualsLength(2)), false);
        assert_eq!(WordleTreeIdentifier::Any.is_more_specific(&WordleTreeIdentifier::Cluster(w("match"))), false);
        assert_eq!(WordleTreeIdentifier::EqualsLength(2).is_more_specific(&WordleTreeIdentifier::Any), true);
        assert_eq!(WordleTreeIdentifier::EqualsLength(2).is_more_specific(&WordleTreeIdentifier::Cluster(w("match"))), false);
        assert_eq!(WordleTreeIdentifier::Cluster(w("match")).is_more_specific(&WordleTreeIdentifier::Any), true);
        assert_eq!(WordleTreeIdentifier::Cluster(w("match")).is_more_specific(&WordleTreeIdentifier::EqualsLength(10)), true);

        // matches()
        let cluster = vec![w("fatal"), w("tally"), w("waltz")];

        // Any matches any cluster
        assert!(WordleTreeIdentifier::Any.matches(&cluster));

        // EqualsLength matches clusters of the same length
        assert!(WordleTreeIdentifier::EqualsLength(3).matches(&cluster));
        assert!(!WordleTreeIdentifier::EqualsLength(2).matches(&cluster));

        // Cluster matches only the cluster with the same first word
        assert!(WordleTreeIdentifier::Cluster(w("fatal")).matches(&cluster));
        assert!(!WordleTreeIdentifier::Cluster(w("tally")).matches(&cluster));
    }

    #[test]
    fn test_parsing() {
        // Small cluster with everything. Verify complete cluster vector loaded. Verify answer count from answers supercedes from identifier.
        let mut parser = Parser::new("    4 (fried, 4) -> fries  [10, 4]  {fried, fries, frisk}".lines());
        let (indent, tree) = WordleTree::parse_single(&mut parser).unwrap();
        assert_eq!(indent, 4);
        assert_eq!(tree.outer_total_turns, 4.0);
        assert_eq!(tree.identifier, WordleTreeIdentifier::Cluster(w("fried")));
        assert_eq!(tree.answer_count, 3);
        assert_eq!(tree.next_guess, WordleGuess::Specific(w("fries")));
        assert_eq!(tree.cluster_vector, Some(ClusterVector::new(vec![10, 4])));
        assert_eq!(tree.answers, Some(vec![w("fried"), w("fries"), w("frisk")]));

        // Implicit identifier
        let expected = WordleTree::new_leaf(vec![w("fried"), w("fries"), w("frisk")], 15.0);
        let mut parser = Parser::new("        15 {fried, fries, frisk}".lines());
        let (indent, tree) = WordleTree::parse_single(&mut parser).unwrap();
        assert_eq!(indent, 8);
        assert_eq!(tree.outer_total_turns, expected.outer_total_turns);
        assert_eq!(tree.identifier, expected.identifier);
        assert_eq!(tree.answer_count, expected.answer_count);
        assert_eq!(tree.next_guess, expected.next_guess);
        assert_eq!(tree.cluster_vector, expected.cluster_vector);
        assert_eq!(tree.answers, expected.answers);

        // Any cluster, summarized cluster vector. Verify cluster vector left out when summarized in text
        let mut parser = Parser::new("        47.1 (*, 6) -> *  [1, 2, 3, 4, 5 .. +64 ^128 ∑79] ".lines());
        let (indent, tree) = WordleTree::parse_single(&mut parser).unwrap();
        assert_eq!(indent, 8);
        assert_eq!(tree.outer_total_turns, 47.1);
        assert_eq!(tree.identifier, WordleTreeIdentifier::Any);
        assert_eq!(tree.answer_count, 6);
        assert_eq!(tree.next_guess, WordleGuess::Random);
        assert_eq!(tree.cluster_vector, None);
        assert_eq!(tree.answers, None);

        // Equals Length cluster, no CV, no answers
        let mut parser = Parser::new("        48 (= 1, 12) -> *".lines());
        let (indent, tree) = WordleTree::parse_single(&mut parser).unwrap();
        assert_eq!(indent, 8);
        assert_eq!(tree.outer_total_turns, 48.0);
        assert_eq!(tree.identifier, WordleTreeIdentifier::EqualsLength(1));
        assert_eq!(tree.answer_count, 12);
        assert_eq!(tree.next_guess, WordleGuess::Random);
        assert_eq!(tree.cluster_vector, None);
        assert_eq!(tree.answers, None);
    }
}