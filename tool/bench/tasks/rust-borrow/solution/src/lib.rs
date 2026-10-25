/// The longest word, the first one on a tie.
pub fn longest(words: &Vec<String>) -> &String {
    let mut best = &words[0];
    for w in words {
        if w.len() > best.len() {
            best = w;
        }
    }
    best
}

/// Every word in upper case, appended to the list.
pub fn shout(words: &mut Vec<String>) {
    let upper: Vec<String> = words.iter().map(|w| w.to_uppercase()).collect();
    words.extend(upper);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn works() {
        let mut v = vec!["ab".to_string(), "abc".to_string()];
        assert_eq!(longest(&v), "abc");
        shout(&mut v);
        assert_eq!(v, ["ab", "abc", "AB", "ABC"]);
    }
}
