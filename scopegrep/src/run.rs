//! 走査 → 読み込み → 解析 → 検索 → 出力の流れ。
//!
//! 🔴 **エラーはファイル単位で報告して続ける。** 1つ読めないファイルがあったせいで
//! 残りを見ないのは、この道具が生まれた事故（片方だけ見て判断した）と同じ形である。

use std::fs;
use std::path::Path;

use crate::options::Options;
use crate::outcome::Outcome;
use crate::output;

/// 全てのパスを見て、結果をまとめる。
pub(crate) fn search(options: &Options) -> Outcome {
    options
        .paths()
        .iter()
        .flat_map(|root| crate::walk::expand(root))
        .map(|file| examine(&file, options))
        .fold(Outcome::Missing, Outcome::combine)
}

/// ファイル1つを見る。読めなければ報告して `Failed` を返す。
fn examine(file: &Path, options: &Options) -> Outcome {
    let source = match fs::read_to_string(file) {
        Ok(text) => text,
        Err(error) => {
            output::unreadable(file, &error);
            return Outcome::Failed;
        }
    };
    let document = match scopegrep_core::parse(&source) {
        Ok(read) => read,
        Err(error) => {
            output::unparsable(file, &error);
            return Outcome::Failed;
        }
    };
    let hits = document.search(options.query());
    if hits.is_empty() {
        return Outcome::Missing;
    }
    for found in &hits {
        output::hit(file, found, options.format());
    }
    Outcome::Found
}
