//! xi-tools 固有の規約検査（CNF-0xx）。
//!
//! 規則の正本は `docs/coding-rules.md`。lint が見ないもの、つまり
//! **このリポジトリとして守るべきこと**だけをここで見る。
//!
//! 🔴 依存を足さずに書くこと（ARC-004）。現在の依存は 0 で、CNF-0xx は全て構文で判定できる。
//! 型情報が要る規則が出てきた時点で ADR を立てて再検討する。
//!
//! ```console
//! $ cargo run --quiet -p xtask
//! ```

mod check;
mod source_file;
mod violation;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::source_file::SourceFile;
use crate::violation::Violation;

/// リポジトリのルート。`xtask/` の1つ上。
fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf()
}

/// 全ファイルに全検査をかけ、違反をパス順に返す。
fn inspect(files: &[SourceFile], root: &Path) -> Vec<Violation> {
    let known = check::known_rule_ids(files);
    files
        .iter()
        .flat_map(|file| {
            let mut found = check::no_default_construction(file);
            found.extend(check::no_forbidden_constructs(file));
            found.extend(check::one_primary_declaration(file));
            found.extend(check::role_bearing_names(file));
            found.extend(check::suppression_cites_rule(file, &known));
            found.extend(check::document_links_resolve(file, root));
            found
        })
        .collect()
}

// 🔴 出力を行ってよい唯一の場所（RS-014）。この #[expect] が「ここだけが標準出力に
//    書く」という宣言そのものである。検査結果を別の場所から出力しないこと。
#[expect(
    clippy::print_stdout,
    reason = "RS-014: 出力は1箇所に集約する。検査結果の報告点は main だけである"
)]
fn main() -> ExitCode {
    let root = repository_root();
    let files = match source_file::collect(&root) {
        Ok(found) => found,
        Err(error) => {
            println!("xtask: 検査対象を読めなかった: {error}");
            return ExitCode::FAILURE;
        }
    };

    let violations = inspect(&files, &root);
    if violations.is_empty() {
        println!("xtask: 規約違反なし（検査 {} ファイル）", files.len());
        return ExitCode::SUCCESS;
    }

    println!("xtask: 規約違反 {} 件", violations.len());
    for violation in &violations {
        println!("  {violation}");
    }
    ExitCode::FAILURE
}
