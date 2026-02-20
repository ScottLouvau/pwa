use crate::word::Word;

pub const ALL_GREEN: u16 = (2 << 8) + (2 << 6) + (2 << 4) + (2 << 2) + 2;

/// Represents one 'tile' of a Wordle response
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tile {
    Black,
    Yellow,
    Green
}

/// Contains the response for a Wordle guess, one tile per letter.
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Response {
    pub value: u16
}

impl Response {
    pub fn new(value: u16) -> Response {
        Response { value }
    }

    /// Convert typed response characters to a Response
    pub fn from_str(text: &str) -> Option<Response> { 
        let mut value = 0;
        let mut count = 0;

        for c in text.chars() {
            count += 1;
            value = value << 2;

            match c {
                '🟩' | 'g' | 'G' => value += Tile::Green as u16,
                '🟨' | 'y' | 'Y' => value += Tile::Yellow as u16,
                '⬛' | 'b' | 'B' => value += Tile::Black as u16,
                _ => return None,
            }
        }

        if count == 5 { 
            Some(Response { value }) 
        } else { 
            None 
        }
    }

    /// Convert knowns string to a Response
    pub fn from_knowns_str(text: &str) -> Option<Response> { 
        let mut value = 0;
        let mut count = 0;

        for c in text.chars() {
            count += 1;
            value = value << 2;

            if c.is_alphabetic() {
                if c.is_ascii_uppercase() {
                    value += Tile::Green as u16;
                } else {
                    value += Tile::Yellow as u16;
                }
            } else if c == '.' || c == '_' {
                value += Tile::Black as u16;
            } else {
                return None;
            }
        }

        if count == 5 { 
            Some(Response { value }) 
        } else { 
            None 
        }
    }
    
    /// Iterator over the Tiles in this Response
    pub fn iter(&self) -> ResponseIterator {
        ResponseIterator { value: self.value, shift: 10u8 }
    }

    // Count the number of non-black tiles returned
    pub fn known_count(&self) -> u8 {
        let mut count = 0u8;
        
        for tile in self.iter() {
            if tile == Tile::Yellow || tile == Tile::Green {
                count += 1;
            }
        }

        count
    }

    /// Show the emoji form of this Response ("🟩🟨⬛🟨🟨")
    pub fn to_string(&self) -> String {
        let mut result = String::new();

        for tile in self.iter() {
            let c = match tile {
                Tile::Black => '⬛',
                Tile::Yellow => '🟨',
                Tile::Green => '🟩'
            };

            result.push(c);
        }

        result
    }

    /// Show known letters for this Response and the associated guess ("🟩🟨⬛🟨🟨" for SOARE -> "So.re")
    pub fn to_knowns_string(&self, guess: &Word) -> String {
        let mut result = String::new();

        for (tile, letter) in self.iter().zip(guess.iter()) {
            let c = match tile {
                Tile::Black => b'.',
                Tile::Yellow => letter.to_ascii_lowercase(),
                Tile::Green => letter.to_ascii_uppercase()
            };

            result.push(c as char);
        }

        result
    }

    pub fn score(guess: Word, answer: Word) -> Response {
        // Count how many mismatched copies of each letter there are in the answer
        let mut mismatched_count_by_letter = [0u8; 26];
        for (answer_letter, guess_letter) in answer.iter_index().zip(guess.iter_index()) {
            if answer_letter != guess_letter {
                mismatched_count_by_letter[answer_letter as usize] += 1;
            }
        }
    
        // Go over the guess and score it
        let mut value = 0;
        for (answer_letter, guess_letter) in answer.iter_index().zip(guess.iter_index()) {
            // Previous tiles are in higher bits; two bits per tile
            value = value << 2;
    
            if answer_letter == guess_letter {
                // Right letter in right position is green (2)
                value += 2;
            } else {
                // If there are mismatched copies of this letter left in the answer, use one and mark yellow (1)
                let guess_index = guess_letter as usize;
                if mismatched_count_by_letter[guess_index] > 0 {
                    value += 1;
                    mismatched_count_by_letter[guess_index] -= 1;
                }
            }
    
            // Otherwise this letter is black (0)
        }
    
        Response::new(value)
    }
}

impl std::fmt::Debug for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_string())
    }
}

impl std::fmt::Display for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_string())
    }
}

pub struct ResponseIterator {
    value: u16,
    shift: u8
}

impl Iterator for ResponseIterator {
    type Item = Tile;

    fn next(&mut self) -> Option<Self::Item> {
        if self.shift == 0 { return None; }

        self.shift -= 2;
        let part = (self.value >> self.shift) & 3u16;
        let tile = match part {
            0 => Tile::Black,
            1 => Tile::Yellow,
            2 => Tile::Green,
            _ => panic!("Invalid score")
        };

        Some(tile)
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ResponseSet {
    responses: u64
}

impl ResponseSet {
    pub fn new() -> ResponseSet {
        ResponseSet { responses: 0u64 }
    }

    pub fn push(&mut self, r: Response) {
        self.responses = (self.responses << 10) | ((r.value as u64) + 1);
    }

    pub fn pop(&mut self) -> Option<Response> {
        if self.responses == 0 { return None; }

        let r = Response { value: ((self.responses & 0x3FF) - 1) as u16 };
        self.responses = self.responses >> 10;
        Some(r)
    }

    pub fn to_knowns(&self, guesses: &Vec<Word>) -> String {
        let mut copy = self.clone();

        let mut responses = Vec::new();
        while let Some(r) = copy.pop() {
            responses.push(r);
        }

        let mut result = String::new();

        for (response, guess) in responses.iter().rev().zip(guesses) {
            if result.len() > 0 { result.push(' '); }
            result += &response.to_knowns_string(guess);
        }
        
        result
    }

    // Return total known letters across guesses
    // This will count a letter repeatedly if it is in several guesses.
    pub fn known_count(&self) -> u8 {
        let mut count = 0u8;

        let mut copy = self.clone();
        while let Some(r) = copy.pop() {
            count += r.known_count();
        }

        count
    }
}

impl std::fmt::Debug for ResponseSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.responses.to_string())
    }
}

pub struct Constraint {
    must_have_letters: u32,
    must_not_have_letters: u32
}

impl Constraint {
    pub fn new() -> Constraint {
        Constraint { must_have_letters: 0u32, must_not_have_letters: 0u32 }
    }

    pub fn add(&mut self, guess: Word, response: Response) {
        let mut must_have_letters = 0u32;
        let mut must_not_have_letters = 0u32;

        for (tile, c) in response.iter().zip(guess.iter_index()) {
            match tile {
                Tile::Black  => must_not_have_letters |= 1u32 << c,
                Tile::Yellow => must_have_letters |= 1u32 << c,
                Tile::Green  => must_have_letters |= 1u32 << c
            };
        }

        // Ensure letters marked black appearing multiple times aren't added to "must-not-have"
        must_not_have_letters = must_not_have_letters & !must_have_letters;

        self.must_have_letters |= must_have_letters;
        self.must_not_have_letters |= must_not_have_letters;
    }

    pub fn matches(&self, word: Word) -> bool {
        let letters = word.letters_in_word();

        if letters & self.must_have_letters != self.must_have_letters { 
            false 
        } else if letters & self.must_not_have_letters != 0u32 { 
            false 
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{*, response::{ResponseSet, Constraint}};
    use super::{Response, Tile};

    #[test]
    fn response_basics() {
        // Verify creation and output (to_string(), Debug trait)
        let one = r("🟩🟨⬛🟨🟨");
        assert_eq!(one.to_string(), "🟩🟨⬛🟨🟨");
        assert_eq!(one.known_count(), 4u8);

        let two = r("🟩🟩🟩🟩🟩");
        assert_eq!(two.to_string(), "🟩🟩🟩🟩🟩");
        assert_eq!(format!("{:?}", two), "🟩🟩🟩🟩🟩");
        assert_eq!(two.known_count(), 5u8);

        let three = r("GgYyb");
        assert_eq!(three.to_string(), "🟩🟩🟨🟨⬛");

        // Verify length and characters validated
        assert!(Response::from_str("bbbb").is_none());
        assert!(Response::from_str("ggbbyy").is_none());
        assert!(Response::from_str("bbbbz").is_none());

        // Verify comparisons and clone
        assert!(one == one);
        assert!(one == one.clone());
        assert!(one != two);
        assert!(two > one);

        // Verify iterators
        assert_eq!(one.iter().collect::<Vec<Tile>>(), vec![Tile::Green, Tile::Yellow, Tile::Black, Tile::Yellow, Tile::Yellow]);

        // to_knowns_string
        assert_eq!(one.to_knowns_string(&w(&"soare")), "So.re");
        assert_eq!(two.to_knowns_string(&w(&"crane")), "CRANE");

        // from_knowns_str
        assert_eq!(Response::from_knowns_str(&one.to_knowns_string(&w(&"soare"))).unwrap(), one);
        assert_eq!(Response::from_knowns_str(&two.to_knowns_string(&w(&"soare"))).unwrap(), two);
        assert_eq!(Response::from_knowns_str(".O.r.").unwrap(), r("bGbyb"));
        assert_eq!(Response::from_knowns_str("___rE").unwrap(), r("bbbyG"));

        let zero = r("⬛⬛⬛⬛⬛");
        assert_eq!(zero.known_count(), 0u8);
    }

    #[test]
    fn score_basics() {
        assert_eq!(r("GYBBB").to_string(), score("decks", "diety"));

        // No matching letters
        assert_eq!("⬛⬛⬛⬛⬛", score("crane", "pools"));

        // Complete match, all unique letters
        assert_eq!("🟩🟩🟩🟩🟩", score("crane", "crane"));

        // Partial match (CR correct, A missing, N wrong place, E missing)
        assert_eq!("🟩🟩⬛🟨⬛", score("crane", "crown"));

        // All in wrong position
        assert_eq!("🟨🟨🟨🟨🟨", score("cares", "scare"));
    }

    #[test]
    fn score_repeat_letters() {
        // Fewer Copies in guess: All marked green or yellow
        assert_eq!("🟨⬛⬛⬛🟩", score("sills", "esses"));

        // If too many in guess, greens always are marked
        assert_eq!("⬛🟩🟩⬛🟩", score("sssss", "esses"));

        // If too many in guess, the first in-wrong-place copies get the yellow
        assert_eq!("🟨🟩🟩⬛⬛", score("sssso", "esses"));
        assert_eq!("🟨⬛🟩🟨⬛", score("sosso", "esses"));

        // Verify no correctly positioned letters aren't marked incorrectly
        assert_eq!("🟨⬛⬛⬛⬛", score("silly", "esses"));
    }

    fn score(guess: &str, answer: &str) -> String {
        Response::score(
            w(guess), 
            w(answer)
        ).to_string()
    }

    #[test]
    fn constraints() {
        let guess = w("acbxy");
        let answer = w("abcde");
        let response = Response::score(guess, answer);
        let mut constraint = Constraint::new(); 
        constraint.add(guess, response);

        // Verify 'abc' added must-have letters, and 'xy' as must-not-have
        let must_have = w("abccc").letters_in_word();
        let must_not_have = w("xxxyy").letters_in_word();
        assert_eq!(constraint.must_have_letters, must_have);
        assert_eq!(constraint.must_not_have_letters, must_not_have);

        // Verify original answer matches
        assert_eq!(constraint.matches(w("abcde")), true);

        // Verify word missing any of abc doesn't match
        assert_eq!(constraint.matches(w("aaabb")), false);

        // Verify word with any of xy doesn't match
        assert_eq!(constraint.matches(w("abcnx")), false);

        // Add another guess/response
        let guess = w("daddy");
        let response = Response::score(guess, answer);
        constraint.add(guess, response);

        // Verify 'd' added to must-have letters
        let must_have = w("abcdd").letters_in_word();
        assert_eq!(constraint.must_have_letters, must_have);

        // Verify must-not-have is unchanged
        // NOTE: 'd' must not be added to must-not-have, even though second and third 'd' had black tile responses
        assert_eq!(constraint.must_not_have_letters, must_not_have);

        // Verify "daddy" excluded due to y
        assert_eq!(constraint.matches(guess), false);

        // "abcde" still matches
        assert_eq!(constraint.matches(w("abcde")), true);

        // "abcdd" matches
        assert_eq!(constraint.matches(w("abcdd")), true);
    }

    #[test]
    fn response_set_basics() {
        let empty = ResponseSet::new();

        let mut set = ResponseSet::new();
        assert_eq!(set, empty);
        assert_eq!(set.known_count(), 0u8);

        set.push(r("ggggg"));
        let single = set.clone();
        assert_ne!(set, empty);
        assert_eq!(set, single);
        assert_eq!(set.known_count(), 5u8);

        set.push(r("ybbyg"));
        assert_ne!(set, single);
        assert_eq!(set.known_count(), 8u8);

        assert_eq!(set.to_knowns(&wv("soare, clint")), "SOARE c..nT");

        assert_eq!(set.pop(), Some(r("ybbyg")));
        assert_eq!(set.pop(), Some(r("ggggg")));
        assert_eq!(set, empty);
        assert_eq!(set.pop(), None);
    }
}
