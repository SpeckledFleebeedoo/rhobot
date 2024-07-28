use std::iter::once;

/// Escapes any markdown formatting in a string.
pub fn escape_formatting(unformatted_string: &str) -> String {
    // This is supposedly cheaper than using the String::replace function.
    unformatted_string
        .chars()
        .flat_map(|c| match c {
            '_' | '*' | '~' => Some('\\'),
            _ => None
        }
            .into_iter()
            .chain(once(c))
        )
        .flat_map(|c| once(c).chain( match c {
            '@' => Some('\u{200b}'),
            _ => None
        }))
        .collect::<String>()
}

/// Capitalizes the first character in str s, lowercases the rest.
pub fn capitalize(s: &str) -> String {
    let lowercased = s.to_lowercase();
    let mut chars = lowercased.chars();
    chars.next().map_or_else(String::new, |f| f.to_uppercase().collect::<String>() + chars.as_str())
}