// simple fnmatch recursive implementation
pub fn fnmatch(pattern: &str, text: &str) -> bool {
    fn helper(pat: &[u8], txt: &[u8]) -> bool {
        if pat.is_empty() {
            return txt.is_empty();
        }

        match pat[0] {
            b'?' => {
                // ? matches any single character
                if txt.is_empty() {
                    false
                } else {
                    helper(&pat[1..], &txt[1..])
                }
            }
            b'*' => {
                // * matches zero or more characters
                helper(&pat[1..], txt) || (!txt.is_empty() && helper(pat, &txt[1..]))
            }
            _ => {
                // exact character match
                if txt.is_empty() || pat[0] != txt[0] {
                    false
                } else {
                    helper(&pat[1..], &txt[1..])
                }
            }
        }
    }

    helper(pattern.as_bytes(), text.as_bytes())
}
