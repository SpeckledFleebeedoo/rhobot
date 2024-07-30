use std::iter::once;

pub trait DiscordFormat {
    fn truncate_for_embed(self, max_len: usize) -> String;
    fn capitalize(self) -> String;
    fn escape_formatting(self) -> String;
}
impl DiscordFormat for String {
    /// Truncates a String to a set length for use in embeds
    fn truncate_for_embed(self, max_len: usize) -> String {
        let mut out = self;
        if out.len() > max_len - 3 {
            out.truncate(max_len);
            out.push_str("...");
        }
        out
    }

    /// Capitalizes the first character in str s, lowercases the rest.
    fn capitalize(self) -> String {
        let lowercased = self.to_lowercase();
        let mut chars = lowercased.chars();
        chars.next().map_or_else(Self::new, |f| f.to_uppercase().collect::<Self>() + chars.as_str())
    }

    /// Escapes any markdown formatting in a string.
    fn escape_formatting(self) -> String {
        // This is supposedly cheaper than using the String::replace function.
        self
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
            .collect::<Self>()
    }
}


impl DiscordFormat for &str {
        /// Truncates a String to a set length for use in embeds
        fn truncate_for_embed(self, max_len: usize) -> String {
            self.to_owned().truncate_for_embed(max_len)
        }
    
        /// Capitalizes the first character in str s, lowercases the rest.
        fn capitalize(self) -> String {
            self.to_owned().capitalize()
        }
    
        /// Escapes any markdown formatting in a string.
        fn escape_formatting(self) -> String {
            self.to_owned().escape_formatting()
        }
}