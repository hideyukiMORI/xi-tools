//! サブプロセスが言った失敗の理由を 1 行にする。
//!
//! 🔑 `git` も `gh` も、失敗の1行目に人が読める理由を書く（`fatal: …` /
//! `gh: …`）。2行目以降は使い方の案内であることが多く、表の脇に出す 1 行としては
//! **最初の行が最も情報量が多い**。

/// stderr の最初の中身のある行。無ければ `fallback`。
pub(crate) fn first_line(stderr: &[u8], fallback: &str) -> String {
    String::from_utf8_lossy(stderr)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or(fallback)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::first_line;

    #[test]
    fn the_first_non_empty_line_wins() {
        assert_eq!(first_line(b"\n  fatal: bad\nmore\n", "x"), "fatal: bad");
        assert_eq!(first_line(b"gh: not logged in\n", "x"), "gh: not logged in");
    }

    /// 🔴 何も言わずに失敗することがある。そのときも理由の欄を空にしない。
    #[test]
    fn a_silent_failure_falls_back() {
        assert_eq!(first_line(b"", "x"), "x");
        assert_eq!(first_line(b"\n \n", "x"), "x");
    }
}
