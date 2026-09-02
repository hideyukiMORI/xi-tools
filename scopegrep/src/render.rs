//! ヒット1件を1行の文字列にする。**出力はしない**（それは `output` の仕事）。
//!
//! 🔑 組み立てと書き出しを分けてあるので、**出力の形はテストで固定できる**。
//! 実際に固定しているのは `tests/cli.rs` の完全一致テストである。

use std::path::Path;

use scopegrep_core::hit::Hit;
use scopegrep_core::hit_kind::HitKind;

/// 人向けの1行。`grep -n` と同じ `<file>:<line>:` で始める。
///
/// 🔴 コメントのヒットには `#comment` を必ず添える。**種別を落とすと、出力は
/// 行ベースの検索と同じ「同じ重みで並んだ5行」に戻る**。
pub(crate) fn human(file: &Path, hit: &Hit) -> String {
    format!(
        "{}:{}: {} = {}",
        file.display(),
        hit.line(),
        head(hit),
        hit.value()
    )
}

/// 所属と種別の見出し。
///
/// ルートに書かれたコメントは所属が空なので、`#comment` だけが残る
/// （JSON Pointer の `""` が「文書全体」を指すのと同じ扱い・RFC 6901）。
fn head(hit: &Hit) -> String {
    let scope = format!("{}", hit.path());
    match hit.kind() {
        HitKind::Value => scope,
        HitKind::Comment if scope.is_empty() => "#comment".to_owned(),
        HitKind::Comment => format!("{scope} #comment"),
    }
}

/// 機械向けの1行。**キーは常に8つ・この順**である（設計メモ D-4）。
///
/// `label` が無ければ `null` を置く。`kind` は `--comments` を付けていなくても
/// 常に出る。キーを落とすと、受け手が「無い」と「そもそも出さない」を
/// 区別できなくなる。
pub(crate) fn json(file: &Path, hit: &Hit) -> String {
    let scope = hit.path();
    let label = match scope.label() {
        Some(text) => quote(text),
        None => "null".to_owned(),
    };
    format!(
        "{{\"file\":{},\"line\":{},\"column\":{},\"pointer\":{},\"path\":{},\"label\":{},\"value\":{},\"kind\":{}}}",
        quote(&file.display().to_string()),
        hit.line(),
        hit.column(),
        quote(&scope.pointer()),
        quote(&format!("{scope}")),
        label,
        quote(hit.value()),
        quote(kind(hit.kind()))
    )
}

/// 種別の機械向けの名前。
fn kind(found: HitKind) -> &'static str {
    match found {
        HitKind::Value => "value",
        HitKind::Comment => "comment",
    }
}

/// RFC 8259 の文字列。**手書きで足りる**（`serde_json` は ADR になる）。
///
/// 退避するのは `"`・`\`・制御文字だけである。それ以外は原文のまま置く。
fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len().saturating_add(2_usize));
    out.push('"');
    for character in text.chars() {
        push_escaped(&mut out, character);
    }
    out.push('"');
    out
}

/// 1文字を JSON の文字列として積む。
fn push_escaped(out: &mut String, character: char) {
    match character {
        '"' => out.push_str("\\\""),
        '\\' => out.push_str("\\\\"),
        '\n' => out.push_str("\\n"),
        '\r' => out.push_str("\\r"),
        '\t' => out.push_str("\\t"),
        '\u{08}' => out.push_str("\\b"),
        '\u{0c}' => out.push_str("\\f"),
        control if control < '\u{20}' => push_hex(out, u32::from(control)),
        plain => out.push(plain),
    }
}

/// 制御文字を `\u00XX` の形で積む。
///
/// 🔑 `format!` を使わないのは、1文字のために String を1つ作る理由が無いからである
/// （`clippy::format_push_string` もそれを言う）。ここに来る値は 0x00..0x1F だけ。
fn push_hex(out: &mut String, code: u32) {
    out.push_str("\\u00");
    out.push(hex_digit(code / 16_u32));
    out.push(hex_digit(code % 16_u32));
}

/// 16 進の1桁。`0..16` の外は来ない。
fn hex_digit(value: u32) -> char {
    char::from_digit(value, 16_u32).unwrap_or('0')
}

#[cfg(test)]
mod tests {
    use super::{human, json, quote};
    use scopegrep_core::hit::Hit;
    use scopegrep_core::search_scope::SearchScope;
    use std::path::Path;

    fn only(source: &str, needle: &str) -> Hit {
        pick(source, needle, SearchScope::Values)
    }

    fn pick(source: &str, needle: &str, scope: SearchScope) -> Hit {
        let document = scopegrep_core::parse(source).expect("読めるはず");
        let found = document.search(needle, scope);
        assert_eq!(found.len(), 1, "ヒットは1件のはず");
        found.into_iter().next().expect("1件ある")
    }

    #[test]
    fn a_human_line_starts_like_grep_n() {
        let hit = only("steps:\n  - name: Build\n    if: target\n", "target");
        assert_eq!(
            human(Path::new("a/b.yml"), &hit),
            "a/b.yml:3: steps[0] \"Build\" .if = target"
        );
    }

    /// コメントのヒットには `#comment` を添える。
    #[test]
    fn a_comment_line_says_it_is_a_comment() {
        let hit = pick(
            "steps:\n  # target\n  - name: Build\n",
            "target",
            SearchScope::ValuesAndComments,
        );
        assert_eq!(
            human(Path::new("a/b.yml"), &hit),
            "a/b.yml:2: steps #comment = # target"
        );
    }

    /// ルートに書かれたコメントは所属が空なので、`#comment` だけが残る。
    #[test]
    fn a_comment_at_the_root_has_no_path_before_the_marker() {
        let hit = pick("# target\na: b\n", "target", SearchScope::ValuesAndComments);
        assert_eq!(
            human(Path::new("a/b.yml"), &hit),
            "a/b.yml:1: #comment = # target"
        );
    }

    #[test]
    fn a_json_line_has_eight_keys_in_a_fixed_order() {
        let hit = only("steps:\n  - name: Build\n    if: target\n", "target");
        assert_eq!(
            json(Path::new("a/b.yml"), &hit),
            "{\"file\":\"a/b.yml\",\"line\":3,\"column\":9,\
             \"pointer\":\"/steps/0/if\",\"path\":\"steps[0] \\\"Build\\\" .if\",\
             \"label\":\"Build\",\"value\":\"target\",\"kind\":\"value\"}"
        );
    }

    /// 🔑 `kind` は旗の有無によらず常に出る。キーの数が入力で変わると、
    /// 受け手が「今回は出ていないだけ」と「そういう値だった」を区別できない。
    #[test]
    fn a_comment_hit_is_marked_in_json() {
        let hit = pick("# target\na: b\n", "target", SearchScope::ValuesAndComments);
        assert_eq!(
            json(Path::new("a/b.yml"), &hit),
            "{\"file\":\"a/b.yml\",\"line\":1,\"column\":3,\
             \"pointer\":\"\",\"path\":\"\",\
             \"label\":null,\"value\":\"# target\",\"kind\":\"comment\"}"
        );
    }

    /// ラベルが無くてもキーは落とさない。落とすと受け手が
    /// 「無い」と「そもそも出さない」を区別できなくなる。
    #[test]
    fn a_missing_label_is_null_not_an_absent_key() {
        let hit = only("steps:\n  - if: target\n", "target");
        assert!(json(Path::new("b.yml"), &hit).contains("\"label\":null,"));
    }

    #[test]
    fn quote_escapes_the_two_characters_json_reserves() {
        assert_eq!(quote("a\"b\\c"), "\"a\\\"b\\\\c\"");
    }

    #[test]
    fn quote_escapes_control_characters() {
        assert_eq!(quote("a\tb\nc\r"), "\"a\\tb\\nc\\r\"");
        assert_eq!(quote("\u{1}\u{1f}"), "\"\\u0001\\u001f\"");
        assert_eq!(quote("\u{8}\u{c}"), "\"\\b\\f\"");
    }

    /// 非 ASCII は退避しない。原文のまま置く（RFC 8259 はそれを許す）。
    #[test]
    fn quote_leaves_non_ascii_alone() {
        assert_eq!(quote("説明"), "\"説明\"");
    }
}
