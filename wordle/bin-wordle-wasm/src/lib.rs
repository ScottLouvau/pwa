// Source: https://developer.mozilla.org/en-US/docs/WebAssembly/Rust_to_Wasm
// Build: wasm-pack build --target web
use wasm_bindgen::prelude::*;
use lib_wordle::{check, word::Word, wordle_tree::{tree_player, WordleTree}};

#[wasm_bindgen]
extern {
    pub fn alert(s: &str);
}

#[wasm_bindgen]
pub fn greet(name: &str) {
    alert(&format!("Hello, {}!", name));
}

#[wasm_bindgen]
pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[wasm_bindgen]
pub struct Wordle {
    valid: Vec<Word>,
    answers: Vec<Word>,
    strategy: WordleTree
}

#[wasm_bindgen]
impl Wordle {
    pub fn new(valid: &str, answers: &str, strategy: &str) -> Wordle {
        let valid = Word::parse_lines(valid);
        let answers = Word::parse_lines(answers);
        let strategy = WordleTree::parse(strategy.lines()).unwrap();

        Wordle { valid, answers, strategy }
    }

    pub fn assess(&self, guesses: &str, simulate_game_count: usize) -> Result<String, String> {
        let player = tree_player::TreePlayer::new(&self.strategy);
        check::assess_and_simulate(Some(guesses), &self.valid, &self.answers, simulate_game_count, player)
    }
}

// TODO: 
//  - Do I "construct" the WASM to pass in answer and valid lists, or something else?
//  - Try wrapping assess and exposing

// #[wasm_bindgen]
// pub fn assess(guesses: &str, valid: &str, answers: &str, simulate_game_count: usize, strategy_text: &str) -> Result<String, String> {
//     let valid = Word::parse_lines(valid);
//     let answers = Word::parse_lines(answers);

//     let tree = WordleTree::parse(strategy_text.lines()).unwrap();
//     let player = tree_player::TreePlayer::new(&tree);

//     check::assess_and_simulate(Some(guesses), &valid, &answers, simulate_game_count, player)
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
