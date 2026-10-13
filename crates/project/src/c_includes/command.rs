//! Splits compilation database command text without evaluating a shell.
#[derive(Default)]
struct CommandWords {
    words: Vec<String>,
    word: String,
    quote: Option<char>,
    escaped: bool,
}

pub(super) fn command_words(command: &str) -> Result<Vec<String>, String> {
    let mut parser = CommandWords::default();
    for character in command.chars() {
        parser.push(character)?;
    }
    parser.finish()
}

impl CommandWords {
    fn push(&mut self, character: char) -> Result<(), String> {
        if self.escaped {
            self.word.push(character);
            self.escaped = false;
            return Ok(());
        }
        if self.consume_quote_or_escape(character) {
            return Ok(());
        }
        if self.requires_evaluation(character) {
            return Err("compile command requires shell evaluation".to_owned());
        }
        if self.quote.is_none() && character.is_whitespace() {
            self.finish_word();
        } else {
            self.word.push(character);
        }
        Ok(())
    }

    fn consume_quote_or_escape(&mut self, character: char) -> bool {
        if character == '\\' && self.quote != Some('\'') {
            self.escaped = true;
        } else if Some(character) == self.quote {
            self.quote = None;
        } else if self.quote.is_none() && matches!(character, '\'' | '"') {
            self.quote = Some(character);
        } else {
            return false;
        }
        true
    }

    fn requires_evaluation(&self, character: char) -> bool {
        match character {
            '$' | '`' => self.quote != Some('\''),
            ';' | '|' | '&' | '<' | '>' => self.quote.is_none(),
            _ => false,
        }
    }

    fn finish_word(&mut self) {
        if !self.word.is_empty() {
            self.words.push(std::mem::take(&mut self.word));
        }
    }

    fn finish(mut self) -> Result<Vec<String>, String> {
        if self.escaped || self.quote.is_some() {
            return Err("compile command has incomplete quoting".to_owned());
        }
        self.finish_word();
        if self.words.is_empty() {
            return Err("compile command is empty".to_owned());
        }
        Ok(self.words)
    }
}
