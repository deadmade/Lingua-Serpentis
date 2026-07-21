/// Fold a word into the orthographic form the dictionary is stored in:
/// lowercase, with `v` written as `u` and `j` as `i`.
pub fn normalize(word: &str) -> String {
    word.chars()
        .map(|c| match c.to_ascii_lowercase() {
            'v' => 'u',
            'j' => 'i',
            other => other,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowercases() {
        assert_eq!(normalize("AMARE"), "amare");
    }

    #[test]
    fn maps_v_to_u_and_j_to_i() {
        // Classical Latin has no distinct v/j; the dictionary is stored in i/u
        // form, so both sides are folded the same way.
        assert_eq!(normalize("Verbum"), "uerbum");
        assert_eq!(normalize("Julius"), "iulius");
    }

    #[test]
    fn leaves_other_characters_alone() {
        // Underscores/digits survive normalization so they can fail lookup and
        // correctly produce a warning.
        assert_eq!(normalize("my_var2"), "my_uar2");
    }
}
