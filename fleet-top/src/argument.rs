//! 引数1つの読み方。

/// 引数1つが何であるか。
///
/// 🔑 `--` より前の引数はここで**必ずどれか一つ**に落ちる。
/// 「旗でも位置引数でもない引数」を作らないので、解析に取りこぼしの隙間が無い。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Argument<'a> {
    /// `--stale-days`。**次の引数が日数である**（値を伴う旗）。
    StaleDays,
    /// `--no-github`。
    NoGithub,
    /// `-h` / `--help`。
    Help,
    /// `-V` / `--version`。
    Version,
    /// 走査するディレクトリ。
    Positional(&'a str),
    /// 知らない旗。使い方を出して 2 で終わる。
    Unknown,
}

impl<'a> Argument<'a> {
    /// 引数1つを読む。`--` は呼び手が先に取り除いている。
    ///
    /// 🔑 `-` 1文字だけは旗ではなく位置引数として扱う。
    /// `-` という名前のディレクトリを渡せなくなる理由が無い。
    pub(crate) fn read(text: &'a str) -> Self {
        // 🔑 `--stale-days=N` は受けない。同じ事を書く形が2つあると、
        //    どちらが正かを覚える必要が出る。書き方は「旗の次に値」の一つだけ。
        if text == "--stale-days" {
            return Self::StaleDays;
        }
        if text == "--no-github" {
            return Self::NoGithub;
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

#[cfg(test)]
mod tests {
    use super::Argument;

    #[test]
    fn every_flag_has_one_spelling_pair() {
        assert_eq!(Argument::read("--stale-days"), Argument::StaleDays);
        assert_eq!(Argument::read("--no-github"), Argument::NoGithub);
        assert_eq!(Argument::read("-h"), Argument::Help);
        assert_eq!(Argument::read("--help"), Argument::Help);
        assert_eq!(Argument::read("-V"), Argument::Version);
        assert_eq!(Argument::read("--version"), Argument::Version);
    }

    /// 🔑 `-` 1文字はパスである。知らない旗と区別する。
    #[test]
    fn a_lone_dash_is_a_path_not_a_flag() {
        assert_eq!(Argument::read("-"), Argument::Positional("-"));
        assert_eq!(Argument::read("."), Argument::Positional("."));
        assert_eq!(Argument::read("-x"), Argument::Unknown);
        assert_eq!(Argument::read("--stale-days=7"), Argument::Unknown);
    }
}
