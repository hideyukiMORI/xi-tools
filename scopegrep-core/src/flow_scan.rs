//! フロー記法（`[…]` / `{…}`）を読み進める状態。
//!
//! 🔑 **行をまたいで再開できる**ことが、この型が独立している理由である。
//! compose の `healthcheck.test:` は `[` を次の行に置いて複数行に割る書き方が多く、
//! 実ファイル計測（設計メモ）では読めなかった 18 件のうち 14 件がこの形だった。
//! 1行で完結する前提の走査では、その 14 件が丸ごと落ちる。
//!
//! 🔑 **中には入らない**。括弧の深さとクォートだけを追い、要素には意味を与えない
//! （設計メモの部分集合）。だから状態は 3 つの値で足りる。

/// フロー記法の走査状態。深さが 0 に戻った時点で閉じている。
#[derive(Debug, Clone, Copy)]
pub(crate) struct FlowScan {
    depth: usize,
    quote: Option<char>,
    escaped: bool,
}

impl FlowScan {
    /// まだ何も読んでいない状態。最初の `[` / `{` で深さ 1 になる。
    pub(crate) fn start() -> Self {
        Self {
            depth: 0_usize,
            quote: None,
            escaped: false,
        }
    }

    /// `text` を読み進める。括弧が閉じきったら、**閉じ括弧の次のバイト位置**を返す。
    ///
    /// 閉じないまま `text` が尽きたら `None`。状態は残るので、次の行で続きを読める。
    pub(crate) fn advance(&mut self, text: &str) -> Option<usize> {
        for (index, character) in text.char_indices() {
            if self.step(character) {
                return Some(index.saturating_add(character.len_utf8()));
            }
        }
        None
    }

    /// 1文字進む。ここで括弧が閉じきったら `true`。
    fn step(&mut self, character: char) -> bool {
        if let Some(open) = self.quote {
            self.quote = self.inside_quote(open, character);
            return false;
        }
        match character {
            '\'' | '"' => {
                self.quote = Some(character);
                false
            }
            '[' | '{' => {
                self.depth = self.depth.saturating_add(1_usize);
                false
            }
            ']' | '}' => {
                self.depth = self.depth.saturating_sub(1_usize);
                self.depth == 0_usize
            }
            _ => false,
        }
    }

    /// クォートの内側で1文字進む。閉じたら `None` を返す。
    fn inside_quote(&mut self, open: char, character: char) -> Option<char> {
        if self.escaped {
            self.escaped = false;
            return Some(open);
        }
        if character == '\\' && open == '"' {
            self.escaped = true;
            return Some(open);
        }
        if character == open { None } else { Some(open) }
    }
}

#[cfg(test)]
mod tests {
    use super::FlowScan;

    fn once(text: &str) -> Option<usize> {
        FlowScan::start().advance(text)
    }

    #[test]
    fn a_flow_that_closes_in_line_reports_the_end() {
        assert_eq!(once("[a, b] rest"), Some(6_usize));
        assert_eq!(once("{a: 1}"), Some(6_usize));
    }

    /// 🔴 クォートの中の閉じ括弧は閉じない。ここを見落とすと値が途中で切れる。
    #[test]
    fn brackets_inside_quotes_do_not_close() {
        assert_eq!(once("[a, \"]\", b] x"), Some(11_usize));
        assert_eq!(once("[a, ']'] x"), Some(8_usize));
    }

    #[test]
    fn an_escaped_quote_does_not_close_the_quote() {
        assert_eq!(once("[\"a\\\"]\", b]"), Some(11_usize));
    }

    #[test]
    fn an_open_bracket_leaves_the_scan_open() {
        assert_eq!(once("[a, b"), None);
    }

    /// 行をまたいで続きを読む。**状態が残ることがこの型の存在理由である。**
    #[test]
    fn the_scan_resumes_on_the_next_line() {
        let mut scan = FlowScan::start();
        assert_eq!(scan.advance("[\"a\","), None);
        assert_eq!(scan.advance("  \"b\","), None);
        assert_eq!(scan.advance("]  # note"), Some(1_usize));
    }

    /// クォートも行をまたぐ。閉じていないクォートの中では括弧を数えない。
    #[test]
    fn an_unclosed_quote_carries_over_to_the_next_line() {
        let mut scan = FlowScan::start();
        assert_eq!(scan.advance("[\"a"), None);
        assert_eq!(scan.advance("]] b\","), None);
        assert_eq!(scan.advance("]"), Some(1_usize));
    }
}
