//! Glob matching for source-role rules.

/// Matches one glob pattern against a repository-relative path.
///
/// `*` and `?` stay inside one path component; `**` crosses directories.
pub fn glob_matches(pattern: &str, text: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let text: Vec<char> = text.chars().collect();
    let mut states = vec![false; text.len() + 1];
    states[0] = true;
    let mut pattern_index = 0;
    while pattern_index < pattern.len() {
        let character = pattern[pattern_index];
        let mut next = vec![false; text.len() + 1];
        for (index, active) in states.iter().copied().enumerate() {
            if !active {
                continue;
            }
            match character {
                '*' => {
                    next[index] = true;
                    let crosses_directories = pattern.get(pattern_index + 1) == Some(&'*');
                    for (text_index, slot) in next.iter_mut().enumerate().skip(index + 1) {
                        if !crosses_directories && text[text_index - 1] == '/' {
                            break;
                        }
                        *slot = true;
                    }
                }
                '?' if index < text.len() && text[index] != '/' => next[index + 1] = true,
                character if index < text.len() && character == text[index] => {
                    next[index + 1] = true
                }
                _ => {}
            }
        }
        states = next;
        if character == '*' && pattern.get(pattern_index + 1) == Some(&'*') {
            pattern_index += 1;
        }
        pattern_index += 1;
    }
    states[text.len()]
}
