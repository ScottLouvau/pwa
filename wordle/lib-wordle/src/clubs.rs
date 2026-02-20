use std::collections::HashMap;
use crate::{bit_vector_slice::BitVectorSlice, word::Word, response::{Response, self}, cluster_vector::ClusterVector, wordle_tree::{WordleTree, WordleTreeIdentifier, WordleGuess, self}};

pub struct Clubs<'a> {
    letters: [LetterClubs; 26],
    answers: &'a Vec<Word>,
    answer_count: usize,

    valid: &'a Vec<Word>
}

#[derive(Clone, Copy)]
struct LetterClubs {
    pub any: BitVectorSlice,
    pub pos: [BitVectorSlice; 5],
}

impl LetterClubs {
    pub fn new() -> LetterClubs {
        LetterClubs {
            any: BitVectorSlice::new(),
            pos: [BitVectorSlice::new(); 5],
        }
    }
}

/// Container for state during best_next searches
struct BestConsiderState<'a> {
    pub within: BitVectorSlice, 

    pub best: (Word, usize),
    pub was_worse: bool,
    pub ideal_turns: usize,

    pub clubs: &'a Clubs<'a>,
    pub choices: &'a mut HashMap<BitVectorSlice, (Word, usize)>
}

impl BestConsiderState<'_> {
    fn new<'a>(within: BitVectorSlice, clubs: &'a Clubs, choices: &'a mut HashMap<BitVectorSlice, (Word, usize)>) -> BestConsiderState<'a> {
        // The initial best is the first word; find turns for it
        let first_word = clubs.answers[within.iter().next().unwrap()];
        let first_turns = clubs.count_best_turns_after(within, first_word, choices);

        BestConsiderState {
            within: within, 

            best: (first_word, first_turns),
            was_worse: false,
            ideal_turns: ((2 * within.count()) - 1) as usize,

            clubs: clubs,
            choices: choices
        }
    }

    fn consider(&mut self, guess: Word) {
        // Are ideal turns better than the best we've seen?
        let guess_ideal_turns = self.clubs.count_ideal_turns(self.within, guess);

        // If not, skip finding ideal turns for this guess
        if guess_ideal_turns >= self.best.1 {
            if guess_ideal_turns > self.best.1 { self.was_worse = true; }
            return; 
        }

        // If this could be better, count actual turns
        let best_turns = self.clubs.count_best_turns_after(self.within, guess, self.choices);

        // If this is a new best, record it
        if best_turns < self.best.1 {
            self.best = (guess, best_turns);
            self.was_worse = true;
        }
    }

    fn stop_searching(&self) -> bool {
        // Stop searching if we've found an ideal option, and we know whether there is a worse choice
        self.best.1 == self.ideal_turns && self.was_worse
    }
}

impl Clubs<'_> {
    pub fn new<'a>(answers: &'a Vec<Word>, valid: &'a Vec<Word>) -> Clubs<'a> {
        let mut result = Clubs {
            letters: [LetterClubs::new(); 26],
            answer_count: answers.len(),
            answers,
            valid: valid
        };

        // Add each Word to the club for each letter+position
        for (word_index, word) in result.answers.iter().enumerate() {
            for (letter_index, letter) in word.iter_index().enumerate() {
                result.letters[letter as usize].pos[letter_index].add(word_index);
            }
        }

        // Fill out the 'any' club per letter to the OR of all positions for the letter
        for club in result.letters.iter_mut() {
            for pos in club.pos.iter() {
                club.any.union_with(pos);
            }
        }

        result
    }

    /// Shortcut to get a vector including all answers in this Clubs instance.
    ///  All search methods take a slice to search within, so that all subsets can also be evaluated easily.
    pub fn all_vector(&self) -> BitVectorSlice {
        BitVectorSlice::new_all(self.answer_count)
    }

    /// Convert a vector to the set of answers included within it
    pub fn cluster_to_words(&self, cluster: BitVectorSlice) -> Vec<Word> {
        cluster.iter().map(|index| self.answers[index].clone()).collect::<Vec<Word>>()
    }

    /// Write a short string 
    pub fn vector_to_string(&self, within: BitVectorSlice) -> String {
        let cluster = within.iter().map(|index| self.answers[index].to_string()).collect::<Vec<String>>();
        if within.count() as usize == self.answer_count {
            "[*]".into()
        } else {
            format!("[{}]", cluster.join(", "))
        }
    }

    /// Return the set of clusters after making a specific guess.
    ///  Call cluster_to_words to convert the answer set to the Words when needed.
    pub fn split(&self, guess: Word, within: BitVectorSlice) -> Vec<(Response, BitVectorSlice)> {
        let mut clusters = Vec::new();

        self.for_each_cluster(within, guess, &mut |response, subcluster| {
            clusters.push((response, subcluster));
        });

        clusters
    }

    /// Show all words in a cluster and the ideal turns and cluster vector for each
    pub fn print_in_cluster(&self, within: BitVectorSlice) -> String {
        let mut options = Vec::new();

        for index in within.iter() {
            let word = self.answers[index];
            options.push((word, self.count_ideal_turns(within, word)));
        }

        options.sort_by(|l, r| r.1.cmp(&l.1).then(l.0.cmp(&r.0)));

        let mut result = String::new();
        for (word, turns) in options.iter() {
            result += &format!("{}  {}  {}\n", word, turns, self.cluster_vector(within, *word).to_string());
        }

        result
    }

    /// Return the best next guess, or None if it doesn't matter, and the total number of turns left to solve each answer in 'within' with that guess.
    pub fn best_next_guess(&self, within: BitVectorSlice) -> (Option<Word>, usize) {
        let mut choices = HashMap::new();
        let best_turns = self.count_best_turns(within, &mut choices);
        
        if let Some(best) = choices.get(&within) {
            (Some(best.0), best.1)
        } else {
            (None, best_turns)
        }
    }

    pub fn count_best_turns_after(&self, within: BitVectorSlice, next_guess: Word, choices: &mut HashMap<BitVectorSlice, (Word, usize)>) -> usize {
        // One per answer left for next_guess itself being guessed
        let outer_count = within.count() as usize;
        let mut best_turns = outer_count;

        // Plus the remaining turns per sub-cluster
        self.for_each_cluster(within, next_guess, &mut |_, subcluster| {
            if subcluster == within {
                // Don't recurse if the guess leaves all words in the same subcluster
                best_turns += outer_count * outer_count;
            } else {
                best_turns += self.count_best_turns(subcluster, choices);
            }
        });

        best_turns
    }

    pub fn count_best_turns(&self, within: BitVectorSlice, choices: &mut HashMap<BitVectorSlice, (Word, usize)>) -> usize {
        let outer_count = within.count();
        if outer_count < 3 {
            // For one or two words, random guessing is the best outcome
            return ((outer_count * 2) - 1) as usize;
        } else {
            // If we've solved this subcluster before, return the previous answer
            if let Some(cached_best) = choices.get(&within) {
                return cached_best.1;
            }

            // Consider each in-cluster guess
            let mut state = BestConsiderState::new(within, self, choices);
            for guess in within.iter().skip(1).map(|index| self.answers[index]) {
                state.consider(guess);
                
                if state.stop_searching() { 
                    break; 
                }
            }

            // If an out-of-cluster choice could be better, consider them
            state.ideal_turns += 1;
            if state.best.1 > state.ideal_turns {
                for guess in self.valid {
                    state.consider(*guess);

                    if state.stop_searching() { 
                        break; 
                    }
                }
            }

            if state.was_worse {
                state.choices.insert(within, state.best);
            }

            return state.best.1;
        }
    }

    pub fn best_strategy(&self, within: BitVectorSlice, choices: &HashMap<BitVectorSlice, (Word, usize)>, show_all: bool, parent: &mut WordleTree) {
        let cluster_count = within.count() as usize;
        if cluster_count < 3 && show_all == false { return; }

        let first_word = within.iter().next().map(|index| self.answers[index]).unwrap();

        if let Some((best, turns)) = choices.get(&within) {
            let mut node = WordleTree::new(WordleTreeIdentifier::Cluster(first_word), WordleGuess::Specific(*best));
            node.outer_total_turns = *turns as f64;
            node.answer_count = cluster_count;

            if cluster_count <= wordle_tree::LIST_ANSWERS_MAX_COUNT {
                node.answers = Some(self.cluster_to_words(within));
            }

            self.for_each_cluster(within, *best, &mut |_, subcluster| {
                self.best_strategy(subcluster, choices, show_all, &mut node);
            });

            parent.add_child_without_rollup(node);
        } else {
            let mut node = WordleTree::new(WordleTreeIdentifier::Cluster(first_word), WordleGuess::Random);
            node.answer_count = cluster_count;

            if cluster_count <= wordle_tree::LIST_ANSWERS_MAX_COUNT {
                node.answers = Some(self.cluster_to_words(within));
            }

            self.for_each_cluster(within, first_word, &mut |_, subcluster| {
                self.best_strategy(subcluster, choices, show_all, &mut node);
            });

            if node.has_children() {
                node.next_guess = WordleGuess::Specific(first_word);
            }

            parent.add_child_without_rollup(node);
        }
    }

    pub fn count_random_turns(&self, within: BitVectorSlice) -> f64 {
        let outer_count = within.count();
        if outer_count < 3 {
            return ((outer_count * 2) - 1) as f64;
        } else {
            let mut inner_total = 0.0;

            for index in within.iter() {
                let guess = self.answers[index];

                self.for_each_cluster(within, guess, &mut |_, subcluster| {
                    inner_total += self.count_random_turns(subcluster);
                });
            }

            return (outer_count as f64) + (inner_total / outer_count as f64);
        }
    }

    pub fn count_random_turns_after(&self, within: BitVectorSlice, next_guess: Word) -> f64 {
        // One per answer left for next_guess itself being guessed
        let mut best_turns = within.count() as f64;

        // Plus the remaining turns per sub-cluster
        self.for_each_cluster(within, next_guess, &mut |_, subcluster| {
            best_turns += self.count_random_turns(subcluster);
        });

        best_turns
    }

    pub fn count_random_turns_after_cache(&self, within: BitVectorSlice, next_guess: Word, cache: &mut HashMap<BitVectorSlice, f64>) -> f64 {
        // One per answer left for next_guess itself being guessed
        let mut best_turns = within.count() as f64;

        // Plus the remaining turns per sub-cluster
        self.for_each_cluster(within, next_guess, &mut |_, subcluster| {
            if let Some(inner_turns) = cache.get(&subcluster) {
                best_turns += inner_turns;
            } else {
                let inner_turns = self.count_random_turns(subcluster);
                cache.insert(subcluster, inner_turns);
                best_turns += inner_turns;
            }
        });

        best_turns
    }

    pub fn count_ideal_turns(&self, within: BitVectorSlice, guess: Word) -> usize {
        let mut turns_left = within.count() as usize;

        self.for_each_cluster(within, guess, &mut |_, cluster| {
            let cluster_count = cluster.count() as usize;
            turns_left += (cluster_count * 2)  - 1;
        });

        turns_left
    }

    pub fn cluster_vector(&self, within: BitVectorSlice, guess: Word) -> ClusterVector {
        let mut cv = ClusterVector::new(Vec::new());

        self.for_each_cluster(within, guess, &mut |_, cluster| {
            cv.add(cluster.count() as usize);
        });

        cv
    }

    fn for_each_cluster(&self, within: BitVectorSlice, guess: Word, action: &mut impl FnMut(Response, BitVectorSlice)) {
        if guess.has_repeat_letters() {
            self.for_each_cluster_repeats_recursive(guess, 0, within, 0u16, action);
        } else {
            self.for_each_cluster_recursive(guess, 0, within, 0u16, action);
        }
    }

    fn for_each_cluster_recursive(&self, guess: Word, next_letter_index: usize, matches: BitVectorSlice, tiles: u16, action: &mut impl FnMut(Response, BitVectorSlice)) {
        // If set is empty, stop
        if matches.count() == 0 { return; }

        if next_letter_index >= 5 {
            // If we've intersected all letter-sets, call the action (except for the 'self' cluster)
            if tiles != response::ALL_GREEN {
                action(Response::new(tiles), matches);
            }
        } else {
            let letter = guess.iter_index().nth(next_letter_index).unwrap() as usize;

            // Recurse for Green: Have this letter at this position
            let mut green_matches = self.letters[letter].pos[next_letter_index];
            green_matches.intersect_with(&matches);
            self.for_each_cluster_recursive(guess, next_letter_index + 1, green_matches, (tiles << 2) + 2, action);

            // Recurse for Yellow: Have this letter, but not at this position
            let mut yellow_matches = self.letters[letter].any;
            yellow_matches.except_with(&green_matches);
            yellow_matches.intersect_with(&matches);
            self.for_each_cluster_recursive(guess, next_letter_index + 1, yellow_matches, (tiles << 2) + 1, action);

            // Recurse for Black: Words which don't have this letter at all
            let mut black_matches = self.letters[letter].any;
            black_matches.not(self.answer_count);
            black_matches.intersect_with(&matches);
            self.for_each_cluster_recursive(guess, next_letter_index + 1, black_matches, (tiles << 2) + 0, action);
        }
    }

    fn for_each_cluster_repeats_recursive(&self, guess: Word, next_letter_index: usize, matches: BitVectorSlice, tiles: u16, action: &mut impl FnMut(Response, BitVectorSlice)) {
        // If set is empty, stop
        if matches.count() == 0 { return; }

        if next_letter_index >= 5 {
            // If we've intersected all letter-sets, call the action (except for the 'self' cluster)
            if tiles != response::ALL_GREEN {
                action(Response::new(tiles), matches);
            }
        } else {
            let letter = guess.iter_index().nth(next_letter_index).unwrap() as usize;

            // Find and Recurse for Green: Have this letter at this position
            let mut green_matches = self.letters[letter].pos[next_letter_index];
            green_matches.intersect_with(&matches);
            self.for_each_cluster_repeats_recursive(guess, next_letter_index + 1, green_matches, (tiles << 2) + 2, action);

            // Find and Recurse for Yellow: Find answers with *remaining* *unmatched* copes of the letter
            let yellow_matches = self.yellows_for(guess, letter as u8, next_letter_index, matches);
            //yellow_matches.intersect_with(&matches); [already done inside]
            self.for_each_cluster_repeats_recursive(guess, next_letter_index + 1, yellow_matches, (tiles << 2) + 1, action);

            // Find and Recurse for Blacks; all answers not green or yellow
            let mut black_matches = matches.clone();
            black_matches.except_with(&green_matches);
            black_matches.except_with(&yellow_matches);
            self.for_each_cluster_repeats_recursive(guess, next_letter_index + 1, black_matches, (tiles << 2) + 0, action);
        }
    }

    /// Correctly find which answers get a yellow tile for a given guess letter when the guess has repeated letters.
    fn yellows_for(&self, guess: Word, letter: u8, letter_index: usize, within: BitVectorSlice) -> BitVectorSlice {
        let mut occurrences_before = 0;
        let mut have_any_unmatched = BitVectorSlice::new();

        for (index, letter_here) in guess.iter_index().enumerate() {
            if letter_here == letter {
                // Count how many times 'letter' was guessed before letter_index
                if index < letter_index {
                    occurrences_before += 1;
                }
            } else {
                // Collect all answers with 'letter' in any positions where it was *not* guessed
                have_any_unmatched.union_with(&self.letters[letter as usize].pos[index]);
            }
        }

        // Yellow only if not green here
        have_any_unmatched.except_with(&self.letters[letter as usize].pos[letter_index]);

        // Filter to 'within' also, to avoid extra per-word word
        have_any_unmatched.intersect_with(&within);

        // If this is the first copy of 'letter' in the guess, all words with any unmatched copies get yellow.
        if occurrences_before == 0 {
            return have_any_unmatched;
        }

        // If this is a later copy of 'letter' in the guess, we have to figure out which answers have 
        //  unmatched copies which weren't assigned to an earlier copy of 'letter' in the guess.
        let mut had_enough_unmatched = BitVectorSlice::new();

        for answer_index in have_any_unmatched.iter() {
            let answer = self.answers[answer_index];
            let mut unmatched_count = 0;

            for (index, (answer_letter, guess_letter)) in answer.iter_index().zip(guess.iter_index()).enumerate() {
                if answer_letter == letter {
                    if guess_letter == letter {
                        // 'letter' in guess and answer here; green tile, not an unmatched copy that could be yellow at letter_index.
                    } else {
                        // Unmatched 'letter' in guess.
                        unmatched_count += 1;
                    }
                } else {
                    if guess_letter == letter {
                        // Unmatched 'letter' scores a yellow earlier in guess here
                        if index < letter_index {
                            unmatched_count -= 1;
                        }
                    } else {
                        // Neither guess or answer had the letter here
                    }
                }
            }

            // If there were remaining unmatched copies of 'letter' in the answer after earlier copies in the guess absorbed them, we get a yellow at letter_index
            if unmatched_count > 0 {
                had_enough_unmatched.add(answer_index);
            }
        }

        // Return the set of answers which had at least one unmatched copy of letter
        //  that wasn't used up by an earlier position of letter in the guess
        return had_enough_unmatched;
    }
}

#[cfg(test)]
mod tests {
    use assert_float_eq::*;
    use super::*;
    use crate::*;

    #[test]
    fn split() {
        let valid = Vec::new();
        let words = wv("blush, slosh, slush, flush, gloss, floss");
        let clubs = Clubs::new(&words, &valid);

        // Verify 'within' respected, and the guess itself doesn't appear in a cluster
        assert_eq!(split_to_string(&clubs, w("blush"), BitVectorSlice::from_vec(vec!(0, 1, 2))), ".L.SH: [slosh]; .LUSH: [slush]");

        // Split all words with blush
        assert_eq!(split_to_string(&clubs, w("blush"), clubs.all_vector()), ".L.S.: [gloss, floss]; .L.SH: [slosh]; .LUSH: [slush, flush]");

        // Verify repeat letter guesses FLOSS S5 should get yellow for S1s because S4 is green, so it didn't take S1.
        assert_eq!(split_to_string(&clubs, w("floss"), clubs.all_vector()), ".L.S.: [blush]; .L.Ss: [slush]; .LOSs: [slosh]; .LOSS: [gloss]; FL.S.: [flush]");

        // Floss S4 matches exact on each other word, so no yellos
        assert_eq!(try_yellows(&clubs, w("floss"), 3), "");
    }

    fn try_yellows(clubs: &Clubs, guess: Word, letter_index: usize) -> String {
        let yellows = clubs.yellows_for(
            guess, 
            guess.iter_index().nth(letter_index).unwrap(), 
            letter_index, 
            clubs.all_vector());

        let mut result = String::new();

        for answer in yellows.iter().map(|index| clubs.answers[index]) {
            if result.len() > 0 { result += ", "; }
            result += &answer.to_string();
        }
        
        result
    }

    fn split_to_string(clubs: &Clubs, guess: Word, within: BitVectorSlice) -> String {
        let mut result = String::new();

        // Find clusters, then sort by Response
        let mut split = clubs.split(guess, within);
        split.sort_by(|l, r| l.0.cmp(&r.0));

        // Write: <knowns>: [answer, answer, answer]; <knowns> ...
        for (i, (response, cluster)) in split.iter().enumerate() {
            if i > 0 { result += "; "; }
            result += &format!("{}: [", response.to_knowns_string(&guess));

            // Write answers (in same order as they exist in club)
            for (j, answer) in clubs.cluster_to_words(*cluster).iter().enumerate() {
                if j > 0 { result += ", "; }
                result += &answer.to_string();
            }

            result += "]";
        }

        result
    }

    #[test]
    fn yellows_for() {
        let valid = Vec::new();
        let words = wv("gnome, nudge, undue, venue");
        let clubs = Clubs::new(&words, &valid);

        // Yellows: Non-repeated letter (N2) -> "nudge, venue" (gnome, undue are green)
        assert_eq!(try_yellows(&clubs, w("undue"), 1), "nudge, venue");

        // Yellows: First of a letter (U1) -> "nudge" (undue green, venue green for second copy, gnome doesn't have)
        assert_eq!(try_yellows(&clubs, w("undue"), 0), "nudge");

        // Yellows: Second of a letter (U4) -> None (undue, venue green. Nudge U2 taken by U1 yellow)
        assert_eq!(try_yellows(&clubs, w("undue"), 3), "");

        let words = wv("tepee, reset, exert, trees, exist");
        let clubs = Clubs::new(&words, &valid);

        // Yellows: First of three (E2) -> exert E1, trees E3, exist E1 (tepee, reset green)
        assert_eq!(try_yellows(&clubs, w("tepee"), 1), "exert, trees, exist");

        // Yellows: First of three (E4) -> exert E3, no more in exist, trees E4 green
        assert_eq!(try_yellows(&clubs, w("tepee"), 3), "exert");

        // Yellows: Third of three (E5) -> none (impossible)
        assert_eq!(try_yellows(&clubs, w("tepee"), 4), "");

        let words = wv("blush, slosh, slush, flush, gloss, floss");
        let clubs = Clubs::new(&words, &valid);

        // Floss S5 should match yellow to the two words with a second S that's not S5
        //  Issue: S5 thinks it's the second S in FLOSS (true), so it's trying to find two unmatched S (wrong)
        assert_eq!(try_yellows(&clubs, w("floss"), 4), "slosh, slush");
    }

    #[test]
    fn club_basics() {
        let words = wv("ennui, begin, denim, given, vixen, widen, index");
        let clubs = Clubs::new(&words, &words);
        let within = clubs.all_vector();

        // How many turns if we guess "begin" next? 
        //  CV = [2, 2] so 1 + 1 + (1 + 2) + (1 + 2) = 8 turns
        assert_eq!(clubs.cluster_vector(within, w("begin")).to_string(), "[2, 2]");
        assert_eq!(clubs.count_ideal_turns(within, w("begin")), 7 + 8);

        // How many turns if we guess "ennui" next?
        //  CV = [2, 0, 0, 1] so 1 + 1 + (1 + 2 + 2 + 2) = 9 turns
        assert_eq!(clubs.cluster_vector(within, w("ennui")).to_string(), "[2, 0, 0, 1]");
        assert_eq!(clubs.count_ideal_turns(within, w("ennui")), 7 + 9);

        // How many turns if we guess "vixen" next?
        //  CV = [4, 1] so 1 + 1 + 1 + 1 + (1 + 2) = 7 turns
        assert_eq!(clubs.cluster_vector(within, w("vixen")).to_string(), "[4, 1]");
        assert_eq!(clubs.count_ideal_turns(within, w("vixen")), 7 + 7);

        // How many turns if we guess "index" next?
        //  CV = [6] so 1 + 1 + 1 + 1 + 1 + 1 = 6 turns
        assert_eq!(clubs.cluster_vector(within, w("index")).to_string(), "[6]");
        assert_eq!(clubs.count_ideal_turns(within, w("index")), 7 + 6);

        // How many turns if we guess "dumbo" next?
        //  CV = [3, 2] so 1 + 1 + 1 + (1 + 2) + (1 + 2) = 9 turns
        assert_eq!(clubs.cluster_vector(within, w("dumbo")).to_string(), "[3, 2]");
        assert_eq!(clubs.count_ideal_turns(within, w("dumbo")), 7 + 9);

        // Verify "best_next_guess" finds the option with the fewest ideal turns 
        assert_eq!(clubs.best_next_guess(within), (Some(w("index")), 7 + 6));

        // parse, clint, index, * would be 27 turns (3*1 + 4*6)
        // There are 13 turns from the index guess (1*1 + 2*6)
        assert_eq!(clubs.print_in_cluster(within),
"ennui  16  [2, 0, 0, 1]
begin  15  [2, 2]
denim  15  [2, 2]
given  14  [4, 1]
vixen  14  [4, 1]
widen  14  [4, 1]
index  13  [6]
");

        // Ask within a subset: [begin, denim, given]
        let mut within = BitVectorSlice::new_all(4);
        within.remove(0);

        // Ideal turns for "denim" is (1 + 2 + 2) = 5
        assert_eq!(clubs.count_ideal_turns(within, w("denim")), 5);

        // Ideal turns is 5 for each of them
        assert_eq!(clubs.best_next_guess(within), (None, 5));

        // parse, clint, index, * would be 27 turns (3*1 + 4*6)
        // There are 13 turns from the index guess (1*1 + 2*6)
        assert_eq!(clubs.print_in_cluster(within),
"begin  5  [2]
denim  5  [2]
given  5  [2]
");
    }

    #[test]
    fn best_next_guess() {
        let valid = Vec::new();
        
        // Safe Triple: returns two turns after guess, and false (which guess doesn't matter)
        let words = wv("begin, denim, given");
        let clubs = Clubs::new(&words, &valid);
        assert_eq!(clubs.best_next_guess(clubs.all_vector()), (None, 5));

        // Unsafe Triple: returns a safe option and true (which one matters) ["widen" can't tell between vixen and given because _i_en common and 'w', 'd' in neither]
        let words = wv("given, vixen, widen");
        let clubs = Clubs::new(&words, &valid);
        assert_eq!(clubs.best_next_guess(clubs.all_vector()), (Some(w("given")), 5));

        // Terrible Triple: returns first option, 3, false; no better choice
        let words = wv("aaaaa, bbbbb, ccccc");
        let clubs = Clubs::new(&words, &valid);
        assert_eq!(clubs.best_next_guess(clubs.all_vector()), (None, 6));

        let valid = wv("aaaaa, bbbbb, ccccc, ddddd, eeeee, abcde");

        // Terrible Triple: out of cluster can never be better
        let words = wv("aaaaa, bbbbb, ccccc");
        let clubs = Clubs::new(&words, &valid);
        assert_eq!(clubs.best_next_guess(clubs.all_vector()), (None, 6));

        // Terrible Five; verify out-of-cluster best is found
        let words = wv("aaaaa, bbbbb, ccccc, ddddd, eeeee");
        let clubs = Clubs::new(&words, &valid);
        assert_eq!(clubs.best_next_guess(clubs.all_vector()), (Some(w("abcde")), 10));
    }

    #[test]
    fn count_random_turns() {
        let valid = Vec::new();

        // Safe Triple: (1 + 2 + 2) turns for each case, so 5.0 random guessing total
        let words = wv("begin, denim, given");
        let clubs = Clubs::new(&words, &valid);
        assert_float_absolute_eq!(clubs.count_random_turns(clubs.all_vector()), 5.0);

        // Partial Triple: given = 5.0; vixen = 5.0; widen = 6.0, so average is 5.333
        let words = wv("given, vixen, widen");
        let clubs = Clubs::new(&words, &valid);
        assert_float_absolute_eq!(clubs.count_random_turns(clubs.all_vector()), 16.0 / 3.0);

        // Terrible Triple: (1 + 2 + 3) turns for each case, so 6.0 random guessing total
        let words = wv("aaaaa, bbbbb, ccccc");
        let clubs = Clubs::new(&words, &valid);
        assert_float_absolute_eq!(clubs.count_random_turns(clubs.all_vector()), 6.0);

        // Safe Quad: (1 + 2 + 2 + 2)
        let words = wv("gnome, nudge, undue, venue");
        let clubs = Clubs::new(&words, &valid);
        assert_float_absolute_eq!(clubs.count_random_turns(clubs.all_vector()), 7.0);

        // Four options.
        //   booze  6	[0, 0, 1]
        // * dodge  3	[3]
        // * gouge  3	[3]
        // * vogue  3	[3]
        //  Three of them distinguish the others and each take 7 turns. (1 + 2 + 2 + 2)
        //  Booze takes one turn when right, and one turn plus the "three fully identifies" otherwise
        //   (1 + (3 * (1 + 5) / 3) = 9 turns;
        //  So 3 * 7 + 1 * 9 = 30 / 4 options = 7.5 total on average.
        let words = wv("booze, dodge, gouge, vogue");
        let clubs = Clubs::new(&words, &valid);
        assert_float_absolute_eq!(clubs.count_random_turns(clubs.all_vector()), 7.5);
    }

    #[test]
    fn count_best_turns() {
        let valid = Vec::new();

        // Safe Triple: (1 + 2 + 2) = 5 turns for each case; no choice recorded
        let tree = run_best_turns("begin, denim, given", &valid);
        assert_eq!(tree.outer_total_turns, 5.0);
        assert_eq!(tree.next_guess, WordleGuess::Random);

        // Terrible Triple: (1 + 2 + 3) = 6 turns for each case; no choice recorded
        let tree = run_best_turns("aaaaa, bbbbb, ccccc", &valid);
        assert_eq!(tree.outer_total_turns, 6.0);

        // Partial Triple: given = 5.0; vixen = 5.0; widen = 6.0; suggest "given"
        let tree = run_best_turns("given, vixen, widen", &valid);
        assert_eq!(tree.outer_total_turns, 5.0);
        assert_eq!(tree.next_guess, WordleGuess::Specific(w("given")));

        // parse clint godly -> folly [2, 1]
        // ..... .l... .O.LY
        //  folly 1
        //    holly 2
        //      jolly 3
        //    lowly 2
        //    wolly 2
        let tree = run_best_turns("folly, holly, jolly, lowly, wooly", &valid);
        assert_eq!(tree.outer_total_turns, 10.0);
        assert_eq!(tree.next_guess, WordleGuess::Specific(w("folly")));

        // parse clint
        // ..... .l... (bulky, 19)
        //   godly  29	[11, 1, 0, 0, 1]
        //     folly  5  [2, 1]
        let tree = run_best_turns("bulky, bully, dolly, dully, folly, fully, ghoul, godly, golly, gully, holly, jolly, lobby, lowly, mogul, moldy, oddly, wooly, would", &valid);
        assert_eq!(tree.outer_total_turns, 43.0);
        assert_eq!(tree.next_guess, WordleGuess::Specific(w("godly")));
        assert_eq!(tree.subtree.unwrap().get(0).unwrap().next_guess, WordleGuess::Specific(w("folly")));
        //assert_eq!(tree.to_string(), "");//, 43, "[*] -> godly 43; [folly, holly, jolly, lowly, wooly] -> folly 10");
    }

    fn run_best_turns(words: &str, valid: &Vec<Word>) -> WordleTree {
        let words = wv(words);
        let clubs = Clubs::new(&words, valid);

        let mut choices = HashMap::new();
        let outer_turns = clubs.count_best_turns(clubs.all_vector(), &mut choices);

        let mut tree = WordleTree::new_sentinel();
        clubs.best_strategy(clubs.all_vector(), &choices, false, &mut tree);
        
        let mut root = tree.take_first_child().unwrap();
        root.outer_total_turns = outer_turns as f64;
        root
    }
}