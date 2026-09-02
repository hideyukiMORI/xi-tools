//! パスを、実際に読むファイルの列に開く。

use std::fs;
use std::path::{Path, PathBuf};

/// ディレクトリを再帰したときに読む拡張子。
///
/// 🔑 **ファイルとして直接渡されたものは拡張子を問わない。**
/// 名前で選ぶのは「何があるか知らない場所」を掘るときだけで、
/// 名指しされたファイルを名前で拒むのは余計なお世話である。
const YAML_EXTENSIONS: [&str; 2] = ["yml", "yaml"];

/// 再帰のときに入らないディレクトリ。**固定リストで、旗では変えられない。**
///
/// 🔴 数がこのリストを決めた（設計リナの実測・2026-09-02）。手元の自前の
/// `.yml` / `.yaml` が **188 件**なのに対し、`node_modules` 配下に **3,206 件**、
/// `vendor` 配下に **3,837 件**あった。**依存の設定は我々が書いた設定ではない**ので、
/// 既定で掘ると、出力のほぼ全部が他人のファイルになる。
///
/// - `.git` は履歴であって設定ではない
/// - `target` / `.venv` はビルド生成物と仮想環境
/// - `dist` は**入れない**。施主の実フォルダ名と衝突する（自前の成果物が消える）
///
/// 🔴 **名指しされたパスには効かない**（[`expand`] の根は検査しない）。
/// `scopegrep x node_modules/foo/` は読む。除外は「何があるか知らない場所を
/// 掘るとき」の話であって、「読ませない」という禁止ではない。
pub(crate) const SKIPPED_DIRECTORIES: [&str; 5] =
    [".git", "node_modules", "vendor", "target", ".venv"];

/// パス1つを、読むファイルの列に開く。
///
/// - ディレクトリでなければ**そのまま1件返す**。存在しないパスもここに含まれ、
///   「読めなかった」として読み込み時に報告される。**ここで握り潰さない**
/// - ディレクトリなら再帰して `.yml` / `.yaml` だけを返す
/// - 並びは**パスのバイト順**で決定的である（RS-016）
/// - シンボリックリンクは辿らない。[`SKIPPED_DIRECTORIES`] は掘らない
pub(crate) fn expand(root: &Path) -> Vec<PathBuf> {
    if !readable(root).is_dir() {
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

/// 読むときのパス。**空のパスは「今いる場所」を指す**。
///
/// 🔑 空のまま名前を組み立てると `a.yml`、`.` から組み立てると `./a.yml` になる。
/// パスを省略した起動で `./` を付けないのは、`grep -rn x` がそうだからである。
fn readable(path: &Path) -> &Path {
    if path.as_os_str().is_empty() {
        Path::new(".")
    } else {
        path
    }
}

/// `directory` を再帰的に辿って `found` に積む。
///
/// 読めないディレクトリは**そのパス自体を積む**。そうすると読み込み側が
/// 同じ I/O エラーに当たって報告するので、「黙って空になる」経路が無くなる。
fn descend(directory: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(readable(directory)) else {
        found.push(directory.to_path_buf());
        return;
    };
    for entry in entries.flatten() {
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        // 🔑 `entry.path()` ではなく自分で繋ぐ。空のパスから始めた走査に
        //    `./` が混ざらないようにするためである。
        let path = directory.join(entry.file_name());
        if kind.is_dir() {
            if !is_skipped(&entry.file_name()) {
                descend(&path, found);
            }
        } else if kind.is_file() && is_yaml(&path) {
            found.push(path);
        }
    }
}

/// 名前が [`SKIPPED_DIRECTORIES`] にあるか調べる。
fn is_skipped(name: &std::ffi::OsStr) -> bool {
    name.to_str()
        .is_some_and(|text| SKIPPED_DIRECTORIES.contains(&text))
}

/// 拡張子が `.yml` / `.yaml` か調べる。
fn is_yaml(path: &Path) -> bool {
    path.extension()
        .and_then(|raw| raw.to_str())
        .is_some_and(|extension| YAML_EXTENSIONS.contains(&extension))
}
