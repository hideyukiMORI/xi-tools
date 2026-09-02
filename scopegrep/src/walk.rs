//! パスを、実際に読むファイルの列に開く。

use std::fs;
use std::path::{Path, PathBuf};

/// ディレクトリを再帰したときに読む拡張子。
///
/// 🔑 **ファイルとして直接渡されたものは拡張子を問わない。**
/// 名前で選ぶのは「何があるか知らない場所」を掘るときだけで、
/// 名指しされたファイルを名前で拒むのは余計なお世話である。
const YAML_EXTENSIONS: [&str; 2] = ["yml", "yaml"];

/// 走査から外すディレクトリ。履歴は我々が書いた設定ではない。
const SKIPPED_DIRECTORY: &str = ".git";

/// パス1つを、読むファイルの列に開く。
///
/// - ディレクトリでなければ**そのまま1件返す**。存在しないパスもここに含まれ、
///   「読めなかった」として読み込み時に報告される。**ここで握り潰さない**
/// - ディレクトリなら再帰して `.yml` / `.yaml` だけを返す
/// - 並びは**パスのバイト順**で決定的である（RS-016）
/// - シンボリックリンクは辿らない。`.git` は掘らない
pub(crate) fn expand(root: &Path) -> Vec<PathBuf> {
    if !root.is_dir() {
        return vec![root.to_path_buf()];
    }
    let mut found = Vec::new();
    descend(root, &mut found);
    found.sort_by(|left, right| {
        left.as_os_str()
            .as_encoded_bytes()
            .cmp(right.as_os_str().as_encoded_bytes())
    });
    found
}

/// `directory` を再帰的に辿って `found` に積む。
///
/// 読めないディレクトリは**そのパス自体を積む**。そうすると読み込み側が
/// 同じ I/O エラーに当たって報告するので、「黙って空になる」経路が無くなる。
fn descend(directory: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        found.push(directory.to_path_buf());
        return;
    };
    for entry in entries.flatten() {
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        if kind.is_dir() {
            if entry.file_name() != SKIPPED_DIRECTORY {
                descend(&path, found);
            }
        } else if kind.is_file() && is_yaml(&path) {
            found.push(path);
        }
    }
}

/// 拡張子が `.yml` / `.yaml` か調べる。
fn is_yaml(path: &Path) -> bool {
    path.extension()
        .and_then(|raw| raw.to_str())
        .is_some_and(|extension| YAML_EXTENSIONS.contains(&extension))
}
