use std::{collections::HashMap, cmp::Ordering};
use crate::{rank, response::Response, word::Word, cluster_vector::ClusterVector};

/// Represents a Wordle game state with any number of guesses.
pub struct State<'a> {
    // Guesses made so far, along with the constrained response, if any
    guesses: Vec<(Word, Option<Response>)>,

    // Remaining answer possibilities, organized by the responses to the guesses which match them
    remaining: HashMap<Vec<Response>, Vec<Word>>,

    // Words which may be used for remaining guesses
    _valid: &'a Vec<Word>,
}

impl State<'_> {
    /// Create a starting Wordle state for a given set of possible answers and allowed guesses
    pub fn new<'a>(answers: &'a Vec<Word>, valid: &'a Vec<Word>) -> State<'a> {
        let mut remaining: HashMap<Vec<Response>, Vec<Word>> = HashMap::new();
        remaining.insert(Vec::new(), answers.clone());

        State {
            guesses: Vec::new(),
            remaining: remaining,
            _valid: valid,
        }
    }

    /// Compute the results after another guess.
    ///  If a response is provided, only answers matching that response will be kept.
    pub fn filter(&mut self, guess: Word, response: Option<Response>) {
        let mut new_remaining: HashMap<Vec<Response>, Vec<Word>> = HashMap::new();
        for (responses, answers) in self.remaining.iter() {
            for answer in answers {
                let new_response = Response::score(guess, *answer);

                if let Some(expected) = response {
                    if new_response != expected {
                        continue;
                    }
                }
                
                let mut new_responses = responses.clone();
                new_responses.push(new_response);

                let entry = new_remaining.entry(new_responses);
                entry.or_insert(Vec::new()).push(*answer);
            }
        }

        self.remaining = new_remaining;
        self.guesses.push((guess, response));
    }

    /// Find the best next guess given the current state.
    ///  It considers each valid guess against each cluster and ranks them by a scoring function.
    /// 
    ///  PERFORMANCE
    ///  ===========
    ///   Skip all guesses which reuse any previous guess words (1.9s -> 0.08s)
    ///   Don't make a full HashMap; just map each cluster individually and accumulate a shared CV. (7.5s -> 1.9s)
    ///   Keep best only instead of full options set. (1.9s -> 1.4s)
    ///   Build a new HashMap per guess/answer instead of making one and clearing each time. (1.4s -> 1.3s)
    pub fn best_next(&self, ranker: fn(&ClusterVector) -> usize) -> Vec<(usize, Word, ClusterVector)> {
        let mut options = Vec::new();
        let map = &self.remaining;

        // Option: Keep best only
        //let mut best = (Word::new(&"zzzzz").unwrap(), usize::MAX, ClusterVector::new(Vec::new()));

        // Option: Exclude all guesses which reuse previously guessed letters
        // let mut used_letters = 0u32;
        // for (guess, _) in self.guesses.iter() {
        //     used_letters = used_letters | guess.letters_in_word();
        // }

        // For each allowed word ...
        for word in self._valid.iter() {
            // Skip words which contain letters already used
            //if word.letters_in_word() & used_letters != 0 { continue; }

            let mut cv = Vec::new();

            // For each existing bucket ...
            for (_, answers) in map.iter() {
                let mut inner_map = HashMap::new();

                // Score each answer
                for answer in answers {
                    if answer == word { continue; }
                    let response = Response::score(*word, *answer);
                    let entry = inner_map.entry(response);
                    *entry.or_insert(0) += 1;
                }

                // Count each sub-bucket by size
                for (_, count) in inner_map.iter() {
                    while cv.len() < *count { cv.push(0); }
                    cv[count - 1] += 1;
                }
            }

            // Accumulate an overall score for this word as a next guess
            let cv = ClusterVector::new(cv);
            let score = ranker(&cv);
            options.push((score, *word, cv));
            // if score < best.1 {
            //     best = (*word, score, cv);
            // }
        }

        options
    }

    pub fn to_cluster_vector(&self) -> ClusterVector {
        ClusterVector::from_map(&self.remaining)
    }

    pub fn print_guesses(&self) {
        let guesses = self.guesses.iter().map(|(guess, _)| guess.to_string()).collect::<Vec<String>>();
        println!("{}", guesses.join(" "));
    }

    pub fn print_knowns(&self) {
        let knowns = self.guesses.iter().map(|(guess, response)| 
        { 
            if let Some(response) = response {
                response.to_knowns_string(&guess)
            } else {
                "*****".to_string()
            }
        }).collect::<Vec<String>>();

        println!("{}", knowns.join(" "));
    }

    // Desired Output Options:
    //  - Show or hide cluster vector (scores only)
    //  - Exclude small clusters
    //  - Exclude "any guess fine" clusters
    //  - Show strategy summary only
    //  - Show "no-lose-min" level strategy only

    /// Print the current state, showing the guesses, response constraints, and
    ///  each possible set of responses with potential answers.
    pub fn print(&self) {
        let find_random_exact = self.guesses.len() > 1; // true;

        // Write current guesses and known letter summary
        self.print_guesses();
        self.print_knowns();
        println!();

        let cv = self.to_cluster_vector();
        let total_answers = cv.word_count();
        let total_turns_before = self.guesses.len() * total_answers;
        let total_turns_pessimistic = cv.total_turns_pessimistic() + total_turns_before;
        let total_turns_predicted = cv.total_turns_predicted() + total_turns_before;

        println!("{} answers in {} clusters.", total_answers, self.remaining.len());

        println!("       CV: {}", cv.to_string());

        let mut unsafe_cv: Vec<usize> = Vec::new();
        for (_responses, answers) in self.remaining.iter() {
            if rank::is_safe_cluster(answers) == false {
                while unsafe_cv.len() < answers.len() { unsafe_cv.push(0); }
                unsafe_cv[answers.len() - 1] += 1;
            }
        }
        let unsafe_cv = ClusterVector::new(unsafe_cv);
        println!("Unsafe CV: {}", unsafe_cv.to_string());

        println!();
        println!("Average Turns:");

        // Compute exact guessing turns, but only if > 1 guess already (performance)
        let mut strategy: Vec<(f64, usize, Word, Word)> = Vec::new();

        if find_random_exact {
            // Compute expected turns if guessing from here
            let total_turns_guessing_exact = rank::total_turns_random_map(&self.remaining);
            let total_guessing_exact = total_turns_guessing_exact + total_turns_before;
            let average_turns_guessing_exact = (total_guessing_exact as f64) / (total_answers as f64);
            println!("{:.4} (guess, exact); {} = ({} + {})", average_turns_guessing_exact, total_guessing_exact, total_turns_before, total_turns_guessing_exact);

            // Compute total turns with best guesses from here
            let total_turns_perfect = rank::total_turns_perfect_map_exact(&self.remaining, &self._valid, Some(&mut strategy));
            let total_perfect = total_turns_perfect + total_turns_before as f64;
            let average_turns_perfect = (total_perfect as f64) / (total_answers as f64);
            println!("{:.4} (perfect);  {:.0} = ({} + {:.0})", average_turns_perfect, total_perfect, total_turns_before, total_turns_perfect);
        }

        println!("{:.4} (predicted):  {:.0}", total_turns_predicted as f64 / total_answers as f64, total_turns_predicted);
        println!("{:.4} (pessimistic): {:.0}", total_turns_pessimistic as f64 / total_answers as f64, total_turns_pessimistic);

        println!();

        // Sort by cluster size descending
        let mut clusters: Vec<(&Vec<Response>, &Vec<Word>)> = self.remaining.iter().collect();
        clusters.sort_by(|l, r| r.1.len().cmp(&l.1.len()).then(order_by_responses(l.0, r.0)));

        //let mut strategy = Vec::new();

        let mut total_answers = 0;
        let mut total_turns = 0;

        // For each cluster, print the knowns, number of possible answers, and answers ranked by fitness as next guess
        for (responses, answers) in clusters {
            total_answers += answers.len();
            print_responses(responses, &self.guesses);

            if answers.len() < 3 {
                // Print 1- and 2- clusters single line; any next guess is equally good.
                print!(" : ");
                for answer in answers {
                    print!("{} ", answer.to_string());
                }

                // It will take one more turn for each single and three for each pair (one guessed first and one guessed second)
                total_turns += if answers.len() == 1 { 1 } else { 3 };
            } else {
                // Rank options by cluster vectors and pessimistic turns
                let inner = State::new(&answers, &answers);
                let in_cluster_options = inner.best_next(ClusterVector::total_turns_pessimistic);
                let mut best = in_cluster_options.iter().min_by(|l, r| l.0.cmp(&r.0)).unwrap().clone();

                // If we found a "best" already for this cluster, show that
                let found = strategy.iter().find(|(_, count, first, _)| *count == answers.len() && first == answers.first().unwrap());
                if let Some((turns, _, _, choice)) = found {
                    let mut map = HashMap::new();
                    rank::split(answers, *choice, &mut map);
                    let cv = ClusterVector::from_map(&map);

                    best = (*turns as usize, *choice, cv);
                } else {
                    // Otherwise, if no in-cluster guess was ideal, look for out-of-cluster options
                    if best.0 > answers.len() - 1 {
                        let inner = State::new(&answers, &self._valid);
                        let all_options = inner.best_next(ClusterVector::total_turns_pessimistic);
                        let all_best = all_options.iter().min_by(|l, r| l.0.cmp(&r.0)).unwrap();

                        // If there was a better out-of-cluster guess, identify it
                        if all_best.0 < best.0 {
                            best = all_best.clone();
                            //strategy.push((responses.clone(), best.1.clone(), best.2.clone()));
                        }
                    }
                }

                // Write the cluster: "first_word": count (best_next)
                let best_in_cluster = answers.contains(&best.1);
                let mark = if best_in_cluster { "" } else { "x " };
                println!(" ({}, {}) -> {}{}", answers.first().unwrap(), answers.len(), mark, best.1);
                print_options_sorted(&in_cluster_options, true);

                // If the best next guess was out-of-cluster, show the CV and turns for it
                if !best_in_cluster {
                    println!();
                    println!("{}", print_option(&best, 'x', true));
                }

                // It will take 'count' more turns (for the next guess for this cluster) plus the number of subsequent turns found in best_next
                let count = answers.len();
                total_turns += count + best.0;
            }

            println!();
        }

        println!();
        println!("Average Turns:");
        let total_best_pessimistic = total_turns + total_turns_before;
        let average_turns = (total_best_pessimistic as f64) / (total_answers as f64);
        println!("{:.4} (shown); {} = ({} + {})", average_turns, total_best_pessimistic, total_turns_before, total_turns);
        
        // // Diagnostics to verify turn calculations
        // println!();
        // println!("Turns: {} singles + {} pairs + {} above = {} turns", _total_singles, _total_pairs, _total_above, (_total_singles + _total_pairs + _total_above));
        // println!("Words: {} singles + {} pairs + {} above (in {} clusters) = {} words", _total_singles, (2 * _total_pairs / 3), _answers_above, _cluster_above, (_total_singles + (2 * _total_pairs / 3) + _answers_above));
        // println!("Clusters: {} singles + {} pairs + {} above = {} clusters", _total_singles, (_total_pairs / 3), _cluster_above, (_total_singles + (_total_pairs / 3) + _cluster_above));

        // Show specific strategy, if computed
        if find_random_exact {
            println!();
            println!("Strategy: ({})", strategy.len());
            strategy.sort_by(|l, r| r.1.cmp(&l.1).then_with(|| l.2.cmp(&r.2)));
            for (inner_turns, cluster_size, cluster_first, guess) in strategy {
                let total_turns = (cluster_size * (self.guesses.len() + 1)) as f64 + inner_turns;
                println!(" {total_turns:.1} ({cluster_first}, {cluster_size}) -> {guess}");
            }
        }

        // println!();
        // println!("Strategy: ({})", strategy.len());
        // strategy.sort_by(|l, r| order_by_responses(&l.0, &r.0));
        // for (responses, guess, cv) in strategy {
        //     print_responses(&responses, &self.guesses);
        //     println!(" : {}  {}", guess, print_cluster_vector(&cv));
        // }
    }

    pub fn answers_left(&self) -> Vec<Word> {
        let mut answers_left = Vec::new();
        
        for (_, answers) in self.remaining.iter() {
            for answer in answers {
                answers_left.push(*answer);
            }
        }

        answers_left
    }
}

pub fn print_options_sorted(options: &Vec<(usize, Word, ClusterVector)>, print_details: bool) {
    let mut options = options.clone();
    options.sort_by(|l, r| r.0.cmp(&l.0));

    let best_score = options.last().unwrap().0;

    for option in options {
        let accent = if option.0 == best_score { '*' } else { ' ' };
        println!("{}", print_option(&option, accent, print_details));
    }
}

pub fn print_option(option: &(usize, Word, ClusterVector), accent: char, print_details: bool) -> String {
    let (score, word, cv) = option;
    let cv_text = cv.to_string();

    if print_details {
        format!("  {accent} {word}  {score}\t{cv_text}")
    } else {
        format!("  {accent} {word}  {score}")
    }
}

pub fn print_responses(responses: &Vec<Response>, guesses: &Vec<(Word, Option<Response>)>) {
    for (i, response) in responses.iter().enumerate() {
        if i > 0 { print!(" "); }
        print!("{}", response.to_knowns_string(&guesses[i].0));
    }
}

pub fn order_by_responses(left: &Vec<Response>, right: &Vec<Response>) -> Ordering {
    for (l, r) in left.iter().zip(right.iter()) {
        let order = l.cmp(r);
        if order != Ordering::Equal { return order; }
    }

    left.len().cmp(&right.len())
}

#[cfg(test)]
mod tests {
    use std::{path::Path, cmp::Ordering};
    use super::State;
    use crate::{word::Word, response::Response};

    #[test]
    fn state_basics() {
        let answers = Word::parse_file(Path::new("tst/_o_er/answers.txt"));
        let mut guesses = Word::parse_file(Path::new("tst/_o_er/guesses.txt"));
        answers.clone().append(&mut guesses);

        let mut state = State::new(&answers, &guesses);

        // Initially, there should be no guesses and all answers are in one cluster
        assert_eq!(state.remaining.len(), 1);
        assert_eq!(state.remaining[&Vec::new()].len(), 25);
        assert_eq!(state.guesses.len(), 0);

        // Consider options after guessing "vower"
        state.filter(Word::new(&"vower").unwrap(), None);
        let cv = state.to_cluster_vector();
        assert_eq!(cv.value, vec![2, 0, 0, 0, 1, 0, 1, 0, 0, 0, 1]);
        assert_eq!(cv.total_turns_pessimistic(), 111);
    }

    #[test]
    fn sort_responses() {
        // Green after Yellow
        assert_eq!(super::order_by_responses(&vec![r("Gbbbb")], &vec![r("ybbbb")]), Ordering::Greater);

        // Sort within Response
        assert_eq!(super::order_by_responses(&vec![r("Gybbb")], &vec![r("Gbbbb")]), Ordering::Greater);

        // Sort next Response if first ones equal
        assert_eq!(super::order_by_responses(&vec![r("bbbbb"), r("ybbbb")], &vec![r("bbbbb"), r("bbbbb")]), Ordering::Greater);

        // Sort by length if all equal
        assert_eq!(super::order_by_responses(&vec![r("bbbbb"), r("yyyyy")], &vec![r("bbbbb"), r("yyyyy"), r("ggggg")]), Ordering::Less);

        // Equal if everything equal
        assert_eq!(super::order_by_responses(&vec![r("bbbbb"), r("yyyyy")], &vec![r("bbbbb"), r("yyyyy")]), Ordering::Equal);
    }

    fn r(text: &str) -> Response {
        Response::from_str(text).unwrap()
    }
}