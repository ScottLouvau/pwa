// Identify the different positions a set of letters can be in, and show all of the potential orderings of those letters (with '_' for omitted letters when fewer than five known).
pub struct LetterOrders {
    letters: Vec<LetterOrder>
}

impl LetterOrders {
    pub fn new() -> LetterOrders {
        LetterOrders { letters: Vec::new() }
    }

    pub fn parse(args: &[&str]) -> Result<LetterOrders, String> {
        let mut result = LetterOrders::new();

        for arg in args {
            let letter = arg.chars().next().ok_or("Empty argument")?;
            if !letter.is_ascii_alphabetic() {
                return Err(format!("'{letter}' wasn't a letter"));
            }

            let position = arg[1..].parse::<usize>().or(Err("Position wasn't a number"))?;
            if position < 1 || position > 5 {
                return Err(format!("{position} was out of range"));
            }
            result.add(letter, position);
        }

        Ok(result)
    }

    pub fn add(&mut self, letter: char, position: usize) {
        if letter.is_ascii_uppercase() {
            // Green. Add a LetterOrder for each one.
            let mut order = LetterOrder::new(letter);
            order.only_pos(position);
            self.letters.push(order);

        } else {
            // Yellow. Remove position from existing, if found, or add new LetterOrder.
            for order in self.letters.iter_mut() {
                if order.letter == letter {
                    order.except_pos(position);
                    return;
                }
            }

            let mut order = LetterOrder::new(letter);
            order.except_pos(position);
            self.letters.push(order);
        }
    }

    pub fn show(&self) -> Vec<String> {
        let mut set = Vec::new();
        let mut assembled_word = ['_'; 5];
    
        self.show_inner(0, &mut assembled_word, &mut set);
        set
    }
    
    fn show_inner(&self, next_letters_index: usize, assembled_word: &mut [char; 5], set: &mut Vec<String>) {
        // If all letters were placed, add this word option
        if next_letters_index >= self.letters.len() {
            set.push(assembled_word.iter().collect::<String>());
            return;
        }
    
        // Try to place the next known letter
        let next_letter = &self.letters[next_letters_index];
        for (i, allowed) in next_letter.allowed_pos.iter().enumerate() {
            // Try each allowed position that is still empty
            if *allowed {
                if assembled_word[i] == '_' {
                    assembled_word[i] = next_letter.letter;
                    self.show_inner(next_letters_index + 1, assembled_word, set);
                    assembled_word[i] = '_';
                }
            }
        }
    }
}

pub struct LetterOrder {
    letter: char,
    allowed_pos: [bool; 5]
}

impl LetterOrder {
    fn new(letter: char) -> LetterOrder {
        LetterOrder { letter: letter.to_ascii_lowercase(), allowed_pos: [true; 5] }
    }

    fn except_pos(&mut self, except_in_pos: usize) {
        self.allowed_pos[except_in_pos - 1] = false;
    }

    fn only_pos(&mut self, only_pos: usize) {
        for i in 0..5 {
            if i != only_pos - 1 {
                self.allowed_pos[i] = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letter_orders() {
        // Only one order if no letters are known
        assert_eq!(LetterOrders::parse(&vec![]).unwrap().show(), vec!["_____"]);

        // Only one order for green letters
        assert_eq!(LetterOrders::parse(&vec!["S1", "T2", "L4"]).unwrap().show(), vec!["st_l_"]);

        // Four options for one yellow
        assert_eq!(LetterOrders::parse(&vec!["c1"]).unwrap().show(), vec!["_c___", "__c__", "___c_", "____c"]);

        // Two options if a yellow eliminated in a few positions
        assert_eq!(LetterOrders::parse(&vec!["c1", "c3", "c5"]).unwrap().show(), vec!["_c___", "___c_"]);

        // One option left for three green and a yellow
        assert_eq!(LetterOrders::parse(&vec!["S1", "T2", "a5", "L4"]).unwrap().show(), vec!["stal_"]);

        // Two options left for three green and a yellow if yellow was in one of the green positions
        assert_eq!(LetterOrders::parse(&vec!["S1", "T2", "a1", "L4"]).unwrap().show(), vec!["stal_", "st_la"]);

        // Two options with if two unknown positions and two yellows that could use either
        assert_eq!(LetterOrders::parse(&vec!["S1", "T2", "L4", "a1", "e1"]).unwrap().show(), vec!["stale", "stela"]);

        // Error handling: Empty argument, non-letter, non-digit, position out of range
        assert_eq!(LetterOrders::parse(&vec![""]).is_err(), true);
        assert_eq!(LetterOrders::parse(&vec!["11"]).is_err(), true);
        assert_eq!(LetterOrders::parse(&vec!["1e"]).is_err(), true);
        assert_eq!(LetterOrders::parse(&vec!["a0"]).is_err(), true);
        assert_eq!(LetterOrders::parse(&vec!["z6"]).is_err(), true);

    }
}