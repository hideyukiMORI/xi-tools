//! porcelain v2 の 1 行が言っていること。

use alloc::string::String;

use crate::divergence::Divergence;
use crate::head::Head;
use crate::porcelain_error_kind::PorcelainErrorKind;

/// `git status --porcelain=v2 --branch` の 1 行を読んだ結果。
///
/// 🔑 **行の読み取りと、状態の組み立てを分ける。** 1 行が何を言っているかを
/// この enum に落としてから状態に反映すると、行ごとの分岐と「どの見出しが揃ったか」の
/// 判断が混ざらない（RS-011 の複雑度が、機能ではなく構造で下がる）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PorcelainLine {
    /// `# branch.head <名前>`。
    Head(Head),
    /// `# branch.upstream <名前>`。
    Upstream(String),
    /// `# branch.ab +A -B`。
    Divergence(Divergence),
    /// 変更・未追跡・衝突のエントリ（`1` `2` `u` `?` で始まる行）。
    Dirty,
    /// 数えない行（`# branch.oid` / 知らない `# branch.*` / `# stash` / `!` の無視ファイル）。
    Ignored,
}

/// 見出しの印。
const HEADER: char = '#';
/// 枝についての見出しの接頭辞。
const BRANCH_FIELD: &str = "branch.";
/// stash の見出し。git が数を足すだけの行で、この道具は使わない。
const STASH_FIELD: &str = "stash";
/// detached HEAD を表す `# branch.head` の値。
const DETACHED: &str = "(detached)";

impl PorcelainLine {
    /// 1 行を読む。行番号は呼び手が付ける（この関数は行の中身しか知らない）。
    pub(crate) fn read(line: &str) -> Result<Self, PorcelainErrorKind> {
        match line.strip_prefix(HEADER) {
            Some(rest) => read_header(rest),
            None => read_entry(line),
        }
    }
}

/// `#` で始まる見出しの行を読む。
///
/// 🔴 **知らない `# branch.*` と `# stash` は無視する。** git は見出しを増やすことがあり、
/// そのたびにこの道具が全リポジトリを `?` にするのは割に合わない。
/// ただし `branch.` でも `stash` でもない見出しは、**porcelain v2 ではない何か**なので拒む。
fn read_header(rest: &str) -> Result<PorcelainLine, PorcelainErrorKind> {
    let body = rest
        .strip_prefix(' ')
        .ok_or(PorcelainErrorKind::UnexpectedLine)?;
    let Some(field) = body.strip_prefix(BRANCH_FIELD) else {
        if body.starts_with(STASH_FIELD) {
            return Ok(PorcelainLine::Ignored);
        }
        return Err(PorcelainErrorKind::UnexpectedLine);
    };
    let (key, value) = field.split_once(' ').unwrap_or((field, ""));
    match key {
        "head" => read_head(value),
        "upstream" => read_upstream(value),
        "ab" => read_divergence(value),
        _ => Ok(PorcelainLine::Ignored),
    }
}

/// `# branch.head` の値を読む。値は git が書いたままで、`/` を含む枝名も通す。
fn read_head(value: &str) -> Result<PorcelainLine, PorcelainErrorKind> {
    if value.is_empty() {
        return Err(PorcelainErrorKind::MalformedHeader);
    }
    if value == DETACHED {
        return Ok(PorcelainLine::Head(Head::Detached));
    }
    Ok(PorcelainLine::Head(Head::Branch(String::from(value))))
}

/// `# branch.upstream` の値を読む。
fn read_upstream(value: &str) -> Result<PorcelainLine, PorcelainErrorKind> {
    if value.is_empty() {
        return Err(PorcelainErrorKind::MalformedHeader);
    }
    Ok(PorcelainLine::Upstream(String::from(value)))
}

/// `# branch.ab` の値（`+A -B`）を読む。形が違えば [`PorcelainErrorKind::MalformedHeader`]。
fn read_divergence(value: &str) -> Result<PorcelainLine, PorcelainErrorKind> {
    value
        .split_once(' ')
        .and_then(|(ahead, behind)| {
            let ahead = count(ahead.strip_prefix('+')?)?;
            let behind = count(behind.strip_prefix('-')?)?;
            Some(PorcelainLine::Divergence(Divergence::new(ahead, behind)))
        })
        .ok_or(PorcelainErrorKind::MalformedHeader)
}

/// 10 進の数。
///
/// 🔴 `str::parse` に任せない。`+1` を通してしまい、`# branch.ab ++1 -0` が読めてしまう
/// （符号を受けるのは `parse` の仕様である）。
fn count(text: &str) -> Option<u32> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse::<u32>().ok()
}

/// 見出し以外の行を読む。先頭の 1 文字だけで種別が決まる（porcelain v2 の定義）。
fn read_entry(line: &str) -> Result<PorcelainLine, PorcelainErrorKind> {
    let Some(first) = line.chars().next() else {
        return Err(PorcelainErrorKind::UnexpectedLine);
    };
    match first {
        // 変更あり（1）・改名/複製（2）・衝突（u）・未追跡（?）。
        '1' | '2' | 'u' | '?' => Ok(PorcelainLine::Dirty),
        // 無視されているファイル。`--ignored` を付けたときだけ出る。数えない。
        '!' => Ok(PorcelainLine::Ignored),
        _ => Err(PorcelainErrorKind::UnexpectedLine),
    }
}

#[cfg(test)]
mod tests {
    use super::PorcelainLine;
    use crate::divergence::Divergence;
    use crate::head::Head;
    use crate::porcelain_error_kind::PorcelainErrorKind;
    use alloc::string::String;

    #[test]
    fn reads_the_head_header() {
        assert_eq!(
            PorcelainLine::read("# branch.head feat/login"),
            Ok(PorcelainLine::Head(Head::Branch(String::from(
                "feat/login"
            ))))
        );
        assert_eq!(
            PorcelainLine::read("# branch.head (detached)"),
            Ok(PorcelainLine::Head(Head::Detached))
        );
    }

    #[test]
    fn reads_the_upstream_and_the_divergence() {
        assert_eq!(
            PorcelainLine::read("# branch.upstream origin/main"),
            Ok(PorcelainLine::Upstream(String::from("origin/main")))
        );
        assert_eq!(
            PorcelainLine::read("# branch.ab +2 -1"),
            Ok(PorcelainLine::Divergence(Divergence::new(2_u32, 1_u32)))
        );
    }

    #[test]
    fn ignores_headers_it_does_not_use() {
        let ignored = [
            "# branch.oid (initial)",
            "# branch.oid be1ac856ed7b0fda91270b20c022e7bda6bf8206",
            "# branch.future something",
            "# stash 2",
            "! ignored.log",
        ];
        for line in ignored {
            assert_eq!(
                PorcelainLine::read(line),
                Ok(PorcelainLine::Ignored),
                "{line} を無視しなかった"
            );
        }
    }

    #[test]
    fn counts_the_dirty_entries() {
        let dirty = [
            "1 .M N... 100644 100644 100644 a7c9 a7c9 notes.md",
            "2 R. N... 100644 100644 100644 0cbf 0cbf R100 new.md\told.md",
            "u UU N... 100644 100644 100644 100644 1111 2222 3333 conflict.txt",
            "? scratch/",
        ];
        for line in dirty {
            assert_eq!(
                PorcelainLine::read(line),
                Ok(PorcelainLine::Dirty),
                "{line} を数えなかった"
            );
        }
    }

    #[test]
    fn refuses_a_malformed_ab_header() {
        let refused = [
            "# branch.ab +x -1",
            "# branch.ab +1",
            "# branch.ab 1 -1",
            "# branch.ab +1 1",
            "# branch.ab ++1 -1",
            "# branch.ab + -1",
            "# branch.ab",
        ];
        for line in refused {
            assert_eq!(
                PorcelainLine::read(line),
                Err(PorcelainErrorKind::MalformedHeader),
                "{line} を受けてしまった"
            );
        }
    }

    #[test]
    fn refuses_empty_header_values() {
        assert_eq!(
            PorcelainLine::read("# branch.head"),
            Err(PorcelainErrorKind::MalformedHeader)
        );
        assert_eq!(
            PorcelainLine::read("# branch.upstream"),
            Err(PorcelainErrorKind::MalformedHeader)
        );
    }

    #[test]
    fn refuses_lines_that_are_not_porcelain_v2() {
        let refused = ["z something", "", "#branch.head main", "# something else"];
        for line in refused {
            assert_eq!(
                PorcelainLine::read(line),
                Err(PorcelainErrorKind::UnexpectedLine),
                "{line} を受けてしまった"
            );
        }
    }
}
