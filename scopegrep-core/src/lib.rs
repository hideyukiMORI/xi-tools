//! `scopegrep` の中核。YAML を読んで「**その値が構造のどこに属するか**」を返す。
//!
//! `grep` は「その行がある」ことしか返さない。設定ファイルで知りたいのは行番号ではなく
//! 所属である。このクレートは所属を持ったヒットを返し、**コメント内の一致は
//! 既定では返さない**。返すときも、値なのかコメントなのかを必ず区別して返す
//! （[`search_scope::SearchScope`] と [`hit_kind::HitKind`]）。
//!
//! ```
//! use scopegrep_core::hit_kind::HitKind;
//! use scopegrep_core::search_scope::SearchScope;
//!
//! let source = "jobs:\n  e2e:\n    steps:\n      - name: Upload\n        if: ${{ !cancelled() }}\n";
//! let Ok(document) = scopegrep_core::parse(source) else {
//!     return;
//! };
//! let hits = document.search("cancelled()", SearchScope::Values);
//! let Some(hit) = hits.first() else {
//!     return;
//! };
//! assert_eq!(hit.path().pointer(), "/jobs/e2e/steps/0/if");
//! assert_eq!(format!("{}", hit.path()), "jobs.e2e.steps[0] \"Upload\" .if");
//! assert_eq!(hit.line().get(), 5);
//! assert_eq!(hit.kind(), HitKind::Value);
//! ```
//!
//! # 何が読めて、何が読めないか
//!
//! 読めるのは**手で書く YAML の部分集合**である。ブロックマッピング・ブロックシーケンス・
//! 1行スカラー（プレーン / `'…'` / `"…"`）・ブロックスカラー（`|` / `>`）・
//! フロー記法（複数行にまたがるものを含む）・タグ（`!override` 等。読み飛ばす）・
//! コメント・先頭の `---` を読む。
//!
//! アンカー・エイリアス・マージキー・複数行のスカラー・閉じないフロー記法・
//! 複数ドキュメントは**読めない**。🔴 **黙って誤読せず [`parse_error::ParseError`] にする。**
//! 部分集合の正本と、この設計に至った理由（却下した案を含む）は
//! [設計メモ](https://github.com/hideyukiMORI/xi-tools/blob/main/docs/design/scopegrep.md)にある。
//!
//! # 依存と環境
//!
//! `#![no_std]` ＋ `alloc` で書かれ、依存は 0 である（ARC-003 / ARC-004）。
//! 時刻・乱数・環境・I/O に**構文的に到達できない**ので、同じ入力からは必ず同じ出力が出る。

#![no_std]

extern crate alloc;

pub mod column;
pub mod document;
pub mod hit;
pub mod hit_kind;
pub mod line_number;
pub mod malformed_input;
pub mod parse_error;
pub mod parse_error_kind;
pub mod scope_path;
pub mod search_scope;
pub mod unsupported_syntax;

mod block_header;
mod comment_line;
mod continuation;
mod entry_value;
mod flow_scan;
mod flow_state;
mod frame;
mod frame_kind;
mod key_span;
mod mapping_entry;
mod pending_block;
mod pending_flow;
mod scalar_line;
mod scalar_value;
mod scanner;
mod segment;

use crate::document::Document;
use crate::parse_error::ParseError;

/// YAML を読んで構造を組み立てる。
///
/// # Errors
///
/// 読める部分集合の外の構文に出会ったら [`ParseError`] を返す。
/// エラーは**必ず行番号と種別を持つ**ので、「何行目の何が読めなかったか」が言える。
pub fn parse(source: &str) -> Result<Document, ParseError> {
    scanner::run(source)
}
