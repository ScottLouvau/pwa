use std::{path::Path, fs};

/// Contains a lowercase, five-letter Wordle word.
///  This representation allows extremely fast cloning and comparison and takes minimal space.
#[derive(Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Word {
    word: u32
}

impl Word {
    pub fn new(text: &str) -> Option<Word> {
        let mut word = 0u32;
        if text.len() != 5 { return None; }

        for c in text.as_bytes() {
            let c = c.to_ascii_lowercase();
            if c < b'a' || c > b'z' { return None; }

            let c = (c.to_ascii_lowercase() - b'a') as u32;
            word = word << 5;
            word = word | c;
        }

        Some(Word { word })
    }

    /// Iterate over UTF-8 bytes of word text
    pub fn iter(&self) -> ByteIterator {
        ByteIterator { word: self.word, shift: 25u32 }
    }

    /// Iterate over 0-based index of letters (b'a' == 0, b'z' == 25)
    pub fn iter_index(&self) -> IndexIterator {
        IndexIterator { word: self.word, shift: 25u32 }
    }

    /// Return a u32 with bits set for letters in this word
    pub fn letters_in_word(&self) -> u32 {
        let mut result = 0u32;

        for c in self.iter_index() {
            result = result | (1u32 << c);
        }

        result
    }

    /// Return whether this word has any repeated letters
    pub fn has_repeat_letters(&self) -> bool {
        let mut letters = 0u32;

        for c in self.iter_index() {
            let mask = 1u32 << c;
            if letters & mask != 0 { return true; }
            letters = letters | mask;
        }

        false
    }

    /// Convert word back to String representation
    pub fn to_string(&self) -> String {
        let mut text = String::new();

        for c in self.iter() {
            text.push(c as char);
        }

        text
    }

    /// Parse a file containing a set of five-letter Wordle words
    pub fn parse_file(file_path: &Path) -> Vec<Word> {
        Self::parse_lines(&fs::read_to_string(&file_path).expect(&format!("Unable to read '{:?}'", &file_path)))
    }

    /// Parse a set of Words, one per line
    pub fn parse_lines(contents: &str) -> Vec<Word> {
        let mut result = Vec::new();

        for line in contents.lines() {
            let word = Word::new(line).expect(&format!("Invalid word: '{:?}'", &line));
            result.push(word);
        }

        result
    }
}

impl std::fmt::Debug for Word {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_string())
    }
}

impl std::fmt::Display for Word {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_string())
    }
}

pub struct ByteIterator {
    word: u32,
    shift: u32
}

impl Iterator for ByteIterator {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.shift == 0 { return None; }

        self.shift -= 5;
        let c = ((self.word >> self.shift) & 31u32) as u8;
        let c = c + b'a';
        Some(c)
    }
}

pub struct IndexIterator {
    word: u32,
    shift: u32
}

impl Iterator for IndexIterator {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.shift == 0 { return None; }

        self.shift -= 5;
        let c = ((self.word >> self.shift) & 31u32) as u8;
        Some(c)
    }
}

pub fn interesting_letters(words: &[Word]) -> u32 {
    let mut result = 0u32;

    for word in words {
        result = result | word.letters_in_word();
    }

    if words.len() > 1 {
        for i in 0..5 {
            let letter = words[0].iter_index().nth(i).unwrap();
            let mut all_same = true;
            
            for word in &words[1..] {
                if word.iter_index().nth(i).unwrap() != letter {
                    all_same = false;
                    break;
                }
            }

            if all_same {
                result &= !(1u32 << letter);
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::{Word, interesting_letters};
    use std::path::Path;

    #[test]
    fn word_basics() {
        // Verify creation and output (to_string(), Debug trait)
        let one = w("crAne");
        assert_eq!(one.to_string(), "crane");
        assert_eq!(format!("{:?}", one), "crane");

        let two = w("FINAL");
        assert_eq!(two.to_string(), "final");

        // Verify 'new' checks length and character set
        assert!(Word::new("strange").is_none());  // (Too long)
        assert!(Word::new("`nope").is_none());
        assert!(Word::new("nope{").is_none());
        assert!(Word::new("@nope").is_none());
        assert!(Word::new("nope[").is_none());
        assert!(Word::new("12345").is_none());

        // Verify comparisons and clone
        assert!(one == one.clone());
        assert!(one != two);
        assert!(one < two);

        // Verify iterators
        assert_eq!(one.iter().collect::<Vec<u8>>(), vec![b'c', b'r', b'a', b'n', b'e']);
        assert_eq!(one.iter_index().collect::<Vec<u8>>(), vec![2u8, 17u8, 0u8, 13u8, 4u8]);

        // Verify letters_in_word
        let next = w("abbde");
        assert_eq!(next.letters_in_word(), 0b0011011);

        // Verify has_repeat_letters
        assert_eq!(w("cabal").has_repeat_letters(), true);
        assert_eq!(w("angst").has_repeat_letters(), false);
        assert_eq!(w("aahed").has_repeat_letters(), true);
        assert_eq!(w("zymiz").has_repeat_letters(), true);
    }

    #[test]
    fn test_interesting_letters() {
        // Only 'b', 'd', 'g' are not in the same position
        let letters = interesting_letters(&vec![w("drain"), w("brain"), w("grain")]);
        assert_eq!(letters, (1u32 << 1) | (1u32 << 3) | (1u32 << 6));

        // All letters but C are in different positions
        let letters = interesting_letters(&vec![w("abcde"), w("edcba")]);
        assert_eq!(letters, (1u32 << 0) | (1u32 << 1) | (1u32 << 3) | (1u32 << 4));
    }

    #[test]
    fn parse() {
        let answers = Word::parse_file(Path::new("tst/_o_er/answers.txt"));
        assert_eq!(answers.len(), 25);
        assert_eq!(answers[0].to_string(), "boxer");
    }

    fn w(text: &str) -> Word {
        Word::new(text).unwrap()
    }
}