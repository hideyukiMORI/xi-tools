//! GitHub の `owner/name`。

use alloc::string::String;

/// GitHub のリポジトリを指す `owner/name`。
///
/// フィールドは非公開で、生成経路は [`GithubSlug::new`] だけである（RS-001 / RS-003）。
/// **不正な形を持てない**ので、この型を受け取った側は検証を繰り返さなくてよい。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GithubSlug {
    owner: String,
    name: String,
}

/// GitHub の remote URL として受ける形。
///
/// 🔑 ここに無い形は**黙って推測しない**。GitHub 以外の origin は
/// 「GitHub に無いリポ」として表の 3 列を `n/a` にするのが正しい振る舞いで
/// （設計メモ F-5）、当てずっぽうで owner/name を作ると、そこだけ嘘の行が出る。
const GITHUB_PREFIXES: [&str; 3] = [
    "https://github.com/",
    "ssh://git@github.com/",
    "git@github.com:",
];

impl GithubSlug {
    /// `owner` と `name` から作る。
    ///
    /// どちらかが空、`[A-Za-z0-9_.-]` 以外を含む、`.` か `..` のときは `None`。
    /// 後者はパスとして解決してしまう名前なので、リポジトリ名として受けない。
    #[must_use]
    pub fn new(owner: &str, name: &str) -> Option<Self> {
        if !(is_segment(owner) && is_segment(name)) {
            return None;
        }
        Some(Self {
            owner: String::from(owner),
            name: String::from(name),
        })
    }

    /// 所有者（ユーザまたは組織）。
    #[must_use]
    pub fn owner(&self) -> &str {
        &self.owner
    }

    /// リポジトリ名。
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// `git remote get-url origin` の出力から `owner/name` を読む。
///
/// 末尾の改行と空白は落とす。末尾の `/` は 1 つまで、`.git` は 1 回だけ剥がす。
/// GitHub 以外の host、`owner` だけ、階層が深いものは `None`。
#[must_use]
pub fn parse_remote_url(url: &str) -> Option<GithubSlug> {
    let path = strip_host(url.trim())?;
    let without_slash = path.strip_suffix('/').unwrap_or(path);
    let without_git = without_slash.strip_suffix(".git").unwrap_or(without_slash);
    let (owner, name) = without_git.split_once('/')?;
    GithubSlug::new(owner, name)
}

/// GitHub の host 部分を落として、`owner/name` の部分を返す。
fn strip_host(url: &str) -> Option<&str> {
    GITHUB_PREFIXES
        .iter()
        .find_map(|prefix| url.strip_prefix(prefix))
}

/// `owner` / `name` として受ける形か。
fn is_segment(text: &str) -> bool {
    if text.is_empty() || text == "." || text == ".." {
        return false;
    }
    text.bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'.' || byte == b'-')
}

#[cfg(test)]
mod tests {
    use super::{GithubSlug, parse_remote_url};

    fn slug(url: &str) -> Option<(alloc::string::String, alloc::string::String)> {
        parse_remote_url(url).map(|found| {
            (
                alloc::string::String::from(found.owner()),
                alloc::string::String::from(found.name()),
            )
        })
    }

    fn is_alpha_beta(url: &str) -> bool {
        slug(url).is_some_and(|(owner, name)| owner == "alpha" && name == "beta")
    }

    #[test]
    fn reads_the_accepted_forms() {
        let accepted = [
            "https://github.com/alpha/beta",
            "https://github.com/alpha/beta.git",
            "git@github.com:alpha/beta.git",
            "git@github.com:alpha/beta",
            "ssh://git@github.com/alpha/beta",
            "ssh://git@github.com/alpha/beta.git",
        ];
        for url in accepted {
            assert!(is_alpha_beta(url), "{url} を読めなかった");
        }
    }

    /// 末尾の改行・空白、末尾の `/`、`.git/` を落とす。
    #[test]
    fn trims_the_tail() {
        let accepted = [
            "https://github.com/alpha/beta\n",
            "  https://github.com/alpha/beta  \n",
            "https://github.com/alpha/beta/",
            "https://github.com/alpha/beta.git/",
            "https://github.com/alpha/beta.git\n",
        ];
        for url in accepted {
            assert!(is_alpha_beta(url), "{url} を読めなかった");
        }
    }

    #[test]
    fn refuses_what_is_not_a_github_repository() {
        let refused = [
            "https://gitlab.com/alpha/beta",
            "https://github.com/alpha",
            "https://github.com/alpha/beta/extra",
            "https://github.com/al pha/beta",
            "https://github.com/alpha/",
            "https://github.com//beta",
            "http://github.com/alpha/beta",
            "https://github.com/alpha/beta//",
            "",
            "   ",
            "origin",
        ];
        for url in refused {
            assert_eq!(slug(url), None, "{url} を受けてしまった");
        }
    }

    /// `.git` は 1 回だけ剥がす。`beta.git.git` は `beta.git` である。
    #[test]
    fn strips_the_git_suffix_only_once() {
        assert!(
            slug("https://github.com/alpha/beta.git.git")
                .is_some_and(|(_, name)| name == "beta.git")
        );
    }

    #[test]
    fn new_refuses_bad_segments() {
        assert_eq!(GithubSlug::new("", "beta"), None);
        assert_eq!(GithubSlug::new("alpha", ""), None);
        assert_eq!(GithubSlug::new("al pha", "beta"), None);
        assert_eq!(GithubSlug::new("alpha", "be/ta"), None);
        assert_eq!(GithubSlug::new("alpha", "be~ta"), None);
        assert_eq!(GithubSlug::new(".", "beta"), None);
        assert_eq!(GithubSlug::new("..", "beta"), None);
        assert_eq!(GithubSlug::new("alpha", "."), None);
        assert_eq!(GithubSlug::new("alpha", ".."), None);
    }

    #[test]
    fn new_accepts_the_allowed_characters() {
        let found = GithubSlug::new("Alpha_9", "be-ta.rs").expect("受けるはずである");
        assert_eq!(found.owner(), "Alpha_9");
        assert_eq!(found.name(), "be-ta.rs");
    }
}
