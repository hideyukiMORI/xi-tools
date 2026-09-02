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
    /// `-i` / `--ignore-case`。
    IgnoreCase,
    /// `--scope`。**次の引数がパターンである**（値を伴う旗）。
    Scope,
    /// `-e` / `--regex`。**次の引数が正規表現である**（値を伴う旗）。
    Regex,
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
        if text == "-i" || text == "--ignore-case" {
            return Self::IgnoreCase;
        }
        // 🔑 `--scope=<pattern>` は受けない。同じ事を書く形が2つあると、
        //    どちらが正かを覚える必要が出る。書き方は「旗の次に値」の一つだけ。
        if text == "--scope" {
            return Self::Scope;
        }
        // 🔑 `-e` は `grep` と同じ綴りである。同じ意味の旗に別の名前を付けない。
        if text == "-e" || text == "--regex" {
            return Self::Regex;
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
