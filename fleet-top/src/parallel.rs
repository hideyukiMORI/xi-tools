//! ワーカープール（設計メモ F-3）。**依存 0・`unsafe` なし**。
//!
//! 🔑 `tokio` を入れない。サブプロセスの待ち合わせに非同期ランタイムは要らず、
//! 試作は `std::thread::scope` ＋ `Mutex<VecDeque>` だけで 60 リポジトリを 1.4 秒で
//! 返した（ADR 0003）。全体の壁時計は `gh` の往復で決まるので、**待つ数**が効く。
//!
//! 🔴 **結果はタスクの投入順で返る。** 到着順に積むと、実行ごとに表と stderr の
//! 並びが変わって差分が取れなくなる（RS-016）。

use std::collections::VecDeque;
use std::sync::{Mutex, PoisonError};
use std::thread;

/// 同時に走らせる上限。
///
/// 実測で 32 並列が 126 リクエストを 3.5 秒、GraphQL 分割なら 60 リポジトリで 1.4 秒。
/// これ以上増やしても縮まず、GitHub の secondary rate limit に近づくだけである。
const MAX_WORKERS: usize = 32;

/// 全タスクを並列に実行し、**投入順**の結果を返す。
///
/// ワーカー数は `min(32, タスク数)`。タスクが 0 個なら糸を立てない。
pub(crate) fn map<T, U, F>(tasks: Vec<T>, work: &F) -> Vec<U>
where
    T: Send,
    U: Send,
    F: Fn(T) -> U + Sync,
{
    let count = tasks.len();
    if count == 0_usize {
        return Vec::new();
    }
    let queue: Mutex<VecDeque<(usize, T)>> = Mutex::new(tasks.into_iter().enumerate().collect());
    // 🔑 `Default` は使わない（CNF-001）ので、長さぶんの `None` を明示的に作る。
    let results: Mutex<Vec<Option<U>>> = Mutex::new((0_usize..count).map(|_| None).collect());

    thread::scope(|scope| {
        for _ in 0_usize..count.min(MAX_WORKERS) {
            scope.spawn(|| serve(&queue, &results, work));
        }
    });

    // 🔑 全ての添字が埋まってから `scope` を抜ける（糸の合流は `scope` が待つ）ので、
    //    ここで落ちる要素は無い。`flatten` は「空きがあれば詰める」ではなく
    //    「空きが無いことを型から外す」ために使っている。
    into_inner(results).into_iter().flatten().collect()
}

/// ワーカー1本。キューが空になるまで取り出して実行する。
fn serve<T, U, F>(queue: &Mutex<VecDeque<(usize, T)>>, results: &Mutex<Vec<Option<U>>>, work: &F)
where
    T: Send,
    U: Send,
    F: Fn(T) -> U + Sync,
{
    while let Some((index, task)) = take(queue) {
        let value = work(task);
        store(results, index, value);
    }
}

/// 次のタスクを1つ取り出す。
fn take<T>(queue: &Mutex<VecDeque<(usize, T)>>) -> Option<(usize, T)> {
    lock(queue).pop_front()
}

/// 結果を**投入時の添字**に置く。
fn store<U>(results: &Mutex<Vec<Option<U>>>, index: usize, value: U) {
    if let Some(slot) = lock(results).get_mut(index) {
        *slot = Some(value);
    }
}

/// 錠を取る。
///
/// 🔑 **毒された錠でも中身を取り出す。** 毒は「他の糸が panic した」ことしか意味せず、
/// ここで守っている `VecDeque` / `Vec<Option<U>>` は panic で壊れる不変条件を持たない。
/// `unwrap` は forbid（RS-005）なので、いずれにせよ握り潰せない。
fn lock<T>(guarded: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    guarded.lock().unwrap_or_else(PoisonError::into_inner)
}

/// 錠を捨てて中身を取り出す。
fn into_inner<T>(guarded: Mutex<T>) -> T {
    guarded.into_inner().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::map;
    use std::sync::Mutex;
    use std::thread;
    use std::time::Duration;

    /// 🔴 **投入順で返る。** 各タスクが投入順と逆の時間だけ眠っても、並びは変わらない。
    /// 到着順に積む実装ならここが逆順になる。
    #[test]
    fn results_come_back_in_submission_order() {
        let tasks: Vec<u64> = (0..8_u64).collect();
        let found = map(tasks, &|task: u64| {
            thread::sleep(Duration::from_millis((8 - task) * 10));
            task
        });
        assert_eq!(found, [0, 1, 2, 3, 4, 5, 6, 7]);
    }

    /// タスクが1個ならワーカーも1本。それでも結果は返る。
    #[test]
    fn a_single_task_runs_on_a_single_worker() {
        let observed = Mutex::new(Vec::new());
        let found = map(vec!["only"], &|task: &str| {
            observed
                .lock()
                .expect("取れるはず")
                .push(thread::current().id());
            task.len()
        });
        assert_eq!(found, [4_usize]);
        assert_eq!(observed.lock().expect("取れるはず").len(), 1);
    }

    /// 上限（32）まではワーカーが増える。40 個のタスクを 32 本で捌く。
    #[test]
    fn many_tasks_run_on_at_most_thirty_two_workers() {
        let threads = Mutex::new(std::collections::BTreeSet::new());
        let tasks: Vec<usize> = (0..40_usize).collect();
        let found = map(tasks, &|task: usize| {
            threads
                .lock()
                .expect("取れるはず")
                .insert(format!("{:?}", thread::current().id()));
            task * 2
        });
        assert_eq!(found.len(), 40);
        assert_eq!(found.first(), Some(&0_usize));
        assert_eq!(found.last(), Some(&78_usize));
        let used = threads.lock().expect("取れるはず").len();
        assert!(used <= 32, "32 本を超えて糸を立てている: {used}");
    }

    /// タスクが 0 個なら糸を立てず、空で返る。
    #[test]
    fn no_task_means_no_thread() {
        let found = map(Vec::new(), &|task: u8| task);
        assert!(found.is_empty());
    }
}
