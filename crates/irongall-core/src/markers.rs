/// Comment syntax for managed regions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommentStyle {
    /// `# IRONGALL-BEGIN`
    Hash,
    /// `/* IRONGALL-BEGIN */`
    Css,
    /// `<!-- IRONGALL-BEGIN -->`
    Xml,
    /// `-- IRONGALL-BEGIN`
    Lua,
    /// `" IRONGALL-BEGIN` (Vim)
    Vim,
    /// `// IRONGALL-BEGIN`
    Slash,
}

impl CommentStyle {
    pub fn begin(self) -> &'static str {
        match self {
            Self::Hash => "# IRONGALL-BEGIN",
            Self::Css => "/* IRONGALL-BEGIN */",
            Self::Xml => "<!-- IRONGALL-BEGIN -->",
            Self::Lua => "-- IRONGALL-BEGIN",
            Self::Vim => "\" IRONGALL-BEGIN",
            Self::Slash => "// IRONGALL-BEGIN",
        }
    }

    pub fn end(self) -> &'static str {
        match self {
            Self::Hash => "# IRONGALL-END",
            Self::Css => "/* IRONGALL-END */",
            Self::Xml => "<!-- IRONGALL-END -->",
            Self::Lua => "-- IRONGALL-END",
            Self::Vim => "\" IRONGALL-END",
            Self::Slash => "// IRONGALL-END",
        }
    }
}

/// Replace an existing managed region, or append one. Never touches text
/// outside the markers. Applying twice is a no-op (idempotent).
pub fn patch(content: &str, body: &str, style: CommentStyle) -> String {
    let begin = style.begin();
    let end = style.end();
    let block = format_block(body, style);
    if let Some(start) = find_begin(content, begin) {
        if let Some(rel_end) = content[start..].find(end) {
            let end_idx = eat_trailing_nl(content, start + rel_end + end.len());
            let mut out = String::with_capacity(content.len() + block.len());
            out.push_str(&content[..start]);
            if !out.is_empty() && !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str(&block);
            if end_idx < content.len() {
                let rest = &content[end_idx..];
                let rest = rest.strip_prefix('\n').unwrap_or(rest);
                if !out.ends_with('\n') {
                    out.push('\n');
                }
                out.push_str(rest);
            }
            if !out.ends_with('\n') {
                out.push('\n');
            }
            return out;
        }
    }
    let mut out = content.to_string();
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    if !out.is_empty() {
        out.push('\n');
    }
    out.push_str(&block);
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

pub fn format_block(body: &str, style: CommentStyle) -> String {
    let body = body.trim_end_matches('\n');
    format!("{}\n{}\n{}\n", style.begin(), body, style.end())
}

fn find_begin(content: &str, begin: &str) -> Option<usize> {
    content.find(begin)
}

fn eat_trailing_nl(content: &str, mut idx: usize) -> usize {
    if idx < content.len() && content.as_bytes()[idx] == b'\n' {
        idx += 1;
    }
    idx
}

/// True when a file already contains a managed region.
pub fn has_region(content: &str, style: CommentStyle) -> bool {
    content.contains(style.begin()) && content.contains(style.end())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_then_replace_is_idempotent() {
        let original = "user setting = 1\n";
        let body = "generated = 2";
        let once = patch(original, body, CommentStyle::Hash);
        let twice = patch(&once, body, CommentStyle::Hash);
        assert_eq!(once, twice);
        assert_eq!(once.matches("# IRONGALL-BEGIN").count(), 1);
        assert!(once.contains("user setting = 1"));
        assert!(once.contains("generated = 2"));
    }

    #[test]
    fn replace_changes_body() {
        let a = patch("keep\n", "one", CommentStyle::Hash);
        let b = patch(&a, "two", CommentStyle::Hash);
        assert!(b.contains("keep"));
        assert!(b.contains("two"));
        assert!(!b.contains("one"));
        assert_eq!(b.matches("# IRONGALL-BEGIN").count(), 1);
    }

    #[test]
    fn css_markers() {
        let out = patch("", "color: red;", CommentStyle::Css);
        assert!(out.contains("/* IRONGALL-BEGIN */"));
        assert!(out.contains("/* IRONGALL-END */"));
        assert!(out.contains("color: red;"));
    }
}
