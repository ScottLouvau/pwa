use std::collections::HashMap;
use rand::seq::SliceRandom;
use crate::{word::Word, response::{Response, Constraint}, rank, cluster_vector::ClusterVector, wv_safe, wordle_tree::tree_player::TreePlayer, clubs::Clubs};

pub fn simulate(answers: &Vec<Word>, game_answer_pool: &Vec<Word>, guesses: &Vec<Word>, game_count: usize, strategy: &mut dyn FnMut(&Vec<Word>, usize, &Vec<Word>) -> Option<Word>, print: bool) -> f64 {
    // Use a faster implementation if we're only considering one answer repeatedly
    if game_answer_pool.len() == 1 {
        return simulate_single(answers, game_answer_pool, game_answer_pool[0], game_count, strategy);
    }

    let mut rng = rand::thread_rng();
    let mut total_turns = 0;

    for _game in 0..game_count {
        // Choose an answer
        let answer = if game_count < game_answer_pool.len() * 2 {
            *game_answer_pool.choose(&mut rng).unwrap()
        } else {
            game_answer_pool[_game % game_answer_pool.len()]
        };
        //let _a = answer.to_string();

        // Simulate the game
        let mut turn: usize = 0;
        let mut answers_left = answers.clone();
        let mut constraint = Constraint::new();
    
        if print {
            println!("=== {} ===", answer.to_string().to_ascii_uppercase());
        }
    
        loop {
            turn += 1;

            // Choose a guess
            let guess = strategy(guesses, turn, &answers_left).unwrap_or_else(|| *answers_left.choose(&mut rng).unwrap());
            
            // Score
            let response = Response::score(guess, answer);
            constraint.add(guess, response);
    
            // Filter remaining answers
            answers_left.retain(|a| constraint.matches(*a) && Response::score(guess, *a) == response);
            let count_left = answers_left.len();
    
            if print { 
                print!("{turn}) {guess}: {response} -> {count_left}");
                if count_left < 10 { 
                    println!("     {:?}", answers_left); 
                } else {
                    println!();
                }
            }
    
            if guess == answer || answers_left.len() == 0 { break; }
        }
    
        if print { println!(); }
        total_turns += turn;
    }

    let average_turns = (total_turns as f64) / (game_count as f64);
    let total_turns_est = average_turns * (answers.len() as f64);
    
    if print {
        println!("{total_turns} / {game_count} = {average_turns:.4} ({total_turns_est:.0}) turns per game.");
    }

    average_turns
}

/// Simulate a single game many times with the same strategy
pub fn simulate_single(answers: &Vec<Word>, valid: &Vec<Word>, answer: Word, game_count: usize, strategy: &mut dyn FnMut(&Vec<Word>, usize, &Vec<Word>) -> Option<Word>) -> f64 {
    // Play the fixed strategy part of the game once
    let mut from_turn = 1;
    let mut from_answers = answers.clone();
    
    while let Some(guess) = strategy(valid, from_turn, &from_answers) {
        //let _guess_text = guess.to_string();

        if guess == answer {
            from_answers.clear();
            break;
        } else {
            // Score guess against answer
            let response = Response::score(guess, answer);
    
            // Filter remaining answers
            from_answers.retain(|a| Response::score(guess, *a) == response);
        }

        if from_answers.len() == 0 { break; }
        from_turn += 1;
    }

    // If planned guesses fully solve this game, return the turn count
    if from_answers.len() == 0 {
        return from_turn as f64;
    }

    let mut rng = rand::thread_rng();
    let mut total_turns = 0;

    // Simulate the random part of each game
    for _game in 0..game_count {
        let mut turn: usize = from_turn - 1;
        let mut answers_left = from_answers.clone();
        let mut constraint = Constraint::new();
    
        loop {
            turn += 1;

            // Choose a guess
            let guess = *answers_left.choose(&mut rng).unwrap();
            if guess == answer { break; }
            
            // Score
            let response = Response::score(guess, answer);
            constraint.add(guess, response);
    
            // Filter remaining answers
            answers_left.retain(|a| constraint.matches(*a) && Response::score(guess, *a) == response);
    
            if answers_left.len() == 0 { break; }
        }

        total_turns += turn;
    }

    (total_turns as f64) / (game_count as f64)
}

pub fn get_strategy(name: &str) -> fn(&Vec<Word>, usize, &Vec<Word>) -> Option<Word> {
    match name {
        "random"         => choose_standard,
        "hybrid_random"  => hybrid_random,
        "hybrid_best"    => hybrid_best,
        "pessimistic"    => choose_best_pessimistic,
        "predicted"      => choose_best_predicted,
        _ => panic!("Strategy '{}' not found.", name)
    }
}

// Use all standard guesses, then guess randomly
pub fn choose_standard(guesses: &Vec<Word>, turn: usize, _answers_left: &Vec<Word>) -> Option<Word> {
    guesses.get(turn - 1).copied()
}

// Guess whenever three or fewer options, otherwise standard guesses until gone
pub fn hybrid_random(guesses: &Vec<Word>, turn: usize, answers_left: &Vec<Word>) -> Option<Word> {
    if answers_left.len() < 4 { None } else { guesses.get(turn - 1).copied() }
}

// Use two standard guesses, then guess clusters < 4, then other standards, then guess
pub fn hybrid_best(guesses: &Vec<Word>, turn: usize, answers_left: &Vec<Word>) -> Option<Word> {
    let left = answers_left.len();

    /*if guesses.len() > turn {
        guesses.get(turn - 1).copied()
    } else*/ if left <= 2 {
        None
    } else if left <= 3 {
        choose_best_pessimistic(&Vec::new(), turn, answers_left)
    } else if guesses.len() >= turn {
        guesses.get(turn - 1).copied()
    } else {
        //choose_best_pessimistic(&Vec::new(), turn, answers_left)
        None
    }
}

// Use all standard guesses, then the best in-cluster given a ranking function
pub fn choose_best(guesses: &Vec<Word>, turn: usize, answers_left: &Vec<Word>, ranker: fn(&HashMap<Response, Vec<Word>>) -> usize) -> Option<Word> {
    if let Some(next) = guesses.get(turn - 1) {
        return Some(*next);
    } else if answers_left.len() <= 2 {
        None
    } else {
        let mut map = HashMap::new();
        let mut best: Option<(usize, Word)> = None;
        let mut worst: Option<(usize, Word)> = None;
        //let _cluster = format!("{answers_left:?}");

        for option in answers_left {
            rank::split(answers_left, *option, &mut map);
            let score = ranker(&map);

            if let Some(best_set) = &best {
                if score < best_set.0 {
                    best = Some((score, *option));
                }
            } else {
                best = Some((score, *option));
            }

            if let Some(worst_set) = &worst {
                if score > worst_set.0 {
                    worst = Some((score, *option));
                }
            } else {
                worst = Some((score, *option));
            }
        }

        if worst.unwrap().0 > best.unwrap().0 {
            //let _choice = best.unwrap().1.to_string();
            return Some(best.unwrap().1);
        } else {
            return None;
        }
        //return best.map(|b| b.1);
    }
}

pub fn choose_best_pessimistic(guesses: &Vec<Word>, turn: usize, answers_left: &Vec<Word>) -> Option<Word> {
    choose_best(guesses, turn, answers_left, rank::total_turns_pessimistic_map)
}

pub fn choose_best_predicted(guesses: &Vec<Word>, turn: usize, answers_left: &Vec<Word>) -> Option<Word> {
    choose_best(guesses, turn, answers_left, rank::total_turns_predicted_map)
}

pub fn assess_and_simulate(guesses: Option<&str>, valid: &Vec<Word>, answers: &Vec<Word>, simulate_game_count: usize, mut player: TreePlayer) -> Result<String, String> {
    let guesses = guesses.ok_or("Must provide guesses")?;
    let guesses = wv_safe(&guesses)?;
    let answer = *guesses.last().ok_or("Must have one or more guesses")?;

    if !answers.contains(&answer) {
        return Err(format!("{answer} isn't a Wordle answer!"));
    }

    let mut output = String::new();
    output += &assess(answer, guesses, valid, answers.clone(), &mut player);

    let simulate_answers = vec![answer];
    let turns = simulate(&answers, &simulate_answers, &Vec::new(), simulate_game_count, &mut |g, t, a| player.choose(g, t, a), false);

    output += "\n\n";
    output += &format!("=> {turns:.3} avg turns ({answer} x{simulate_game_count})\n\n");

    output += "* = best in-cluster guesses\n";
    output += "x = best out-of-cluster guess\n";
    output += "s = strategy guess\n";
    output += "> = actual guess\n";

    Ok(output)
}

/// Assess a Game.
///  PURPOSE: Show how play compared to optimal choices.
///   - Did I pivot to guessing at the right turn?
///   - Did I identify all possible answers properly?
///   - Was the guess optimal? Was the standard one optimal?
/// 
///  OUTPUT:
///  - Show each guess, response, and remaining answer count.
///  - If few enough answers are left, ...
///     - Show each answer with expected total turns if chosen and clusters.
///     - Sort with the best choices last.
///     - Ties for best marked with '*'.
///     - Next move (if in cluster) marked with '>'.
/// 
///  - Empty line between in-cluster options and any others shown.
///  - Show best out-of-cluster guess if no in-cluster choices were optimal.
///  - Show next standard guess marked with 's'.
///  - Show actual next guess marked with '>'.
pub fn assess(answer: Word, guesses: Vec<Word>, valid: &Vec<Word>, mut answers_left: Vec<Word>, player: &mut TreePlayer) -> String {
    let mut result = String::new();
    let mut turns = 0;

    result += &format!("=== {} ===", answer.to_string().to_ascii_uppercase());

    // Reset TreePlayer to start of game (it needs to know where in the tree to search)
    player.choose(&guesses, 1, &answers_left);

    for (i, guess) in guesses.iter().enumerate() {
        let _g = guess.to_string();
        turns += 1;
        
        // Score the guess
        let response = Response::score(*guess, answer);

        // Filter remaining answers
        answers_left.retain(|a| Response::score(*guess, *a) == response);
        let count_left = answers_left.len();

        // Show turn #, guess, response, and answer count left
        result += &format!("\n{turns}) {guess}: {response} -> {count_left}\n");

        // Stop here if we solved it
        if *guess == answer { break; }

        // If there are few enough answers left, list and analyze them
        if let Some(next_guess) = guesses.get(i + 1) {
            let mut need_newline = false;
            let mut next_guess_shown = false;

            if count_left < 30 {
                // Compute average turns remaining for each answer remaining answer (best last)
                let ranked = rank_all_cluster(&answers_left, &answers_left);
                let best_score = ranked.last().unwrap().0;

                // Since we're showing in-cluster options, make sure to add a newline before other choices
                need_newline = true;

                // Write all in-cluster words with expected total game turns if chosen.
                for (score, choice, cv) in ranked.iter() {
                    // * for ties for best option
                    // > for actual next guess
                    let mut mark = if *score == best_score { "*" } else { " " };
                    if next_guess == choice { mark = ">"; next_guess_shown = true; }

                    result += &format!("  {} {}  {:.2}  {}\n", mark, choice, (turns as f64) + 1.0 + score, cv.to_string());
                }

                // If no in-cluster guess is ideal, also show the best possible guess from all valid words
                // (An ideal in-cluster guess will have a "turns left" under 1.0; zero when it's the answer and one for everything else.)
                // (An ideal out-of-cluster guess has a best turns left of 1.0, so if the in-cluster best is one, out-of-cluster won't be better.)
                if best_score > 1.0 {
                    let (score, choice, cv) = best(valid, &answers_left);
                    if score < best_score {
                        if need_newline { need_newline = false; result += "\n"; }
                        result += &format!("  x {}  {:.2}  {}\n", choice, (turns as f64) + 1.0 + score, &cv.to_string());
                    }
                }
            }

            // If the next standard guess wasn't used, also show the outcome for it
            let strategy_next = player.choose(&guesses, turns + 1, &answers_left);
            if let Some(next_standard) = strategy_next {
                if next_standard != *next_guess {
                    if need_newline { need_newline = false; result += "\n"; }
                    let (score, choice, cv) = rank_cluster(next_standard, &answers_left);
                    result += &format!("  s {}  {:.2}  {}\n", choice, (turns as f64) + 1.0 + score, cv.to_string());
                }
            }

            // If the next guess wasn't shown, also show the outcome for it
            if !next_guess_shown {
                if need_newline { /*need_newline = false;*/ result += "\n"; }
                let (score, choice, cv) = rank_cluster(*next_guess, &answers_left);
                result += &format!("  > {}  {:.2}  {}\n", choice, (turns as f64) + 1.0 + score, cv.to_string());
            }
        }
    }

    result
}

fn rank_cluster(guess: Word, cluster: &Vec<Word>) -> (f64, Word, ClusterVector) {
    let mut map = HashMap::new();
    rank::split(cluster, guess, &mut map);

    let _g = guess.to_string();
    let cv = ClusterVector::from_map(&map);
    let score = rank::total_turns_random_map_exact(&map);
    let average_turns = (score as f64) / (cluster.len() as f64);
    
    (average_turns, guess, cv)
}

fn rank_all_cluster(guess_options: &Vec<Word>, answers: &Vec<Word>) -> Vec<(f64, Word, ClusterVector)> {
    let mut result;

    if answers.len() > 64 {
        result = answers.iter().map(|a| rank_cluster(*a, &answers)).collect::<Vec<_>>();
    } else {
        result = Vec::new();

        let clubs = Clubs::new(answers, answers);
        let within = clubs.all_vector();
        let mut cache = HashMap::new();

        for guess in guess_options {
            let total_turns = clubs.count_random_turns_after_cache(within, *guess, &mut cache);
            let average_turns = ((total_turns as f64) / (answers.len() as f64)) - 1.0;
            let cv = clubs.cluster_vector(within, *guess);
            result.push((average_turns, *guess, cv));
        }
    }

    result.sort_by(|l, r| r.0.total_cmp(&l.0));
    result
}

fn best(guess_options: &Vec<Word>, cluster: &Vec<Word>) -> (f64, Word, ClusterVector) {
    rank_all_cluster(guess_options, cluster).last().unwrap().clone()
}

#[cfg(test)]
mod tests {
    use crate::{word::Word, wordle_tree};

    #[test]
    fn rank_cluster() {
        let mut rank;
        
        rank = super::rank_cluster(w("sheck"), &vec![w("crane"), w("crack"), w("crash"), w("crost"), w("crunk")]);
        assert_eq!(rank.0, 1.0); // (1 + 1 + 1 + 1 + 1) / 5

        rank = super::rank_cluster(w("crane"), &vec![w("crane"), w("crack"), w("crash"), w("crost"), w("crunk")]);
        assert_eq!(rank.0, 1.0); // (0 + 1 + 1 + 3) / 5

        rank = super::rank_cluster(w("crane"), &vec![w("xxxxx"), w("yyyyy"), w("zzzzz")]);
        assert_eq!(rank.0, 2.0); // (1 + 2 + 3) / 3

        rank = super::rank_cluster(w("crane"), &vec![w("wwwww"), w("xxxxx"), w("yyyyy"), w("zzzzz")]);
        assert_eq!(rank.0, 2.5); // (1 + 2 + 3 + 4) / 4

        rank = super::rank_cluster(w("xxxxx"), &vec![w("crane"), w("crash"), w("crack")]);
        assert_eq!(rank.0, 2.0); // (1 + 2 + 3) / 3
    }

    #[test]
    fn assess() {
        let answers = vec![w("crane"), w("crack"), w("crash"), w("crost"), w("crunk"), w("dowry"), w("sheck")]; //Word::parse_file(Path::new("data/2315/answers.txt"));
        let valid = answers.clone();

        let standard = vec![w("crane"), w("spilt"), w("dumbo")];
        let tree = wordle_tree::builders::build("standard", &answers, &standard);
        let mut player = wordle_tree::tree_player::TreePlayer::new(&tree);

        let mut result;

        // One possible, found on first try: Heading and victory turn
        result = super::assess(w("crane"), vec![w("crane")], &valid, answers.clone(), &mut player);
        assert_eq!(result, 
"=== CRANE ===
1) crane: 🟩🟩🟩🟩🟩 -> 1
");

        // Two possible, standard guess, then non-standard guess: 
        //  - Show each option with expected turns (2 or 3 -> 2.50)
        //  - If next guess is standard guess, show only once as '>'
        //  - If next guess is not standard guess, show both (turn two)
        result = super::assess(w("crash"), vec![w("crane"), w("spilt"), w("crash")], &valid, answers.clone(), &mut player);
        assert_eq!(result,
"=== CRASH ===
1) crane: 🟩🟩🟩⬛⬛ -> 2
  * crack  2.50  [1]
  * crash  2.50  [1]

  > spilt  3.00  [2]

2) spilt: 🟨⬛⬛⬛⬛ -> 1
  > crash  3.00  

  s dumbo  4.00  [1]

3) crash: 🟩🟩🟩🟩🟩 -> 1
");

        // Better out-of-cluster guess
        //  - Show higher expected from any in-cluster guess
        //  - Show out-of-cluster guess with 'x'
        result = super::assess(w("yyyyy"), vec![w("crane"), w("spilt"), w("waxys"), w("yyyyy")], &vec![w("waxys")], vec![w("wwwww"), w("xxxxx"), w("yyyyy"), w("zzzzz")], &mut player);
        assert_eq!(result,
"=== YYYYY ===
1) crane: ⬛⬛⬛⬛⬛ -> 4
  * wwwww  3.50  [0, 0, 1]
  * xxxxx  3.50  [0, 0, 1]
  * yyyyy  3.50  [0, 0, 1]
  * zzzzz  3.50  [0, 0, 1]

  x waxys  3.00  [4]
  > spilt  4.50  [0, 0, 0, 1]

2) spilt: ⬛⬛⬛⬛⬛ -> 4
  * wwwww  4.50  [0, 0, 1]
  * xxxxx  4.50  [0, 0, 1]
  * yyyyy  4.50  [0, 0, 1]
  * zzzzz  4.50  [0, 0, 1]

  x waxys  4.00  [4]
  s dumbo  5.50  [0, 0, 0, 1]
  > waxys  4.00  [4]

3) waxys: ⬛⬛⬛🟩⬛ -> 1
  > yyyyy  4.00  

4) yyyyy: 🟩🟩🟩🟩🟩 -> 1
");

    }

    fn w(text: &str) -> Word {
        Word::new(text).unwrap()
    }
}