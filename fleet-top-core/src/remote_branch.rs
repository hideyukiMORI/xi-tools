//! GitHub 側のリモート枝 1 本。

use alloc::string::String;

use crate::day::Day;

/// GitHub 側のリモート枝 1 本（名前と、先頭コミットの日）。
///
/// 🔑 時刻ではなく[`Day`]で持つ。表に出るのは「何日前か」だけで、
/// 時分秒を持ち回ると「どの時計で比べるか」がここに漏れてくる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RemoteBranch {
    name: String,
    last_commit: Day,
}

impl RemoteBranch {
    /// 枝名と先頭コミットの日から作る。
    pub(crate) fn new(name: String, last_commit: Day) -> Self {
        Self { name, last_commit }
    }

    /// 枝名（`refs/heads/` を除いた部分）。
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// 先頭コミットの日。
    pub(crate) fn last_commit(&self) -> Day {
        self.last_commit
    }
}

#[cfg(test)]
mod tests {
    use super::RemoteBranch;
    use crate::day::Day;
    use alloc::string::String;

    #[test]
    fn keeps_the_name_and_the_day() {
        let day = Day::parse_iso8601("2026-07-01").expect("読めるはずである");
        let branch = RemoteBranch::new(String::from("feat/login"), day);
        assert_eq!(branch.name(), "feat/login");
        assert_eq!(branch.last_commit(), day);
    }
}
