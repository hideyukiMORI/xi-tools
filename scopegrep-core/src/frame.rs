//! 入れ子の1段。

use alloc::string::String;

use crate::frame_kind::FrameKind;
use crate::segment::Segment;

/// 入れ子の1段。**桁（インデント）が段を決める**というのが行指向スキャナの中核である。
#[derive(Debug, Clone)]
pub(crate) struct Frame {
    indent: usize,
    kind: FrameKind,
    open: bool,
}

impl Frame {
    /// 桁 `indent` のマッピングを始める。
    pub(crate) fn mapping(indent: usize) -> Self {
        Self {
            indent,
            kind: FrameKind::Mapping { key: None },
            open: false,
        }
    }

    /// 桁 `indent` のシーケンスを始める。
    pub(crate) fn sequence(indent: usize) -> Self {
        Self {
            indent,
            kind: FrameKind::Sequence { index: None },
            open: false,
        }
    }

    /// この段の桁。
    pub(crate) fn indent(&self) -> usize {
        self.indent
    }

    /// マッピングか。
    pub(crate) fn is_mapping(&self) -> bool {
        match self.kind {
            FrameKind::Mapping { .. } => true,
            FrameKind::Sequence { .. } => false,
        }
    }

    /// 今の項目が空の値で終わっており、入れ子を受け取れるか。
    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    /// 入れ子を受け取れるかを設定する。
    pub(crate) fn set_open(&mut self, open: bool) {
        self.open = open;
    }

    /// マッピングのキーを読んだ。シーケンスなら何もしない。
    pub(crate) fn set_key(&mut self, key: String) {
        match self.kind {
            FrameKind::Mapping { key: ref mut slot } => *slot = Some(key),
            FrameKind::Sequence { .. } => {}
        }
    }

    /// シーケンスの次の要素へ進む。マッピングなら何もしない。
    pub(crate) fn start_item(&mut self) {
        match self.kind {
            FrameKind::Mapping { .. } => {}
            FrameKind::Sequence { ref mut index } => {
                *index = Some(index.map_or(0_usize, |current| current.saturating_add(1_usize)));
            }
        }
    }

    /// 今の項目が所属パスに足す要素。まだ何も読んでいなければ `None`。
    pub(crate) fn segment(&self) -> Option<Segment> {
        match self.kind {
            FrameKind::Mapping { ref key } => key.clone().map(Segment::Key),
            FrameKind::Sequence { index } => index.map(|value| Segment::Index {
                index: value,
                label: None,
            }),
        }
    }
}
