//! `docs/coding-rules.md` の CNF-0xx を実際に見る検査。
//!
//! 🔴 検査対象は **本番コードだけ** である。`.rs` ファイルは、桁 0 の
//! `#[cfg(test)]` が現れた行より後を見ない。テストは意図的な違反を書く場所であり、
//! そこを検査すると「検査器のテストが検査器に落とされる」ことになる。
//! テストモジュールをファイル末尾に置くのは Rust の慣習なので、この打ち切りで足りる。

use std::collections::BTreeSet;
use std::path::Path;

use crate::source_file::{self, SourceFile};
use crate::violation::Violation;

/// 本番コードの行だけを、1 始まりの行番号付きで返す。
fn production_lines(file: &SourceFile) -> impl Iterator<Item = (usize, &str)> {
    file.numbered_lines()
        .take_while(|&(_, line)| line.trim_end() != "#[cfg(test)]")
}

/// CNF-001 — `Default` はファクトリを迂回する門なので、実装も derive も書かせない。
///
/// RS-003 の裏付け。Rust には Go のようなゼロ値が無いが、`Default` を書けば
/// 自分でゼロ値を作れてしまう。**残った唯一の穴がここである。**
pub(crate) fn no_default_construction(file: &SourceFile) -> Vec<Violation> {
    if !file.has_extension("rs") {
        return Vec::new();
    }
    let mut found = Vec::new();
    for (number, line) in production_lines(file) {
        let trimmed = line.trim();
        let problem = if trimmed.starts_with("#[derive(") && trimmed.contains("Default") {
            Some("Default を derive している")
        } else if trimmed.starts_with("impl Default for") {
            Some("Default を実装している")
        } else if trimmed.contains(concat!("..Default::", "default()")) {
            Some("Default で構造体の穴を埋めている（E0063 を無効化する書き方）")
        } else {
            None
        };
        if let Some(message) = problem {
            found.push(Violation::new(
                "CNF-001",
                file.path(),
                number,
                format!("{message}。生成経路は唯一のファクトリに限る（RS-003）"),
            ));
        }
    }
    found
}

/// CNF-002 が禁じる構文と、その理由。
///
/// 🔴 **検査語を `concat!` で分割して書く。** ここに検出対象をそのまま書くと、
/// 検査器が自分自身を違反として報告する（実測: 初版で7件の自己検出が出た）。
/// ファイル単位の除外で黙らせると、このファイルだけ他の CNF も効かなくなるので採らない。
const FORBIDDEN_CONSTRUCTS: [(&str, &str); 5] = [
    (concat!("dyn ", "Any"), "型の代用に使われる（RS-006）"),
    (
        concat!("Once", "Lock"),
        "可変グローバルの遅延生成である（RS-009）",
    ),
    (
        concat!("Lazy", "Lock"),
        "可変グローバルの遅延生成である（RS-009）",
    ),
    (
        concat!("lazy_", "static"),
        "可変グローバルの遅延生成である（RS-009）",
    ),
    (
        concat!("proc-macro", " = true"),
        "手続きマクロは書かない（RS-010）",
    ),
];

/// CNF-002 — 汎用データバッグ・遅延グローバル・言語マジックを禁じる。
pub(crate) fn no_forbidden_constructs(file: &SourceFile) -> Vec<Violation> {
    if !(file.has_extension("rs") || file.has_extension("toml")) {
        return Vec::new();
    }
    let mut found = Vec::new();
    for (number, line) in production_lines(file) {
        for &(needle, reason) in &FORBIDDEN_CONSTRUCTS {
            if line.contains(needle) {
                found.push(Violation::new(
                    "CNF-002",
                    file.path(),
                    number,
                    format!("`{needle}` を書いている。{reason}"),
                ));
            }
        }
    }
    found
}

/// 型を宣言するキーワード。桁 0 に現れたものだけを主要宣言として数える。
const TYPE_KEYWORDS: [&str; 4] = ["struct ", "enum ", "trait ", "union "];

/// 桁 0 の型宣言なら、その型名を返す。
fn declared_type_name(line: &str) -> Option<&str> {
    if line.starts_with(char::is_whitespace) {
        return None;
    }
    let body = line
        .strip_prefix("pub(crate) ")
        .or_else(|| line.strip_prefix("pub "))
        .unwrap_or(line);
    let rest = TYPE_KEYWORDS
        .iter()
        .find_map(|keyword| body.strip_prefix(keyword))?;
    let name = rest
        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .next()?;
    (!name.is_empty()).then_some(name)
}

/// CNF-003 — 1ファイル1主要宣言。寄せ集めのファイルを作らせない（RS-012）。
pub(crate) fn one_primary_declaration(file: &SourceFile) -> Vec<Violation> {
    if !file.has_extension("rs") {
        return Vec::new();
    }
    let declared: Vec<(usize, &str)> = production_lines(file)
        .filter_map(|(number, line)| declared_type_name(line).map(|name| (number, name)))
        .collect();

    let Some(&(_, first)) = declared.first() else {
        return Vec::new();
    };
    declared
        .iter()
        .skip(1)
        .map(|&(number, name)| {
            Violation::new(
                "CNF-003",
                file.path(),
                number,
                format!(
                    "`{name}` は2つ目の主要宣言（1つ目は `{first}`）。ファイルを分ける（RS-012）"
                ),
            )
        })
        .collect()
}

/// 常に禁止する型名の語尾。文脈次第で妥当な語（`Processor` 等）は入れない。
const FORBIDDEN_TYPE_SUFFIXES: [&str; 5] = ["Manager", "Helper", "Util", "Utils", "Common"];

/// 常に禁止するモジュール名。
const FORBIDDEN_MODULE_NAMES: [&str; 5] = ["utils", "helpers", "managers", "misc", "common"];

/// CNF-004 — 役割を語らない名前を拒む（RS-013）。
pub(crate) fn role_bearing_names(file: &SourceFile) -> Vec<Violation> {
    if !file.has_extension("rs") {
        return Vec::new();
    }
    let mut found = Vec::new();
    found.extend(forbidden_type_names(file));
    found.extend(forbidden_module_name(file));
    found
}

/// 禁止語尾を持つ型宣言を探す。
fn forbidden_type_names(file: &SourceFile) -> Vec<Violation> {
    production_lines(file)
        .filter_map(|(number, line)| {
            let name = declared_type_name(line)?;
            let suffix = FORBIDDEN_TYPE_SUFFIXES
                .iter()
                .find(|candidate| name.ends_with(*candidate))?;
            Some(Violation::new(
                "CNF-004",
                file.path(),
                number,
                format!("型名 `{name}` の語尾 `{suffix}` は役割を語らない（RS-013）"),
            ))
        })
        .collect()
}

/// ファイル名がそのままモジュール名になるので、禁止名を弾く。
fn forbidden_module_name(file: &SourceFile) -> Option<Violation> {
    let stem = Path::new(file.path()).file_stem()?.to_str()?;
    FORBIDDEN_MODULE_NAMES.contains(&stem).then(|| {
        Violation::new(
            "CNF-004",
            file.path(),
            0,
            format!("モジュール名 `{stem}` は役割を語らない（RS-013）"),
        )
    })
}

/// `docs/coding-rules.md` の見出しから、実在する規則 ID を集める。
pub(crate) fn known_rule_ids(files: &[SourceFile]) -> BTreeSet<String> {
    files
        .iter()
        .filter(|file| file.path() == "docs/coding-rules.md")
        .flat_map(SourceFile::numbered_lines)
        .filter_map(|(_, line)| line.strip_prefix("### "))
        .filter_map(|heading| heading.split_whitespace().next())
        .filter(|token| is_rule_id(token))
        .map(str::to_owned)
        .collect()
}

/// `RS-001` の形をしているか調べる。
fn is_rule_id(token: &str) -> bool {
    let Some((prefix, number)) = token.split_once('-') else {
        return false;
    };
    matches!(prefix, "RS" | "ARC" | "QLT" | "CNF")
        && number.len() == 3
        && number.chars().all(|c| c.is_ascii_digit())
}

/// CNF-006a — `#[expect]` の `reason` は、実在する規則 ID を引く（QLT-006）。
///
/// 🔴 「なぜ抑制したか」ではなく「どの規則の、どの例外か」を残させる。
/// 規約に無い規則 ID を引いた抑制は、規約が動いたときに気づけない。
pub(crate) fn suppression_cites_rule(
    file: &SourceFile,
    known: &BTreeSet<String>,
) -> Vec<Violation> {
    if !file.has_extension("rs") {
        return Vec::new();
    }
    let mut found = Vec::new();
    for (number, block) in expect_attributes(file) {
        let cited = block
            .split_once("reason = \"")
            .and_then(|(_, rest)| rest.split_once(':'))
            .map(|(id, _)| id.trim().to_owned());
        match cited {
            Some(id) if known.contains(&id) => {}
            Some(id) => found.push(Violation::new(
                "CNF-006",
                file.path(),
                number,
                format!("`{id}` は docs/coding-rules.md に無い規則 ID である"),
            )),
            None => found.push(Violation::new(
                "CNF-006",
                file.path(),
                number,
                "reason が `<規則 ID>: <理由>` の形になっていない（QLT-006）".to_owned(),
            )),
        }
    }
    found
}

/// `#[expect(...)]` を、複数行にまたがるものも含めて1件ずつ取り出す。
fn expect_attributes(file: &SourceFile) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    let mut open: Option<(usize, String)> = None;
    for (number, line) in production_lines(file) {
        let trimmed = line.trim();
        match open.take() {
            Some((start, mut buffer)) => {
                buffer.push_str(trimmed);
                if trimmed.ends_with(")]") {
                    found.push((start, buffer));
                } else {
                    open = Some((start, buffer));
                }
            }
            None if trimmed.starts_with("#[expect(") => {
                if trimmed.ends_with(")]") {
                    found.push((number, trimmed.to_owned()));
                } else {
                    open = Some((number, trimmed.to_owned()));
                }
            }
            None => {}
        }
    }
    found
}

/// CNF-006b — 文書内の相対リンク先が実在すること。
///
/// 規約・ADR・証明は互いを参照し合う。**リンクが切れた規約は、読まれない規約になる。**
pub(crate) fn document_links_resolve(file: &SourceFile, root: &Path) -> Vec<Violation> {
    if !file.has_extension("md") {
        return Vec::new();
    }
    file.numbered_lines()
        .flat_map(|(number, line)| markdown_links(line).map(move |link| (number, link)))
        .filter(|(_, link)| !link.starts_with("http") && !link.starts_with('#'))
        .filter(|(_, link)| {
            let target = source_file::resolve_sibling(file.path(), link);
            !root.join(target).exists()
        })
        .map(|(number, link)| {
            Violation::new(
                "CNF-006",
                file.path(),
                number,
                format!("リンク先 `{link}` が存在しない"),
            )
        })
        .collect()
}

/// 1行から Markdown のリンク先を取り出す。
fn markdown_links(line: &str) -> impl Iterator<Item = &str> {
    line.split("](")
        .skip(1)
        .filter_map(|rest| rest.split_once(')'))
        .map(|(link, _)| link)
}

/// 中核クレートに要求する宣言。
const NO_STD_ATTRIBUTE: &str = "#![no_std]";

/// マニフェストの `[package] name`。
fn package_name(file: &SourceFile) -> Option<&str> {
    file.numbered_lines()
        .find_map(|(_, line)| line.strip_prefix("name = \""))
        .and_then(|rest| rest.split_once('"'))
        .map(|(name, _)| name)
}

/// 桁 0 の**最初の属性行群**に `#![no_std]` があるか調べる。
///
/// 属性より後ろは見ない。`extern crate alloc;` のような実コードが1行でも来たら
/// そこで打ち切る（クレート属性は先頭にしか書けない）。
fn declares_no_std(file: &SourceFile) -> bool {
    file.numbered_lines()
        .map(|(_, line)| line.trim_end())
        .take_while(|line| line.is_empty() || line.starts_with("//") || line.starts_with("#!["))
        .any(|line| line == NO_STD_ATTRIBUTE)
}

/// CNF-007 — `-core` で終わるクレートは `#![no_std]` を宣言する（ARC-003 / RS-015）。
///
/// 🔑 `no_std` にすると `std::fs` / `std::env` / `std::time` が lint 違反ではなく
/// **名前解決エラー**になる。この宣言が消えた瞬間、環境への到達不能性が黙って失われる。
/// **消えたことに気づく仕掛けがここである。**
pub(crate) fn core_crates_declare_no_std(files: &[SourceFile]) -> Vec<Violation> {
    files
        .iter()
        .filter_map(|manifest| {
            let name = package_name(manifest)?;
            let directory = manifest.path().strip_suffix("Cargo.toml")?;
            if !name.ends_with("-core") {
                return None;
            }
            let library = format!("{directory}src/lib.rs");
            match files.iter().find(|file| file.path() == library) {
                Some(file) if declares_no_std(file) => None,
                Some(_) => Some(Violation::new(
                    "CNF-007",
                    &library,
                    0,
                    format!(
                        "`{name}` は中核クレートだが、先頭の属性に `{NO_STD_ATTRIBUTE}` が無い（ARC-003）"
                    ),
                )),
                None => Some(Violation::new(
                    "CNF-007",
                    manifest.path(),
                    0,
                    format!("`{name}` は中核クレートだが `{library}` が無い（ARC-003）"),
                )),
            }
        })
        .collect()
}

/// ビルド時にコードを生成する経路の名前。
const BUILD_SCRIPT: &str = "build.rs";

/// マニフェストからビルドスクリプトを指す書き方。
const BUILD_MANIFEST_KEY: &str = concat!("build", " = ");

/// CNF-008 — `build.rs` を置かない（RS-010）。
///
/// 🔑 ビルド時のコード生成は「読んだコードと動いたコードが違う」経路そのものである。
/// 依存が持ち込む `build.rs` は止められないが（それは ARC-004 の ADR の仕事）、
/// **自分では書かない**ことは構文で守れる。
pub(crate) fn no_build_script(file: &SourceFile) -> Vec<Violation> {
    if Path::new(file.path())
        .file_name()
        .is_some_and(|name| name == BUILD_SCRIPT)
    {
        return vec![Violation::new(
            "CNF-008",
            file.path(),
            0,
            "ビルドスクリプトである。ビルド時にコードを生成しない（RS-010）".to_owned(),
        )];
    }
    if !file.path().ends_with("Cargo.toml") {
        return Vec::new();
    }
    production_lines(file)
        .filter(|&(_, line)| line.starts_with(BUILD_MANIFEST_KEY))
        .map(|(number, _)| {
            Violation::new(
                "CNF-008",
                file.path(),
                number,
                "マニフェストがビルドスクリプトを指している（RS-010）".to_owned(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 検査対象のふりをするファイルを作る。
    fn fixture(path: &str, text: &str) -> SourceFile {
        SourceFile::new(path.to_owned(), text.to_owned())
    }

    /// 存在しないルート。リンクが必ず解決しないことを保証する。
    fn missing_root() -> &'static Path {
        Path::new("/xtask-test-root-that-does-not-exist")
    }

    // ── CNF-001 ────────────────────────────────────────────────────────────

    #[test]
    fn cnf001_catches_derive_default() {
        let file = fixture("a.rs", "#[derive(Debug, Default)]\nstruct A;\n");
        assert_eq!(no_default_construction(&file).len(), 1);
    }

    #[test]
    fn cnf001_catches_impl_default() {
        let file = fixture("a.rs", "impl Default for A {}\n");
        assert_eq!(no_default_construction(&file).len(), 1);
    }

    #[test]
    fn cnf001_catches_struct_update_syntax() {
        let file = fixture("a.rs", "let a = A { x: 1, ..Default::default() };\n");
        assert_eq!(no_default_construction(&file).len(), 1);
    }

    #[test]
    fn cnf001_ignores_clean_source() {
        let file = fixture("a.rs", "struct A;\nimpl A { fn new() -> Self { Self } }\n");
        assert!(no_default_construction(&file).is_empty());
    }

    /// 🔴 テストモジュールより後ろは見ない。ここが効かないと、検査器自身の
    /// テストが検査器に落とされる。
    #[test]
    fn cnf001_ignores_test_module() {
        let file = fixture(
            "a.rs",
            "struct A;\n#[cfg(test)]\n#[derive(Default)]\nstruct B;\n",
        );
        assert!(no_default_construction(&file).is_empty());
    }

    #[test]
    fn cnf001_ignores_markdown() {
        let file = fixture("a.md", "#[derive(Default)]\n");
        assert!(no_default_construction(&file).is_empty());
    }

    // ── CNF-002 ────────────────────────────────────────────────────────────

    #[test]
    fn cnf002_catches_lazy_global() {
        let file = fixture("a.rs", "static X: OnceLock<u8> = OnceLock::new();\n");
        assert_eq!(no_forbidden_constructs(&file).len(), 1);
    }

    #[test]
    fn cnf002_catches_proc_macro_manifest() {
        let file = fixture("a/Cargo.toml", "[lib]\nproc-macro = true\n");
        assert_eq!(no_forbidden_constructs(&file).len(), 1);
    }

    #[test]
    fn cnf002_ignores_clean_source() {
        let file = fixture("a.rs", "fn f() -> u8 { 1_u8 }\n");
        assert!(no_forbidden_constructs(&file).is_empty());
    }

    // ── CNF-003 ────────────────────────────────────────────────────────────

    #[test]
    fn cnf003_catches_second_declaration() {
        let file = fixture("a.rs", "pub struct A;\nenum B { X }\n");
        assert_eq!(one_primary_declaration(&file).len(), 1);
    }

    #[test]
    fn cnf003_allows_single_declaration() {
        let file = fixture("a.rs", "pub struct A;\nimpl A {}\n");
        assert!(one_primary_declaration(&file).is_empty());
    }

    /// 桁 0 だけを主要宣言として数える。関数の中の型は対象外。
    #[test]
    fn cnf003_ignores_nested_declaration() {
        let file = fixture("a.rs", "pub struct A;\nfn f() {\n    struct Local;\n}\n");
        assert!(one_primary_declaration(&file).is_empty());
    }

    // ── CNF-004 ────────────────────────────────────────────────────────────

    #[test]
    fn cnf004_catches_forbidden_suffix() {
        let file = fixture("a.rs", "pub struct ScopeManager;\n");
        assert_eq!(role_bearing_names(&file).len(), 1);
    }

    #[test]
    fn cnf004_catches_forbidden_module_name() {
        let file = fixture("scopegrep/src/utils.rs", "pub struct Scope;\n");
        assert_eq!(role_bearing_names(&file).len(), 1);
    }

    /// 判断が要る語は機械では拒否しない。ここが緩いことは意図である（RS-013）。
    #[test]
    fn cnf004_allows_context_dependent_name() {
        let file = fixture("a.rs", "pub struct NodeProcessor;\n");
        assert!(role_bearing_names(&file).is_empty());
    }

    // ── CNF-006a ───────────────────────────────────────────────────────────

    fn known() -> BTreeSet<String> {
        ["RS-014".to_owned()].into_iter().collect()
    }

    #[test]
    fn cnf006_accepts_expect_citing_known_rule() {
        let file = fixture(
            "a.rs",
            "#[expect(clippy::print_stdout, reason = \"RS-014: 出力は1箇所\")]\nfn f() {}\n",
        );
        assert!(suppression_cites_rule(&file, &known()).is_empty());
    }

    #[test]
    fn cnf006_catches_unknown_rule_id() {
        let file = fixture(
            "a.rs",
            "#[expect(clippy::print_stdout, reason = \"RS-999: 無い規則\")]\nfn f() {}\n",
        );
        assert_eq!(suppression_cites_rule(&file, &known()).len(), 1);
    }

    #[test]
    fn cnf006_catches_reason_without_rule_id() {
        let file = fixture(
            "a.rs",
            "#[expect(clippy::print_stdout, reason = \"必要だから\")]\nfn f() {}\n",
        );
        assert_eq!(suppression_cites_rule(&file, &known()).len(), 1);
    }

    /// 複数行にまたがる `#[expect]` も1件として読む。
    #[test]
    fn cnf006_reads_multiline_expect() {
        let file = fixture(
            "a.rs",
            "#[expect(\n    clippy::print_stdout,\n    reason = \"RS-999: 無い規則\"\n)]\nfn f() {}\n",
        );
        assert_eq!(suppression_cites_rule(&file, &known()).len(), 1);
    }

    // ── CNF-007 ────────────────────────────────────────────────────────────

    /// `-core` クレート1つ分のファイル並び。
    fn core_crate(library: &str) -> Vec<SourceFile> {
        vec![
            fixture("a-core/Cargo.toml", "[package]\nname = \"a-core\"\n"),
            fixture("a-core/src/lib.rs", library),
        ]
    }

    #[test]
    fn cnf007_catches_core_crate_without_no_std() {
        let files = core_crate("//! 中核。\n\nextern crate alloc;\n");
        assert_eq!(core_crates_declare_no_std(&files).len(), 1);
    }

    /// 属性より後ろに書いても宣言にはならない（クレート属性は先頭にしか書けない）。
    #[test]
    fn cnf007_catches_no_std_after_real_code() {
        let files = core_crate("extern crate alloc;\n#![no_std]\n");
        assert_eq!(core_crates_declare_no_std(&files).len(), 1);
    }

    #[test]
    fn cnf007_allows_core_crate_with_no_std() {
        let files = core_crate("//! 中核。\n\n#![no_std]\n\nextern crate alloc;\n");
        assert!(core_crates_declare_no_std(&files).is_empty());
    }

    /// `-core` で終わらないクレートには要求しない。std を使うのが仕事である。
    #[test]
    fn cnf007_ignores_a_binary_crate() {
        let files = vec![
            fixture("a/Cargo.toml", "[package]\nname = \"a\"\n"),
            fixture("a/src/main.rs", "fn main() {}\n"),
        ];
        assert!(core_crates_declare_no_std(&files).is_empty());
    }

    // ── CNF-008 ────────────────────────────────────────────────────────────

    #[test]
    fn cnf008_catches_a_build_script() {
        let file = fixture("a-core/build.rs", "fn main() {}\n");
        assert_eq!(no_build_script(&file).len(), 1);
    }

    #[test]
    fn cnf008_catches_a_manifest_pointing_at_one() {
        let file = fixture(
            "a/Cargo.toml",
            "[package]\nname = \"a\"\nbuild = \"gen.rs\"\n",
        );
        assert_eq!(no_build_script(&file).len(), 1);
    }

    #[test]
    fn cnf008_ignores_a_clean_manifest() {
        let file = fixture("a/Cargo.toml", "[package]\nname = \"a\"\n");
        assert!(no_build_script(&file).is_empty());
    }

    /// `rebuild.rs` のような名前を巻き込まない。見るのはファイル名そのものである。
    #[test]
    fn cnf008_ignores_a_similar_file_name() {
        let file = fixture("a/src/rebuild.rs", "fn main() {}\n");
        assert!(no_build_script(&file).is_empty());
    }

    // ── CNF-006b ───────────────────────────────────────────────────────────

    #[test]
    fn cnf006_catches_broken_link() {
        let file = fixture("docs/a.md", "[規約](coding-rules.md) を読む\n");
        assert_eq!(document_links_resolve(&file, missing_root()).len(), 1);
    }

    #[test]
    fn cnf006_ignores_external_link() {
        let file = fixture("docs/a.md", "[rust](https://www.rust-lang.org/)\n");
        assert!(document_links_resolve(&file, missing_root()).is_empty());
    }

    #[test]
    fn cnf006_ignores_anchor_link() {
        let file = fixture("docs/a.md", "[節](#section)\n");
        assert!(document_links_resolve(&file, missing_root()).is_empty());
    }
}
