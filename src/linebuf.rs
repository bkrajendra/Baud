pub struct LineAssembler {
    partial: Vec<u8>,
}

impl LineAssembler {
    pub fn new() -> Self {
        Self { partial: Vec::new() }
    }

    pub fn push(&mut self, bytes: &[u8]) -> Vec<String> {
        self.partial.extend_from_slice(bytes);

        let mut lines = Vec::new();
        while let Some(pos) = self.partial.iter().position(|&b| b == b'\n') {
            let mut line: Vec<u8> = self.partial.drain(..=pos).collect();
            line.pop(); // remove '\n'
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            lines.push(String::from_utf8_lossy(&line).into_owned());
        }
        lines
    }

    pub fn flush(&mut self) -> Option<String> {
        if self.partial.is_empty() {
            return None;
        }
        let remaining = std::mem::take(&mut self.partial);
        Some(String::from_utf8_lossy(&remaining).into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_chunk_single_line() {
        let mut a = LineAssembler::new();
        assert_eq!(a.push(b"hello\n"), vec!["hello".to_string()]);
    }

    #[test]
    fn splits_multiple_lines_in_one_chunk() {
        let mut a = LineAssembler::new();
        assert_eq!(
            a.push(b"one\ntwo\nthree\n"),
            vec!["one".to_string(), "two".to_string(), "three".to_string()]
        );
    }

    #[test]
    fn strips_trailing_cr() {
        let mut a = LineAssembler::new();
        assert_eq!(a.push(b"hello\r\n"), vec!["hello".to_string()]);
    }

    #[test]
    fn buffers_partial_line_across_chunks() {
        let mut a = LineAssembler::new();
        assert_eq!(a.push(b"hel"), Vec::<String>::new());
        assert_eq!(a.push(b"lo\n"), vec!["hello".to_string()]);
    }

    #[test]
    fn flush_returns_partial_line() {
        let mut a = LineAssembler::new();
        assert_eq!(a.push(b"partial"), Vec::<String>::new());
        assert_eq!(a.flush(), Some("partial".to_string()));
        assert_eq!(a.flush(), None);
    }

    #[test]
    fn invalid_utf8_is_lossily_decoded() {
        let mut a = LineAssembler::new();
        let lines = a.push(&[0xFF, 0xFE, b'\n']);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains('\u{FFFD}'));
    }
}
