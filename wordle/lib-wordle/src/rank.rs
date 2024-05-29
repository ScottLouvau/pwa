use std::{collections::{HashMap, HashSet}, mem};
use crate::{word::{Word, self}, response::{Response, ResponseSet}, cluster_vector::ClusterVector};

/// Split a cluster given a next guess - return a map of each distinct response and the answers which get that response.
pub fn split(cluster: &Vec<Word>, guess: Word, map: &mut HashMap<Response, Vec<Word>>) {
    map.clear();

    for answer in cluster.iter() {
        if guess == *answer { continue; }
        let response = Response::score(guess, *answer);
        let entry = map.entry(response);
        entry.or_insert(Vec::new()).push(*answer);
    }
}

/// Split a cluster given a next guess - return a map of each distinct response and the answers which get that response.
pub fn split_as_set(cluster: &Vec<Word>, guess: Word, map: &mut HashMap<ResponseSet, Vec<Word>>) -> usize {
    let mut excluded_count = 0;
    map.clear();

    for answer in cluster.iter() {
        if guess == *answer { 
            excluded_count += 1;
            continue; 
        }

        let mut set = ResponseSet::new();
        set.push(Response::score(guess, *answer));
        
        let entry = map.entry(set);
        entry.or_insert(Vec::new()).push(*answer);
    }

    excluded_count
}

/// Split a cluster given multiple subsequent guesses
pub fn split_all(cluster: &Vec<Word>, guesses: &Vec<Word>, start: ResponseSet, map: &mut HashMap<ResponseSet, Vec<Word>>) {
    map.clear();

    for answer in cluster {
        let mut responses = start.clone();

        for guess in guesses {
            let response = Response::score(*guess, *answer);
            responses.push(response);
        }

        map.entry(responses).or_insert(Vec::new()).push(*answer);
    }
}

/// Split a cluster given a next guess - return a map of each distinct response and the answers which get that response.
pub fn split_map(map: &HashMap<ResponseSet, Vec<Word>>, guess: Word, over_count: usize, next_map: &mut HashMap<ResponseSet, Vec<Word>>) -> usize {
    let mut excluded_count = 0;

    next_map.clear();
    for (responses, answers) in map.iter() {
        if answers.len() <= over_count {
            excluded_count += answers.len();
            continue;
        }

        for answer in answers.iter() {
            if guess == *answer { 
                excluded_count += 1;
                continue; 
            
            }
            let mut inner_key = responses.clone();
            let response = Response::score(guess, *answer);
            inner_key.push(response);

            next_map.entry(inner_key).or_insert(Vec::new()).push(*answer);
        }
    }

    excluded_count
}

/// Find all responses for a given next guess and the number of answers in each
pub fn counts(cluster: &Vec<Word>, guess: Word, map: &mut HashMap<Response, usize>) {
    map.clear();

    for answer in cluster.iter() {
        if guess == *answer { continue; }
        let response = Response::score(guess, *answer);
        let entry = map.entry(response);
        *entry.or_insert(0) += 1;
    }
}

/// Identify whether a cluster is "safe"; every word in a safe cluster will fully distinguish every other word.
/// This means any random word in the cluster can be guessed next.
pub fn is_safe_cluster(cluster: &Vec<Word>) -> bool {
    if cluster.len() < 3 { return true; }

    let mut map: HashSet<Response> = HashSet::new();

    for guess in cluster.iter() {
        map.clear();
        for answer in cluster.iter() {
            if guess == answer { continue; }
            let response = Response::score(*guess, *answer);

            if map.insert(response) == false {
                return false;
            }
        }
    }

    true
}

/// Compute predicted total turns.
pub fn total_turns_predicted_map<T>(map: &HashMap<T, Vec<Word>>) -> usize {
    ClusterVector::from_map(map).total_turns_predicted()
}

/// Compute pessimistic total turns. (Guess randomly and assume no information from responses)
pub fn total_turns_pessimistic_map<T>(map: &HashMap<T, Vec<Word>>) -> usize {
    ClusterVector::from_map(map).total_turns_pessimistic()
}

/// Compute ideal total turns. (Assume every cluster has one guess which will be correct or distinguish every other answer)
pub fn total_turns_ideal_map<T>(map: &HashMap<T, Vec<Word>>) -> usize {
    ClusterVector::from_map(map).total_turns_ideal()
}

pub fn total_turns_random(cluster: &Vec<Word>) -> f64 {
    if cluster.len() == 1 {
        // Singles will be guessed the next turn
        1.0
    } else if cluster.len() == 2 {
        // A pair will take 1 + 2 = 3 total turns to guess.
        3.0
    } else {
        let mut total = 0.0;

        // For larger clusters, consider each possible next guess
        let mut map: HashMap<Response, Vec<Word>> = HashMap::new();

        for guess in cluster.iter() {
            // Build a map of the clusters after this next guess
            split(cluster, *guess, &mut map);

            // Add up the remaining turns needed for each subcluster
            for (_, subcluster) in map.iter() {
                total += total_turns_random(subcluster);
            }
        }

        // Turns is one per item (it might be guessed first) plus the each subcluster total times the odds the subcluster occurs 
        //  (only when 'guess' from 'cluster' was the next guess)
        (cluster.len() as f64) + (total / (cluster.len() as f64))
    }
}

pub fn total_turns_random_map<T>(clusters: &HashMap<T, Vec<Word>>) -> usize {
    total_turns_random_map_exact(clusters) as usize
}

pub fn total_turns_random_map_exact<T>(clusters: &HashMap<T, Vec<Word>>) -> f64 {
    let mut total = 0.0;

    for (_, cluster) in clusters.iter() {
        total += total_turns_random(cluster);
    }

    total
}

pub fn total_turns_perfect_map<T>(clusters: &HashMap<T, Vec<Word>>, valid: &Vec<Word>) -> usize {
    let mut strategy = Vec::new();
    total_turns_perfect_map_exact(clusters, valid, Some(&mut strategy)) as usize
}

pub fn total_turns_perfect_map_exact<T>(clusters: &HashMap<T, Vec<Word>>, valid: &Vec<Word>, strategy: Option<&mut Vec<(f64, usize, Word, Word)>>) -> f64 {
    let mut total = 0.0;

    if let Some(mut strat) = strategy {
        for (_, cluster) in clusters.iter() {
            total += total_turns_perfect(cluster, valid, Some(&mut strat));
        }
    } else {
        for (_, cluster) in clusters.iter() {
            total += total_turns_perfect(cluster, valid, None);
        }
    }

    total
}

pub fn total_turns_perfect(cluster: &Vec<Word>, valid: &Vec<Word>, strategy: Option<&mut Vec<(f64, usize, Word, Word)>>) -> f64 {
    let length = cluster.len();

    if length == 1 {
        // Singles will be guessed the next turn
        1.0
    } else if length == 2 {
        // A pair will take 1 + 2 = 3 total turns to guess.
        3.0
    } else {
        let mut options = Vec::new();

        let mut best_strategy: Vec<(f64, usize, Word, Word)> = Vec::new();

        let mut map: HashMap<Response, Vec<Word>> = HashMap::new();
        let mut count_map = HashMap::new();
        let mut current_strategy: Vec<(f64, usize, Word, Word)> = Vec::new();

        // Consider each in-cluster option
        let _in_cluster_ideal_turns = length - 1;
        let mut worst_in_cluster_ideal: Option<usize> = None;
        let mut best_in_cluster_ideal: Option<usize> = None;

        for guess in cluster.iter() {
            counts(cluster, *guess, &mut count_map);
            let score = ClusterVector::from_counts(&count_map).total_turns_ideal();

            if best_in_cluster_ideal == None || score < best_in_cluster_ideal.unwrap() {
                best_in_cluster_ideal = Some(score);
            }

            if worst_in_cluster_ideal == None || score > worst_in_cluster_ideal.unwrap() {
                worst_in_cluster_ideal = Some(score);
            }

            // ISSUE: We sometimes don't choose the alphabetically first best guess with this.
            // It seems like the order of words in the cluster isn't staying alphebetical?

            // if score == _in_cluster_ideal_turns && worst_in_cluster_ideal.unwrap() > best_in_cluster_ideal.unwrap() {
            //     options.clear();
            //     options.push((score, *guess, true));
            //     break;
            // }

            options.push((score, *guess, true));
        }

        // If the best in-cluster choice takes more than a perfect out-of-cluster choice, consider out-of-cluster options.
        //  Best case is the out-of-cluster guess and then the answer for each answer, so a total of 2 x cluster_size turns.
        let out_of_cluster_ideal = length;
        if length > 3 && best_in_cluster_ideal.unwrap() > out_of_cluster_ideal {
            let interesting_letters = word::interesting_letters(cluster);
            let terrible = 2 * length - 1;

            for guess in valid.iter() {
                if guess.letters_in_word() | interesting_letters == 0 { continue; }

                counts(cluster, *guess, &mut count_map);
                let score = ClusterVector::from_counts(&count_map).total_turns_ideal();

                // Terrible guesses leave every answer in the same cluster, and would cause infinite recursion
                if score == terrible { continue; }

                options.push((score, *guess, false));

                // Stop if an ideal split was found (everything to a single)
                if score <= out_of_cluster_ideal { 
                    break; 
                }
            }
        }

        // Sort by ideal turns ascending, in-cluster first, then alphabetical order
        // ISSUE: Not working right. 
        options.sort_by(|l, r| l.0.cmp(&r.0)
            .then_with(|| r.2.cmp(&l.2))
            .then_with(|| l.1.cmp(&r.1)));

        let mut best: Option<(f64, Word, bool)> = None;

        for (i, (score, guess, in_cluster)) in options.iter().enumerate() {
            // Stop when remaining option ideal turns are worse than actual turns found
            if let Some(best) = best {
                if i > 0 && *score as f64 >= best.0 { break; }
            }

            // Compute actual turns for this guess
            split(cluster, *guess, &mut map);
            current_strategy.clear();
            let turns = total_turns_perfect_map_exact(&map, &valid, Some(&mut current_strategy));

            // Keep the best guess found
            if best == None || turns < best.unwrap().0 {
                best = Some((turns, *guess, *in_cluster));
                mem::swap(&mut best_strategy, &mut current_strategy);
            }
        }

        // The total turns is one for the next turn (for every answer) plus turns after this guess
        let inner_total = (length as f64) + best.unwrap().0;

        // If any in-cluster option wasn't perfect, add a strategy for what we chose
        if best.unwrap().0 < worst_in_cluster_ideal.unwrap() as f64 || best.unwrap().2 == false {
            if let Some(strategy) = strategy {
                strategy.push((best.unwrap().0, length, cluster[0], best.unwrap().1));
                strategy.append(&mut best_strategy);
            }
        }
        
        inner_total
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use crate::{word::Word, response::{Response, ResponseSet}};

    #[test]
    fn total_turns_random() {
        // Base cases: 1 for 1- cluster, 3 for 2-cluster
        assert_eq!(super::total_turns_random(&vec![w("aaaaa")]), 1.0);
        assert_eq!(super::total_turns_random(&vec![w("aaaaa"), w("bbbbb")]), 3.0);

        // Three cluster where no guess distinguishes the others
        //  Three next options. In each case, solving takes 1 + 2 + 3 turns across the three answers.
        //  So, 3 * (1 + 2 + 3) = 18 turns / 3 options = 6 total turns to solve on average.
        assert_eq!(super::total_turns_random(&vec![w("clash"), w("clasp"), w("class")]), 6.0);

        // Three cluster where every guess fully identifies the others
        //  Three next options. In each case, solving takes 1 + 2 + 2 turns across the three answers.
        //  So, 3 * (1 + 2 + 2) = 15 turns / 3 options = 5 total turns to solve on average.
        assert_eq!(super::total_turns_random(&vec![w("purse"), w("reuse"), w("verse")]), 5.0);

        // Four words that all distinguish each other alternative
        //  4 * (1 + 2 + 2 + 2) = 28 / 4 options = 7 total turns to solve on average.
        assert_eq!(super::total_turns_random(&vec![w("belie"), w("bible"), w("bilge"), w("liege")]), 7.0);

        // Four options.
        //   booze  6	[0, 0, 1]
        // * dodge  3	[3]
        // * gouge  3	[3]
        // * vogue  3	[3]
        //  Three of them distinguish the others and each take 7 turns. (1 + 2 + 2 + 2)
        //  Booze takes one turn when right, and one turn plus the "three fully identifies" otherwise
        //   (1 + (3 * (1 + 5) / 3) = 9 turns;
        //  So 3 * 7 + 1 * 9 = 30 / 4 options = 7.5 total on average.
        assert_eq!(super::total_turns_random(&vec![w("booze"), w("dodge"), w("gouge"), w("vogue")]), 7.5);
    }

    #[test]
    fn total_turns_perfect() {
        let valid = vec![w("crane"), w("spilt"), w("dumpy")];
        let mut strategy = Vec::new();

        // Base cases: 1 for 1- cluster, 3 for 2-cluster
        assert_eq!(super::total_turns_perfect(&vec![w("aaaaa")], &valid, Some(&mut strategy)), 1.0);
        assert_eq!(super::total_turns_perfect(&vec![w("aaaaa"), w("bbbbb")], &valid, Some(&mut strategy)), 3.0);

        // Three cluster where no guess distinguishes the others
        //  Three next options. In each case, solving takes 1 + 2 + 3 turns across the three answers.
        //  So, 3 * (1 + 2 + 3) = 18 turns / 3 options = 6 total turns to solve on average.
        assert_eq!(super::total_turns_perfect(&vec![w("clash"), w("clasp"), w("class")], &valid, Some(&mut strategy)), 6.0);

        // Three cluster where every guess fully identifies the others
        //  Three next options. In each case, solving takes 1 + 2 + 2 turns across the three answers.
        //  So, 3 * (1 + 2 + 2) = 15 turns / 3 options = 5 total turns to solve on average.
        assert_eq!(super::total_turns_perfect(&vec![w("purse"), w("reuse"), w("verse")], &valid, Some(&mut strategy)), 5.0);
        assert_eq!(strategy.len(), 0);

        // Three cluster where one fully identifies the others
        //  sissy or missy identify the other, but kiosk can't distinguish
        assert_eq!(super::total_turns_perfect(&vec![w("kiosk"), w("sissy"), w("missy")], &valid, Some(&mut strategy)), 5.0);

        // Verify "missy" recommended for "kiosk", costing two more turns after guess
        assert_eq!(strategy, vec![(2.0, 3, w("kiosk"), w("missy"))]);
        strategy.clear();

        // Four words that all distinguish each other alternative
        //  4 * (1 + 2 + 2 + 2) = 28 / 4 options = 7 total turns to solve on average.
        assert_eq!(super::total_turns_perfect(&vec![w("belie"), w("bible"), w("bilge"), w("liege")], &valid, Some(&mut strategy)), 7.0);
        assert_eq!(strategy.len(), 0);

        // Four options, at least one ideal
        //   booze  6	[0, 0, 1]
        // * dodge  3	[3]
        // * gouge  3	[3]
        // * vogue  3	[3]
        // (1 + 2 + 2 + 2) = 7.0 turns for best vs. (1 + (2 + 3 + 3)) = 9.0 turns worst
        assert_eq!(super::total_turns_perfect(&vec![w("booze"), w("dodge"), w("gouge"), w("vogue")], &valid, Some(&mut strategy)), 7.0);

        // Verify "dodge" recommended for "booze"
        assert_eq!(strategy, vec![(3.0, 4, w("booze"), w("dodge"))]);
        strategy.clear();

        // Four options where an out-of-cluster choice is best
        //  x dumpy -> 2 + 2 + 2 + 2 = 8.0 (vs 1 + 2 + 3 + 4 = 10.0)
        assert_eq!(super::total_turns_perfect(&vec![w("ddddd"), w("uuuuu"), w("mmmmm"), w("ppppp")], &valid, Some(&mut strategy)), 8.0);
        assert_eq!(strategy, vec![(4.0, 4, w("ddddd"), w("dumpy"))]);
        strategy.clear();

        // Four options with no ideal choices
        //  x dumpy -> 2 + 2 + 2 + 3 = 9.0
        assert_eq!(super::total_turns_perfect(&vec![w("ddddd"), w("uuuuu"), w("xxxxx"), w("zzzzz")], &valid, Some(&mut strategy)), 9.0);
        assert_eq!(strategy, vec![(5.0, 4, w("ddddd"), w("dumpy"))]);
        strategy.clear();

        // Four options with all awful choices
        //  1 + 2 + 3 + 4 = 10.0
        assert_eq!(super::total_turns_perfect(&vec![w("bbbbb"), w("ddddd"), w("fffff"), w("ggggg")], &valid, Some(&mut strategy)), 10.0);
        assert_eq!(strategy.len(), 0);

        assert_eq!(strategy.len(), 0);

    }

    #[test]
    fn split() {
        let cluster = vec![w("scamp"), w("shack"), w("smack")];
        let mut map = HashMap::new();
        super::split(&cluster, w("dumpy"), &mut map);
        assert_eq!(map.len(), 3);
        assert_eq!(map[&r("bbyyb")], vec![w("scamp")]);
        assert_eq!(map[&r("bbbbb")], vec![w("shack")]);
        assert_eq!(map[&r("bbybb")], vec![w("smack")]);

        let cluster = vec![w("syrup"), w("shrub"), w("shrug")];
        super::split(&cluster, w("dumpy"), &mut map);
        assert_eq!(map.len(), 2);
        assert_eq!(map[&r("bybyy")], vec![w("syrup")]);
        assert_eq!(map[&r("bybbb")], vec![w("shrub"), w("shrug")]);
    }

    #[test]
    fn split_all() {
        let cluster = vec![w("cable"), w("cause"), w("cache"), w("caste"), w("canoe")];
        let mut start = ResponseSet::new();
        start.push(Response::score(w("carve"), *cluster.first().unwrap()));

        let mut map = HashMap::new();
        super::split_all(&cluster, &vec![w("downy"), w("plumb")], start, &mut map);
        assert_eq!(map.len(), 4);
        assert_eq!(map[&rs(vec!["GGbbG", "bbbbb", "bbbbb"])], vec![w("cache"), w("caste")]);
        assert_eq!(map[&rs(vec!["GGbbG", "bbbbb", "bbGbb"])], vec![w("cause")]);
        assert_eq!(map[&rs(vec!["GGbbG", "bbbbb", "bYbbY"])], vec![w("cable")]);
        assert_eq!(map[&rs(vec!["GGbbG", "bYbYb", "bbbbb"])], vec![w("canoe")]);
    }

    #[test]
    fn cluster_safe() {
        // ..ack words can distinguish each other from scamp. Scamp has 'm' to distinguish sMack from shack
        let cluster = vec![w("scamp"), w("shack"), w("smack")];
        assert_eq!(super::is_safe_cluster(&cluster), true);

        // syrup does not have 'b' or 'g', so can't distinguish "shrub" from "shrug"
        let cluster = vec![w("syrup"), w("shrub"), w("shrug")];
        assert_eq!(super::is_safe_cluster(&cluster), false);

        // Base Case: All clusters under size three are safe (guess any one and one or fewer will be left)
        let cluster = vec![w("shrub"), w("shrug")];
        assert_eq!(super::is_safe_cluster(&cluster), true);
    }

    fn w(text: &str) -> Word {
        Word::new(text).unwrap()
    }

    fn r(text: &str) -> Response {
        Response::from_str(text).unwrap()
    }

    fn rs(parts: Vec<&str>) -> ResponseSet {
        let mut result = ResponseSet::new();

        for part in parts {
            result.push(r(part));
        }

        result
    }
}