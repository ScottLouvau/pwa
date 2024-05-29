use std::{iter::Peekable, str::{Chars, Lines}};
use crate::word::Word;

pub struct Parser<'a> {
    pub line_number: usize,
    pub char_in_line: usize,
    pub current: String,
    pub current_line: String,

    lines: Lines<'a>,
    line: Peekable<Chars<'a>>,
}

impl Parser<'_> {
    pub fn new<'a>(text: Lines<'a>) -> Parser<'a> {
        let mut parser = Parser {
            line_number: 0,
            char_in_line: 0,
            current: "".into(),
            current_line: "".into(),
            lines: text,
            line: "".chars().peekable(),
        };

        // Get the first line and first token
        let _ = parser.next_line();

        parser
    }

    pub fn next_line(&mut self) -> Result<(), String> {
        self.line_number += 1;
        self.char_in_line = 1;
        self.current = "".into();
        self.line = "".chars().peekable();

        if let Some(line) = self.lines.next() {
            self.current_line = line.into();
            self.line = line.chars().peekable();

            self.next()?;
            Ok(())
        } else {
            Err(self.error("Out of content when more expected."))
        }
    }

    pub fn next(&mut self) -> Result<&str, String> {
        // Error to call next the second time when nothing is left on the line
        if self.line.peek().is_none() && self.current == "" {
            return Err(self.error("Out of content when more expected."));
        }

        let mut result = String::new();

        // Advance position beyond previous token
        self.char_in_line += self.current.chars().count();

        // Ignore any leading spaces
        while let Some(' ') = self.line.peek() {
            self.char_in_line += 1;
            self.line.next();
        }

        // Collect until a delimiter: space, comma, paren, brace, bracket
        while let Some(c) = self.line.peek() {
            let c = *c;
            if c == ' ' || c == ',' || c == '(' || c == ')' || c == '[' || c == ']' || c == '{' || c == '}' { 
                if result.len() == 0 { 
                    result.push(c); 
                    self.line.next();
                }

                break; 
            } else {
                result.push(c);
                self.line.next();
            }
        }

        // Capture and return this token
        self.current = result;
        Ok(&self.current)
    }

    pub fn as_word(&self) -> Result<Option<Word>, String> {
        if self.current == "*" {
            Ok(None)
        } else if let Some(word) = Word::new(&self.current) {
            Ok(Some(word))
        } else {
            Err(self.error("Not a valid word or '*'"))
        }
    }

    pub fn as_f64(&self) -> Result<f64, String> {
        if let Ok(value) = self.current.parse::<f64>() {
            Ok(value)
        } else {
            Err(self.error("Not a valid floating point number"))
        }
    }

    pub fn as_usize(&self) -> Result<usize, String> {
        if let Ok(value) = self.current.parse::<usize>() {
            Ok(value)
        } else {
            Err(self.error("Not a valid number"))
        }
    }

    pub fn require(&mut self, expected: &str) -> Result<(), String> {
        if self.current == expected {
            let _ = self.next()?;
            Ok(())
        } else {
            Err(self.error(&format!("'{}' required here", expected)))
        }
    }

    pub fn error(&self, message: &str) -> String {
        format!("@({}, {}) \"{}\": {}", self.line_number, self.char_in_line, self.current, message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    #[test]
    fn test_parsing_primitives() {
        let text = "    4.3 (*, 6) -> fries  [10, 4]  {fried, fries, frisk}";
        let mut parser = Parser::new(text.lines());
        
        // Verify whitespace skipped, first value found
        assert_eq!(parser.current, "4.3");

        // Verify parses as float, but not usize or Word
        assert_eq!(parser.as_f64().unwrap(), 4.3);
        assert!(parser.as_usize().is_err());
        assert!(parser.as_word().is_err());

        // Verify position as expected
        assert_eq!(parser.line_number, 1);
        assert_eq!(parser.char_in_line, 5);

        assert_eq!(next(&mut parser), "(");
        
        // Verify '*' parses as a valid Word (None)
        assert_eq!(next(&mut parser), "*");
        assert_eq!(parser.as_word().unwrap(), None);
        
        // Verify require checks current token and advances when ok
        assert_eq!(next(&mut parser), ",");
        assert!(parser.require("->").is_err());
        assert!(parser.require(",").is_ok());

        // Verify 6 parses as float or usize
        assert_eq!(parser.current, "6");
        assert_eq!(parser.as_f64().unwrap(), 6.0);
        assert_eq!(parser.as_usize().unwrap(), 6usize);

        assert_eq!(next(&mut parser), ")");

        // Verify '->' stays as a single token and "fries" parses as a Some Word.
        assert_eq!(next(&mut parser), "->");
        assert_eq!(next(&mut parser), "fries");
        assert_eq!(parser.as_word().unwrap(), Some(w("fries")));
        
        assert_eq!(next(&mut parser), "[");
        assert_eq!(next(&mut parser), "10");
        assert_eq!(next(&mut parser), ",");
        assert_eq!(next(&mut parser), "4");
        assert_eq!(next(&mut parser), "]");

        assert_eq!(next(&mut parser), "{");
        assert_eq!(next(&mut parser), "fried");
        assert_eq!(next(&mut parser), ",");
        assert_eq!(next(&mut parser), "fries");
        assert_eq!(next(&mut parser), ",");
        assert_eq!(next(&mut parser), "frisk");
        assert_eq!(parser.as_word().unwrap(), Some(w("frisk")));
        assert_eq!(next(&mut parser), "}");

        // Verify last token, position was tracked properly (1-based, so start of last '}' is same as length)
        assert_eq!(parser.line_number, 1);
        assert_eq!(parser.char_in_line, text.len());

        // Verify next and next_line error when content runs out
        assert_eq!(next(&mut parser), "");
        assert!(parser.next().is_err());
        assert!(parser.next_line().is_err());
    }

    fn next<'a>(parser: &'a mut Parser) -> &'a str {
        parser.next().unwrap()
    }
}