//! `fleet-top` の中核。**文字列を受けて値を返すだけ**の部分である。
//!
//! `fleet-top` は数十の git リポジトリの状態（枝・未コミット・ahead/behind・open PR・
//! CI・古い枝）を 1 画面に出す道具で、その仕事の大半は `git` と `gh` を起動して
//! 出力を読むことにある。**起動・並列・時刻の取得は bin に置き、このクレートには
//! 「その出力をどう読むか」だけを置く**（ARC-003）。
//!
//! ```
//! use fleet_top_core::json_parser;
//!
//! let Ok(value) = json_parser::parse_json(r#"{"totalCount": 2}"#) else {
//!     return;
//! };
//! assert_eq!(
//!     value.get("totalCount").and_then(|count| count.as_number()).and_then(|count| count.as_u64()),
//!     Some(2_u64)
//! );
//! ```
//!
//! # 依存と環境
//!
//! `#![no_std]` ＋ `alloc` で書かれ、依存は 0 である（ARC-003 / ARC-004）。
//! I/O・時刻・環境に**構文的に到達できない**ので、同じ入力からは必ず同じ出力が出る。
//! 「今日」は [`day::Day`] の値として外から受け取る。
//!
//! JSON を手で書いている理由（`serde_json` を採らなかった理由）と、GitHub を
//! 分割した GraphQL で叩く判断は
//! [ADR 0003](https://github.com/hideyukiMORI/xi-tools/blob/main/docs/adr/0003-fleet-top-fetches-github-via-chunked-graphql.md)、
//! 道具全体の設計は
//! [設計メモ](https://github.com/hideyukiMORI/xi-tools/blob/main/docs/design/fleet-top.md)にある。
//!
//! # 今あるもの
//!
//! `git status --porcelain=v2` の読み取り・GraphQL の組み立てと応答の解釈・表の整形は
//! まだ無い。今あるのは JSON（[`json_parser::parse_json`]）・日付（[`day::Day`]）・
//! remote URL（[`github_slug::parse_remote_url`]）の 3 つである。

#![no_std]

extern crate alloc;

pub mod day;
pub mod github_slug;
pub mod json_error;
pub mod json_error_kind;
pub mod json_number;
pub mod json_parser;
pub mod json_value;
