//! ディレクトリ直下の走査（設計メモ F-4）。
//!
//! 🔴 **再帰しない。** 見るのは引数のディレクトリの**直下**だけで、その中で `.git` を
//! 持つものがリポジトリである。深く掘る形（`--depth`）は、フリートの置き方（直下に並ぶ）で
//! 困っていないので作らない。困ってから作る。

use std::fs;
use std::io;
use std::path::Path;

use crate::repository::Repository;

/// git のリポジトリであることの印。
///
/// 🔑 **ディレクトリとは限らない。** `git worktree` で切った作業木では `.git` は
/// 本体を指す**ファイル**である。存在だけを見る。
const GIT_ENTRY: &str = ".git";

/// `root` の直下にある git リポジトリを、**ファイル名のバイト順**で返す。
///
/// 並べ替えは `fleet_top_core::table::render` も行うが、走査の時点で決めておく。
/// 並列に投げる順・stderr に理由を出す順も、これで決まる（RS-016）。
///
/// # Errors
///
/// `root` が存在しない・ディレクトリでない・読む権限が無いときは [`io::Error`]。
pub(crate) fn directory(root: &Path) -> io::Result<Vec<Repository>> {
    let mut found: Vec<Repository> = Vec::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        // 🔑 `is_dir` も `exists` もシンボリックリンクを辿る。リンク先がリポジトリなら
        //    それはリポジトリである（フリートでは実際にリンクで並べる置き方がある）。
        if !(path.is_dir() && path.join(GIT_ENTRY).exists()) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        found.push(Repository::new(name, path));
    }
    found.sort_by(|left, right| left.name().as_bytes().cmp(right.name().as_bytes()));
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::directory;
    use crate::repository::Repository;
    use std::fs;
    use std::path::PathBuf;

    fn temporary(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("fleet-top-scan-{}-{name}", std::process::id()))
    }

    /// `.git` を持つ直下のディレクトリだけが、バイト順で返る。
    ///
    /// `.git` がファイルの場合（worktree）も拾い、再帰はしない。
    #[test]
    fn only_directories_with_a_git_entry_are_repositories() {
        let root = temporary("scan");
        for name in ["beta", "alpha", "Zulu"] {
            fs::create_dir_all(root.join(name).join(".git")).expect("作れるはず");
        }
        fs::create_dir_all(root.join("worktree")).expect("作れるはず");
        fs::write(root.join("worktree/.git"), "gitdir: /elsewhere\n").expect("書けるはず");
        fs::create_dir_all(root.join("not-a-repo")).expect("作れるはず");
        fs::create_dir_all(root.join("nested/inner/.git")).expect("作れるはず");
        fs::write(root.join("loose.txt"), "x").expect("書けるはず");

        let found = directory(&root).expect("読めるはず");
        fs::remove_dir_all(&root).expect("片付けられるはず");

        let names: Vec<&str> = found.iter().map(Repository::name).collect();
        // 🔑 大文字は小文字より前（バイト順）。ロケールで並びが変わらない。
        assert_eq!(names, ["Zulu", "alpha", "beta", "worktree"]);
    }

    #[test]
    fn an_empty_directory_has_no_repositories() {
        let root = temporary("empty");
        fs::create_dir_all(&root).expect("作れるはず");
        let found = directory(&root).expect("読めるはず");
        fs::remove_dir_all(&root).expect("片付けられるはず");
        assert!(found.is_empty());
    }

    #[test]
    fn a_missing_directory_is_an_error() {
        assert!(directory(&temporary("missing")).is_err());
    }
}
