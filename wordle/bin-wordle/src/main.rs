use std::{time::Instant, env, path::Path, fs};
use lib_wordle::{clubs::Clubs, letter_orders::LetterOrders, response::Response, scrappy, single_use::NeighborSide, word::Word, wordle_tree::{tree_player, WordleTree, WordleTreeToStringOptions}, *};

/*
   I used this code to improve my Wordle play.
   'assess' each day to see how my play compared to my planned strategy and the best possible moves for the situation.
   
   'search' to look for optimized starting guesses.
   'build' to generate strategy files describing how I plan to play.
     Add code to wordle_tree/builders.rs to generate trees for different flexible strategies.
   'simulate' to test how strategy files perform across all answers.
     Strategy files can be edited manually to add specific guessed in particular cases; simulate can test those variations.
   'best' and 'best_all' to look for the best words for specific situations.

   Overall, I wanted a high performing strategy that a human could really play - something without too much memorization,
   and which didn't require me to perfectly find all possible words in a any situation.

   This code allows experimenting with strategies with different levels of complexity to see how they should perform.
*/

// Modes
// =====
const USAGE: &str = "Usage: wordle_v2 <mode> [--set <wordSet>]? <args>...
 assess <strategyPath> <guessesIncludingAnswer>
   ex: assess ../data/v12.txt CLINT SOARE ELATE PLATE
   Assess play compared to a pre-planned strategy.
   Shows how many answers were left, the best guesses, and how the actual next guess compared.
   Simulates 10,000 games with the strategy for that answer to show expected turns to solve.

 analyze <guessesWithOptionalResponses>
   ex: analyze soare gbbby clint (SOARE with green, black, black, black, yellow, then show all possible responses for CLINT...)
   Shows remaining possible answers and the best next guesses.
   If some responses are omitted, will show all possible answers for that guess.
   If all responses are omitted, will show all of the outcomes for the guesses and the cluster of answers for each.

 build <strategy> <startingGuesses>
  Generate a strategy tree file given the strategy name and initial guesses.
  Strategies: 'standard', 'hybrid', 'best', 'first', 'v11'
  
 simulate <game_count> <strategyPath> [--games <answers_file_path> | --answer <single_answer> | --cluster <target_word> <at_turn>]? [--total]
  Simulate games using a strategy tree file. Can run for a specific answer or cluster only to check average turns for specific games.

 best            : For a strategy tree and in-cluster word, show best choices after the strategy.
 best_all        : For a set of standard guesses, show the how the best option for each cluster compares to the last guess.
 stats           : For a set of guesses, compute cluster vector, distribution vector (random, ideal, and no-lose-min), average turns, failure rate.
 explain_hybrid  : Show turns for perfect 'hybrid' play (after each guess, when there are 1-2 answers left, guess, otherwise, use next standard)
 
 search          : Find the best starting words after specific starting guesses by considering all possible guesses and scoring them. (expensive)

 score_answers   : Score answers by 'difficulty' (the sum of cluster size containing this answer across every possible guess)
 score_guesses   : Score each allowed guess by the number of clusters created (number of different tile responses)
 neighbors       : Show the most common neighbors for each letter (to help coming up with possible words when a particular letter is known)
 answer_stats    : Show letter frequency, first letter frequency, and repeated letter odds.
 ";

// perf            : Performance test the primary 'score' function (find tiles for a guess and answer pair)

// TODO:
//  - Separate strategy for continuation from print; figure out how to show the different outcomes.
//  - Change all "print" methods to build a string so that they can be tested and reused better.

fn main() {
    let args: Vec<String> = env::args().collect();
    let args = &args.iter().map(|s| s.as_str()).collect::<Vec<&str>>();
    let mut args: &[&str] = &args[1..];

    if args.len() < 1 {
        println!("{}", USAGE);
        return;
    }

    // Get mode from args
    let mode = args.first().unwrap().to_ascii_lowercase();
    let mode = mode.as_str();
    args = &args[1..];

    // See if word set is overridden; take args if so
    let mut set = "2315";
    if let Some(next) = args.first() {
        if *next == "--set" {
            if let Some(override_set) = args.get(1) {
                set = override_set;
                args = &args[2..];
            }
        }
    }

    let _answers = Word::parse_file(&Path::new(&format!("../data/{set}/answers.txt")));
    let mut _valid = Word::parse_file(&Path::new(&format!("../data/{set}/valid.txt")));

    let start = Instant::now();

    match mode {
        "best_all" => {
            // For a set of guesses, show how the last guess compares to the best choices for each cluster.
            let mut guesses = args.iter().map(|s| Word::new(s).unwrap()).collect::<Vec<Word>>();
            let next_guess = guesses.pop().unwrap();
            println!("{}", scrappy::recommended_strategy(&guesses, next_guess, &_answers, &_valid));
        }

        "best" => {
            // Pass a strategy path and a word in the cluster to explore
            // best ./data/v8.txt bobby
            // => SOARE, CLINT leave 16 options. DOUGH best, then (bobby, 5) -> BOOBY and (moody, 2) -> MOODY
            
            let strategy_path = args[0];
            let strategy_text = fs::read_to_string(&strategy_path).unwrap();
            let tree = WordleTree::parse(strategy_text.lines()).unwrap();
            let mut player = tree_player::TreePlayer::new(&tree);

            let answers_left = player.cluster(w(args[1]), &_answers, 6);
            println!("For ({}, {})", answers_left[0], answers_left.len());

            for (i, answer) in answers_left.iter().enumerate() {
                if i > 0 { print!(", ");}
                print!("{}", answer);
            }
            println!();

            let clubs = Clubs::new(&answers_left, &_valid); 
            let within = clubs.all_vector();

            let mut choices = std::collections::HashMap::new();
            let best = clubs.count_best_turns(within, &mut choices);

            let mut sentinel = WordleTree::new_sentinel();
            clubs.best_strategy(within, &choices, true, &mut sentinel);
            let mut tree = sentinel.take_first_child().unwrap();
            tree.outer_total_turns = best as f64;

            println!();
            println!("{}", tree.to_string());
        }

        "assess" => {
            // Convert args to comma delimited
            let strategy_path = args[0];
            let args = args[1..].iter().map(|s| s.to_string()).collect::<Vec<String>>();
            let guesses = args.join(",");

            let result = assess_inner(Some(&guesses), &_valid, &_answers, &strategy_path);
            match result {
                Ok(result) => println!("{}", result),
                Err(e) => println!("{}", e),
            }
        }

        "simulate" => {
            // Simulate games using a specific strategy tree file.
            // The files can be generated, manually created, or generated and then edited to craft a specific strategy.
            
            // Simulate can try all answers, a specific set of games, a single answer, or all words in a given cluster.
            // Simulating for a set of games estimates how different strategies would've done in a real life sequence of time.
            // Simulating for one answer shows average turns over many plays when random guesses are involved in the game.
            // Simulating for a cluster shows how the strategy performs in a particular cluster (and whether the "total turns" computed for it by build is accurate)
            const USAGE: &str = "Usage: wordle_v2 simulate <game_count> <tree_file_path> [--games <answers_file_path> | --answer <single_answer> | --cluster <target_word> <at_turn>]? [--total]";
            if args.len() < 2 {
                println!("Not enough arguments.\n{}", USAGE);
                return;
            }

            let game_count = args[0].parse::<usize>().unwrap();
            let print = game_count < 100;
            args = &args[1..];

            let strategy_path = args[0];
            args = &args[1..];
            let strategy_text = fs::read_to_string(&strategy_path).unwrap();
            let tree = WordleTree::parse(strategy_text.lines()).unwrap();
            let mut player = tree_player::TreePlayer::new(&tree);

            let mut game_answers = None;
            let mut show_average_turns = true;
            while args.len() > 0 {
                if args[0] == "--games" {
                    game_answers = Some(Word::parse_file(Path::new(args[1])));
                    args = &args[2..];
                } else if args[0] == "--answer" {
                    game_answers = Some(vec![Word::new(args[1]).unwrap()]);
                    args = &args[2..];
                } else if args[0] == "--cluster" {
                    let target_word = Word::new(args[1]).unwrap();
                    let at_turn = args[2].parse::<usize>().unwrap();
                    args = &args[3..];

                    game_answers = Some(player.cluster(target_word, &_answers, at_turn));
                    player.clear_play_stats();
                } else if args[0] == "--total" {
                    show_average_turns = false;
                    args = &args[1..];
                } else {
                    println!("Unrecognized argument '{}', {}", args[0], USAGE);
                    return;
                }
            }

            let game_answers = if let Some(game_answers) = &game_answers { &game_answers } else { &_answers };
            let answer_count = game_answers.len();
            let answer_description = if answer_count <= 10 { format!("{{{}}}", game_answers.iter().map(|w| w.to_string()).collect::<Vec<String>>().join(", ")) } else { format!("({}, {})", game_answers.first().unwrap(), answer_count) };

            println!("Simulating {game_count} games for {strategy_path} in {answer_description}:");
            let average_turns = check::simulate(&_answers, &game_answers, &Vec::new(), game_count, &mut |g, t, a| player.choose(g, t, a), print);

            let mut options = WordleTreeToStringOptions::default();
            options.show_average_turns = show_average_turns;
            options.show_zero_turn_paths = false;
            println!();
            println!("{}", player.to_string(game_answers.len(), &options));

            let total_turns = average_turns * answer_count as f64;
            println!();
            println!("{total_turns:.0} ({average_turns:.3})");
        }

        "build" => {
            let strategy = args[0].to_ascii_lowercase();
            let guesses = args[1..].iter().map(|s| Word::new(s).unwrap()).collect::<Vec<Word>>();
            let tree = wordle_tree::builders::build(&strategy, &_answers, &guesses);

            let mut options = WordleTreeToStringOptions::default();
            options.show_average_turns  = options.show_average_turns;

            let mut output = String::new();
            tree.add_to_string(&options, 0, &mut output);
            println!("{}", output);
        }

        "analyze" => {
            let guesses_and_responses = read_guesses_and_responses(args);
            let guesses_and_responses = analyze::parse_into_guesses_and_responses(guesses_and_responses);
            analyze::analyze(&_answers, &_valid, guesses_and_responses);
        }

        "stats" => {
            let guesses_and_responses = read_guesses_and_responses(args);
            let guesses_and_responses = analyze::parse_into_guesses_and_responses(guesses_and_responses);
            analyze::stats(&_answers, &_valid, guesses_and_responses);
        }

        "search" => {
            if args.len() < 1 {
                println!("Usage: wordle_v2 search [--after <after_word>]? [--cutoff <cutoff>]? [--ratio <cutoff_ratio>]? <count_to_find> <starting_guesses>...");
                return;
            }

            let mut cutoff = 0.0;
            let mut cutoff_ratio = 0.0;

            while let Some(first) = args.first() {
                match *first {
                    "--cutoff" => {
                        cutoff = args[1].parse::<f64>().unwrap();
                        args = &args[2..];
                    },
                    "--ratio" => {
                        cutoff_ratio = args[1].parse::<f64>().unwrap();
                        args = &args[2..];
                    },
                    "--after" => {
                        let word = Word::new(args[1]).unwrap();
                        let index = _valid.iter().position(|w| *w >= word).unwrap();
                        _valid = _valid[index..].to_vec();
                        args = &args[2..];
                    },
                    _ => break,
                }
            }

            let count_left = args.first().unwrap().parse::<usize>().unwrap();
            let initial_guesses = args[1..].iter().map(|s| Word::new(s).unwrap()).collect::<Vec<Word>>();

            let mut best = search::find_best(
                &_answers, 
                &_valid, 
                initial_guesses,
                count_left,
                //score::total_turn_two_savings 
                //score::total_turns_savings
                //search::score_cluster_count
                //score::total_turns_ideal_map
                //score::total_turns_pessimistic_map
                rank::total_turns_predicted_map,
                //rank::total_turns_random_map,
                cutoff,
                cutoff_ratio
            );

            println!();
            println!("Top {} (last is best):", best.len());
            while let Some((score, guesses, cv)) = best.pop() {
                println!("{}: {:?} {}", score, guesses, cv.to_string());
            }
        }

        "orders" => {
            // Show the possible letter orderings given known letters.
            // Pass letter+position pairs. Uppercase for green letters, lowercase for yellow.
            // v2 orders l2 i3 n4 o2

            // Note: No way to indicate two copies of a letter which are both yellow in this implementation.

            let orders = LetterOrders::parse(args).unwrap().show();
            for word in orders.iter() {
                println!("{}", word);
            }

            println!();
            println!("{} orders", orders.len());
        }

        "explain_hybrid" => {
            // For the guesses, show how perfect hybrid play would go.
            // After each guess, if there are 1-2 words left, guess from them. Otherwise, use next standard guess.
            // Show how many words are guessed directly after each standard guess and the total turns for the strategy across all possible answers.
            let guesses = args.iter().map(|s| Word::new(s).unwrap()).collect::<Vec<Word>>();
            analyze::explain_hybrid(_answers.clone(), guesses);
        }

        "score_answers" => {
            let results = single_use::score_answers_by_turns(&_answers, &_valid);
            for (score, answer, cv) in results.iter() {
                println!("{}: {}  {}", score, answer, cv.to_string());
            }
        }

        "score_guesses" => {
            let results = single_use::score_guesses_by_responses(&_answers, &_valid);
            for (_score, guess) in results[0..500].iter() {
                //println!("{}: {}", guess, score);
                println!("{}", guess);
            }
        }

        "neighbors" => {
            let neighbors = single_use::letter_neighbors(&_answers, 0..6);
            println!("BEFORE");
            println!("{}", single_use::print_letter_neighbors(&neighbors, NeighborSide::Before));

            println!();
            println!("AFTER");
            println!("{}", single_use::print_letter_neighbors(&neighbors, NeighborSide::After));
        }

        "consonant_pairs" => {
            let pairs = single_use::consonant_pairs(&_answers);
            println!("CONSONANT PAIRS");
            println!("{}", single_use::print_consonant_pairs(&pairs));
        }

        "answer_stats" => {
            let counts = single_use::letter_frequency(&_answers);
            single_use::letter_first_odds(&counts);
            single_use::repeat_letter_odds(&_answers);
        }

        "perf" => {
            let mut count = 0;

            for guess in _valid.iter() {
                for answer in _answers.iter() {
                    let _resp = Response::score(*guess, *answer);
                    count += 1;
                }
            }

            println!("{} score iterations", count);
        }

        "setup" => {
            // One-Time Work
            single_use::merge_answers_into_guesses("data/2315");
            single_use::merge_answers_into_guesses("data/2309");
        }

        _ => {
            println!("Unknown mode: {}", mode);
        }
    }

    let duration: f64 = start.elapsed().as_secs_f64();
    eprintln!(" -> {duration:.3} sec");
}

fn read_guesses_and_responses(args: &[&str]) -> Vec<String> {
    if args.len() > 0 {
        args.iter().map(|s| s.to_string()).collect::<Vec<String>>()
    } else {
        println!("Enter guesses ('soare') and responses ('gbbby') so far:");

        let mut entries = Vec::new();
        let lines = std::io::stdin().lines();
        for line in lines {
            let line = line.unwrap().trim().to_ascii_lowercase();
            if line.len() == 0 { break; }

            for part in line.split(' ') {
                entries.push(part.to_string());
            }
        }

        entries
    }
}

fn assess_inner(guesses: Option<&str>, valid: &Vec<Word>, answers: &Vec<Word>, strategy_path: &str) -> Result<String, String> {
    let simulate_game_count = 10000;

    let strategy_text = fs::read_to_string(&strategy_path).unwrap();
    let tree = WordleTree::parse(strategy_text.lines()).unwrap();
    let player = tree_player::TreePlayer::new(&tree);

    return check::assess_and_simulate(guesses, valid, answers, simulate_game_count, player);
}