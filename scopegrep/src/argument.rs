//! 引数1つの読み方。

/// 引数1つが何であるか。
///
/// 🔑 `--` より前の引数はここで**必ずどれか一つ**に落ちる。
/// 「旗でも位置引数でもない引数」を作らないので、解析に取りこぼしの隙間が無い。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Argument<'a> {
    /// `--json`。
    Json,
    /// `--comments`。
    Comments,
    /// `-h` / `--help`。
    Help,
    /// `-V` / `--version`。
    Version,
    /// needle かパス。
    Positional(&'a str),
    /// 知らない旗。使い方を出して 2 で終わる。
    Unknown,
}

impl<'a> Argument<'a> {
    /// 引数1つを読む。`--` は呼び手が先に取り除いている。
    ///
    /// 🔑 `-` 1文字だけは旗ではなく位置引数として扱う。
    /// `-` という名前のファイルを渡せなくなる理由が無い。
    pub(crate) fn read(text: &'a str) -> Self {
        if text == "--json" {
            return Self::Json;
        }
        if text == "--comments" {
            return Self::Comments;
        }
        if text == "-h" || text == "--help" {
            return Self::Help;
        }
        if text == "-V" || text == "--version" {
            return Self::Version;
        }
        if text.starts_with('-') && text.chars().count() > 1_usize {
            return Self::Unknown;
        }
        Self::Positional(text)
    }
}
