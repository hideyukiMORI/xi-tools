//! 行指向のスキャナ。1行ずつ読んで、桁から入れ子を復元する。
//!
//! 🔑 **コメントを捨てない**ことがこの実装の存在理由である（設計メモ D-2）。
//! 値だけを読むパーサを使うと「コメント内の一致」を設定値と区別できず、
//! この道具の中核が消える。

use alloc::borrow::ToOwned;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use crate::block_header::BlockHeader;
use crate::column::Column;
use crate::comment_line::CommentLine;
use crate::continuation::Continuation;
use crate::document::Document;
use crate::entry_value::EntryValue;
use crate::flow_state::FlowState;
use crate::frame::Frame;
use crate::line_number::LineNumber;
use crate::malformed_input::MalformedInput;
use crate::mapping_entry::{self, MappingEntry};
use crate::parse_error::ParseError;
use crate::parse_error_kind::ParseErrorKind;
use crate::pending_block::PendingBlock;
use crate::pending_flow::PendingFlow;
use crate::scalar_line::ScalarLine;
use crate::scalar_value::{self, ScalarValue};
use crate::scope_path::ScopePath;
use crate::segment::Segment;
use crate::unsupported_syntax::UnsupportedSyntax;

/// 走査の途中の状態。
#[derive(Debug)]
pub(crate) struct Scanner {
    stack: Vec<Frame>,
    scalars: Vec<ScalarLine>,
    comments: Vec<CommentLine>,
    labels: BTreeMap<String, String>,
    pending: Option<Continuation>,
    line: LineNumber,
    started: bool,
    document_started: bool,
}

impl Scanner {
    /// 空の状態から始める。
    fn start() -> Self {
        Self {
            stack: Vec::new(),
            scalars: Vec::new(),
            comments: Vec::new(),
            labels: BTreeMap::new(),
            pending: None,
            line: LineNumber::first(),
            started: false,
            document_started: false,
        }
    }

    /// 1行読む。行番号は必ず1つ進める（エラーでも進めるが、そのまま返す）。
    fn feed(&mut self, line: &str) -> Result<(), ParseError> {
        let result = self.read(line);
        self.line = self.line.advance();
        result
    }

    /// 1行の中身。**続きを待っている行（ブロックスカラー・フロー記法）が最優先**である。
    fn read(&mut self, line: &str) -> Result<(), ParseError> {
        if self.feed_pending(line)? {
            return Ok(());
        }
        let Some(indent) = self.indentation(line)? else {
            return Ok(());
        };
        let content = line.get(indent..).unwrap_or("");
        if content.starts_with('#') {
            self.record_comment(indent, content);
            return Ok(());
        }
        self.structure(line, indent)
    }

    /// 桁を測る。空行だけが `None`（読み飛ばす）。
    ///
    /// 🔑 コメント行の桁も返す。**コメントを捨てないため**に、行全体のコメントも
    /// 「どの桁に書かれたか」を知る必要がある（所属はそこから決まる）。
    fn indentation(&self, line: &str) -> Result<Option<usize>, ParseError> {
        if line.trim().is_empty() {
            return Ok(None);
        }
        let indent = line.len().saturating_sub(line.trim_start().len());
        let prefix = line.get(..indent).unwrap_or("");
        if prefix.contains('\t') {
            return Err(self.error(ParseErrorKind::Malformed(MalformedInput::TabIndentation)));
        }
        if prefix.chars().any(|c| c != ' ') {
            return Err(self.error(ParseErrorKind::Malformed(
                MalformedInput::InconsistentIndentation,
            )));
        }
        Ok(Some(indent))
    }

    /// 構造を持つ行を読む。
    fn structure(&mut self, line: &str, indent: usize) -> Result<(), ParseError> {
        let content = line.get(indent..).unwrap_or("");
        if self.marker(content)? {
            return Ok(());
        }
        self.started = true;
        if content == "-" || content.starts_with("- ") {
            self.align(indent, true)?;
            return self.sequence_item(line, indent);
        }
        if opens_flow(content) {
            return self.nested_flow(line, indent);
        }
        let found = mapping_entry::parse(line, indent).map_err(|kind| self.error(kind))?;
        let Some(entry) = found else {
            return Err(self.error(ParseErrorKind::Unsupported(
                UnsupportedSyntax::MultiLineScalar,
            )));
        };
        self.align(indent, false)?;
        let comment = entry.comment();
        self.apply_entry(entry, indent)?;
        self.record_trailing(line, comment);
        Ok(())
    }

    /// `key:` の**次の行**に置かれたフロー記法を、そのキーの値として読む。
    ///
    /// compose の `healthcheck.test:` がこの形で、実ファイル計測では
    /// 読めなかった 18 件のうち 11 件がこれだった（設計メモ）。
    /// 段は積まない。**これは入れ子ではなく、直前のキーの値**である。
    fn nested_flow(&mut self, line: &str, indent: usize) -> Result<(), ParseError> {
        let inconsistent = ParseErrorKind::Malformed(MalformedInput::InconsistentIndentation);
        let Some(parent) = self
            .stack
            .last()
            .filter(|frame| frame.is_open() && frame.indent() < indent)
            .map(Frame::indent)
        else {
            return Err(self.error(inconsistent));
        };
        if let Some(top) = self.stack.last_mut() {
            top.set_open(false);
        }
        let value = scalar_value::parse(line, indent).map_err(|kind| self.error(kind))?;
        let comment = value.comment();
        self.record_scalar(value, parent);
        self.record_trailing(line, comment);
        Ok(())
    }

    /// ドキュメント境界とディレクティブ。読み飛ばしたら `true`。
    fn marker(&mut self, content: &str) -> Result<bool, ParseError> {
        let unsupported = ParseErrorKind::Unsupported(UnsupportedSyntax::MultipleDocuments);
        if content.starts_with('%') {
            return Err(self.error(ParseErrorKind::Unsupported(UnsupportedSyntax::Directive)));
        }
        if content.starts_with("...") {
            return Err(self.error(unsupported));
        }
        if !content.starts_with("---") {
            return Ok(false);
        }
        if content != "---" || self.started || self.document_started {
            return Err(self.error(unsupported));
        }
        self.document_started = true;
        Ok(true)
    }

    /// 桁に合わせて入れ子の段を出し入れする。**ここが行指向スキャナの心臓**である。
    fn align(&mut self, indent: usize, dash: bool) -> Result<(), ParseError> {
        while self
            .stack
            .last()
            .is_some_and(|frame| frame.indent() > indent)
        {
            self.stack.pop();
        }
        let Some((top_indent, top_is_mapping)) = self
            .stack
            .last()
            .map(|frame| (frame.indent(), frame.is_mapping()))
        else {
            self.stack.push(open_frame(indent, dash));
            return Ok(());
        };
        // 桁が深い＝親の空の値の中身。`steps:` の次に同じ桁の `- ` が来る形も
        // 「マッピングの中にシーケンスが始まる」なので同じ扱いにする。
        if top_indent < indent || (dash && top_is_mapping) {
            return self.push_child(indent, dash);
        }
        if !dash && !top_is_mapping {
            return self.close_sequence(indent);
        }
        Ok(())
    }

    /// 今の項目の中身として、新しい段を積む。
    fn push_child(&mut self, indent: usize, dash: bool) -> Result<(), ParseError> {
        if !self.stack.last().is_some_and(Frame::is_open) {
            return Err(self.error(ParseErrorKind::Malformed(
                MalformedInput::InconsistentIndentation,
            )));
        }
        if let Some(top) = self.stack.last_mut() {
            top.set_open(false);
        }
        self.stack.push(open_frame(indent, dash));
        Ok(())
    }

    /// 同じ桁にキーが来たらシーケンスは終わる。戻り先はマッピングでなければならない。
    fn close_sequence(&mut self, indent: usize) -> Result<(), ParseError> {
        self.stack.pop();
        let back = self
            .stack
            .last()
            .is_some_and(|frame| frame.indent() == indent && frame.is_mapping());
        if back {
            Ok(())
        } else {
            Err(self.error(ParseErrorKind::Malformed(
                MalformedInput::InconsistentIndentation,
            )))
        }
    }

    /// `- ` で始まる行を読む。
    ///
    /// 🔴 要素の先頭がフロー記法（`[` / `{`）なら、**`key: value` として読まない**。
    /// v1 は `- { $ref: '…' }` の `{ $ref` をキーと誤読して「余分な文字」にしていた。
    fn sequence_item(&mut self, line: &str, indent: usize) -> Result<(), ParseError> {
        // タグ（`- !override`）は値ではないので読み飛ばす（v1.1）。
        let start = scalar_value::skip_tag(line, item_start(line, indent));
        let rest = line.get(start..).unwrap_or("");
        if rest.is_empty() || rest.starts_with('#') {
            self.start_item(true);
            // `-` だけの行に付いたコメントは、始まったばかりの要素に属する。
            self.record_trailing(line, (!rest.is_empty()).then_some(start));
            return Ok(());
        }
        if rest == "-" || rest.starts_with("- ") {
            return Err(self.error(ParseErrorKind::Unsupported(
                UnsupportedSyntax::NestedInlineSequence,
            )));
        }
        self.start_item(false);
        if opens_flow(rest) {
            return self.item_scalar(line, start, indent);
        }
        let found = mapping_entry::parse(line, start).map_err(|kind| self.error(kind))?;
        let Some(entry) = found else {
            return self.item_scalar(line, start, indent);
        };
        self.stack.push(Frame::mapping(start));
        let comment = entry.comment();
        self.apply_entry(entry, start)?;
        self.record_trailing(line, comment);
        Ok(())
    }

    /// 要素そのものがスカラー（フロー・クォート・プレーン）である場合。
    fn item_scalar(&mut self, line: &str, start: usize, indent: usize) -> Result<(), ParseError> {
        let value = scalar_value::parse(line, start).map_err(|kind| self.error(kind))?;
        let comment = value.comment();
        self.record_scalar(value, indent);
        self.record_trailing(line, comment);
        Ok(())
    }

    /// シーケンスの次の要素へ進む。
    fn start_item(&mut self, open: bool) {
        if let Some(top) = self.stack.last_mut() {
            top.start_item();
            top.set_open(open);
        }
    }

    /// マッピングの1項目を今の段に当てはめる。
    fn apply_entry(&mut self, entry: MappingEntry, indent: usize) -> Result<(), ParseError> {
        if !self.stack.last().is_some_and(Frame::is_mapping) {
            return Err(self.error(ParseErrorKind::Malformed(
                MalformedInput::InconsistentIndentation,
            )));
        }
        let key = entry.key().to_owned();
        let value = entry.into_value();
        if let Some(top) = self.stack.last_mut() {
            top.set_key(key.clone());
            top.set_open(matches!(value, EntryValue::Empty));
        }
        match value {
            EntryValue::Empty => Ok(()),
            EntryValue::Scalar(scalar) => {
                self.remember_label(&key, &scalar);
                self.record_scalar(scalar, indent);
                Ok(())
            }
            EntryValue::Block(header) => {
                self.begin_block(header, indent);
                Ok(())
            }
        }
    }

    /// シーケンス要素の `name` をラベルとして覚える。
    ///
    /// 🔑 パスに直接書き込まない。`name:` は `if:` より後に書けるので、
    /// **読み終えてから当てはめる**（[`ScopePath::with_labels`]）。
    ///
    /// ラベルは**キーと同じ扱い**で、クォートを1枚外す（設計メモ D-1）。
    fn remember_label(&mut self, key: &str, value: &ScalarValue) {
        if key != "name" {
            return;
        }
        let Some(pointer) = self.element_pointer() else {
            return;
        };
        self.labels
            .insert(pointer, scalar_value::unquote(value.text()).to_owned());
    }

    /// 今いるシーケンス要素の JSON Pointer。シーケンスの中でなければ `None`。
    fn element_pointer(&self) -> Option<String> {
        let count = self.stack.len();
        let parent = self.stack.get(count.checked_sub(2_usize)?)?;
        if parent.is_mapping() {
            return None;
        }
        let segments: Vec<Segment> = self
            .stack
            .iter()
            .take(count.saturating_sub(1_usize))
            .filter_map(Frame::segment)
            .collect();
        Some(ScopePath::new(segments).pointer())
    }

    /// 今の所属パス。
    fn current_path(&self) -> ScopePath {
        ScopePath::new(self.stack.iter().filter_map(Frame::segment).collect())
    }

    /// 行全体のコメントの所属パス。
    ///
    /// 規則は機械的である: **その桁で開いている最も内側のコンテナ**に付ける。
    /// 桁 `indent` の段はその桁に項目が並ぶコンテナなので、パスに足すのは
    /// **その段より外側の要素だけ**である。
    ///
    /// 🔴 「このコメントは誰の説明か」は推測しない。設計メモ「D-2 実測」のとおり、
    /// 木で持つ実装は**直前の兄弟に付けて取り違える**（29〜30 行目のコメントは
    /// `steps[3]` の説明だが、`tree-sitter-yaml` の木では `steps[2]` に付く）。
    ///
    /// 桁が**その段より深い**ときは、まだ段になっていない入れ子の中である。
    /// `steps:` の直後・最初の要素より前に書かれたコメントがこれで、
    /// 段が開くのを待たずに `steps` の中として扱う。
    /// 🔴 ここを見落とすと、**同じ位置のコメントが「最初の要素の前か後か」で
    /// 違う所属になる**（段は次の行を読むまで積まれないため）。
    ///
    /// どの段の桁とも一致しない桁（既に開いているコンテナの項目の桁より浅く、
    /// 親より深い）は**より浅い方＝外側**に付ける。深い方に寄せると、
    /// その桁には何も無いコンテナの中にコメントを置くことになる。
    fn comment_path(&self, indent: usize) -> ScopePath {
        let depth = self
            .stack
            .iter()
            .rposition(|frame| frame.indent() <= indent)
            .map_or(0_usize, |at| self.reached_depth(at, indent));
        ScopePath::new(
            self.stack
                .iter()
                .take(depth)
                .filter_map(Frame::segment)
                .collect(),
        )
    }

    /// コメントの所属パスに使う段数。`at` はその桁で見つけた最も内側の段である。
    ///
    /// その段が入れ子を待っていて（`key:` の直後）、コメントがそれより深い桁にあるなら、
    /// **その項目の中**である。段はまだ積まれていないので、ここで1段ぶん数える。
    fn reached_depth(&self, at: usize, indent: usize) -> usize {
        let pending = self
            .stack
            .get(at)
            .is_some_and(|frame| frame.is_open() && frame.indent() < indent);
        if pending {
            at.saturating_add(1_usize)
        } else {
            at
        }
    }

    /// 行全体のコメントを表に足す。`text` は `#` から行末までの原文である。
    fn record_comment(&mut self, indent: usize, text: &str) {
        let path = self.comment_path(indent);
        // 桁の前は空白だけであることを [`Scanner::indentation`] が確かめている。
        self.comments.push(CommentLine::new(
            path,
            self.line,
            Column::after(indent),
            text.to_owned(),
        ));
    }

    /// 値の後ろの行末コメントを表に足す。`at` は `#` の位置（行頭からのバイト）。
    ///
    /// 所属は**その値のパス**である（行全体のコメントと違い、書かれた桁ではない）。
    fn record_trailing(&mut self, line: &str, at: Option<usize>) {
        let Some(at) = at else {
            return;
        };
        let path = self.current_path();
        let column = Column::after(line.get(..at).unwrap_or("").chars().count());
        let text = line.get(at..).unwrap_or("").to_owned();
        self.comments
            .push(CommentLine::new(path, self.line, column, text));
    }

    /// スカラー1行を表に足す。フロー記法が閉じていなければ、続きを待つ状態に入る。
    ///
    /// `parent_indent` はこの値を導入したキー（または `-`）の桁である。
    /// 続きの行はそれより深くなければならない。
    fn record_scalar(&mut self, value: ScalarValue, parent_indent: usize) {
        let flow = value.flow();
        let path = self.current_path();
        self.record(&path, value);
        match flow {
            FlowState::Complete => {}
            FlowState::Unclosed(scan) => {
                self.pending = Some(Continuation::Flow(PendingFlow::new(
                    path,
                    parent_indent,
                    self.line,
                    scan,
                )));
            }
        }
    }

    /// スカラー1行を、所属パスを指定して表に足す。
    fn record(&mut self, path: &ScopePath, value: ScalarValue) {
        let line = self.line;
        self.scalars.push(ScalarLine::new(
            path.clone(),
            line,
            value.column(),
            value.into_text(),
        ));
    }

    /// ブロックスカラーを開く。
    fn begin_block(&mut self, header: BlockHeader, indent: usize) {
        let path = self.current_path();
        let explicit = header.indent().map(|width| indent.saturating_add(width));
        self.pending = Some(Continuation::Block(PendingBlock::new(
            path, indent, explicit,
        )));
    }

    /// 続きを待っている行を取り込む。取り込んだら `true`。
    ///
    /// 🔑 **ここで取り込む行は構文解析にかけない。** ブロックスカラーの内容の `#` も、
    /// 括弧の中の `#` も、コメントではなく値だからである。
    fn feed_pending(&mut self, line: &str) -> Result<bool, ParseError> {
        let Some(pending) = self.pending.take() else {
            return Ok(false);
        };
        match pending {
            Continuation::Block(block) => Ok(self.feed_block(block, line)),
            Continuation::Flow(flow) => self.feed_flow(flow, line).map(|()| true),
        }
    }

    /// ブロックスカラーの内容行なら取り込む。取り込んだら `true`。
    fn feed_block(&mut self, mut block: PendingBlock, line: &str) -> bool {
        if line.trim().is_empty() {
            if let Some(start) = block.indent() {
                self.record_block_line(&block, line, start);
            }
            self.pending = Some(Continuation::Block(block));
            return true;
        }
        let indent = leading_spaces(line);
        let Some(start) = block_content_start(block.parent_indent(), block.indent(), indent) else {
            // 桁が戻った。ブロックは終わりで、この行は普通に読み直す。
            return false;
        };
        block.set_indent(start);
        self.record_block_line(&block, line, start);
        self.pending = Some(Continuation::Block(block));
        true
    }

    /// ブロックスカラーの内容行を1行ぶん表に足す。
    fn record_block_line(&mut self, block: &PendingBlock, line: &str, start: usize) {
        let text = line.get(start..).unwrap_or("").to_owned();
        self.scalars.push(ScalarLine::new(
            block.path().clone(),
            self.line,
            Column::after(start),
            text,
        ));
    }

    /// 閉じていないフロー記法の続きを1行取り込む。
    ///
    /// 各行は**同じパスの別のスカラー行**になる（ブロックスカラーと同じ扱い）。
    /// 桁が親まで戻ったら、そこで「閉じていない」と分かる。
    ///
    /// # Errors
    ///
    /// 閉じないまま桁が戻ったとき、および閉じ括弧の後ろに値が続くとき。
    fn feed_flow(&mut self, mut flow: PendingFlow, line: &str) -> Result<(), ParseError> {
        if line.trim().is_empty() {
            self.pending = Some(Continuation::Flow(flow));
            return Ok(());
        }
        let indent = leading_spaces(line);
        if indent < flow.parent_indent() {
            // 親より浅い行が来た。ここまで来れば、閉じないことが確定している。
            return Err(unclosed_flow(&flow));
        }
        let rest = line.get(indent..).unwrap_or("");
        let column = Column::after(indent);
        let Some(end) = flow.advance(rest) else {
            let text = rest.to_owned();
            self.record(flow.path(), ScalarValue::new(text, column, None));
            self.pending = Some(Continuation::Flow(flow));
            return Ok(());
        };
        // 閉じた。後ろに許すのは空白と行末コメントだけである（1行のフローと同じ）。
        let value =
            scalar_value::bounded(rest, end, column, indent).map_err(|kind| self.error(kind))?;
        let comment = value.comment();
        self.record(flow.path(), value);
        self.record_trailing(line, comment);
        Ok(())
    }

    /// 今の行のエラーを作る。**行番号を持たないエラーは作れない**。
    fn error(&self, kind: ParseErrorKind) -> ParseError {
        ParseError::new(self.line, kind)
    }

    /// 閉じていないフローが残っていれば、それを報告するエラー。
    fn unfinished(&self) -> Option<ParseError> {
        match self.pending {
            // ブロックスカラーは EOF で終わってよい。閉じ括弧が要るのはフローだけである。
            None | Some(Continuation::Block(_)) => None,
            Some(Continuation::Flow(ref flow)) => Some(unclosed_flow(flow)),
        }
    }

    /// 走査を終えて、ラベルを当てはめた文書にする。
    ///
    /// # Errors
    ///
    /// フロー記法が閉じないままファイルが終わったとき。
    fn finish(self) -> Result<Document, ParseError> {
        if let Some(error) = self.unfinished() {
            return Err(error);
        }
        let labels = self.labels;
        Ok(Document::new(
            self.scalars
                .into_iter()
                .map(|scalar| scalar.with_labels(&labels))
                .collect(),
            self.comments
                .into_iter()
                .map(|comment| comment.with_labels(&labels))
                .collect(),
        ))
    }
}

/// 閉じていないフロー記法のエラー。**行番号は括弧を開いた行**である。
///
/// 🔑 閉じていないと分かるのは後の行（または EOF）だが、直すべきなのは開いた行である。
fn unclosed_flow(flow: &PendingFlow) -> ParseError {
    ParseError::new(
        flow.line(),
        ParseErrorKind::Unsupported(UnsupportedSyntax::UnclosedFlow),
    )
}

/// この内容がフロー記法で始まるか。
fn opens_flow(content: &str) -> bool {
    content.starts_with('[') || content.starts_with('{')
}

/// ブロックスカラーの内容行として取り込むなら、その内容の桁。
///
/// 内容の桁は、指示子が無ければ**最初の内容行が決める**。それより浅い行で block は終わる。
fn block_content_start(parent_indent: usize, known: Option<usize>, indent: usize) -> Option<usize> {
    match known {
        Some(start) => (indent >= start).then_some(start),
        None => (indent > parent_indent).then_some(indent),
    }
}

/// 新しい段。`dash` ならシーケンス。
fn open_frame(indent: usize, dash: bool) -> Frame {
    if dash {
        Frame::sequence(indent)
    } else {
        Frame::mapping(indent)
    }
}

/// `- ` の後ろの、最初の非空白のバイト位置。
fn item_start(line: &str, indent: usize) -> usize {
    let from = indent.saturating_add(1_usize);
    let tail = line.get(from..).unwrap_or("");
    from.saturating_add(
        tail.len()
            .saturating_sub(tail.trim_start_matches(' ').len()),
    )
}

/// 行頭の空白の数。
fn leading_spaces(line: &str) -> usize {
    line.len()
        .saturating_sub(line.trim_start_matches(' ').len())
}

/// 走査を最初から最後まで回す。
///
/// # Errors
///
/// 読める部分集合の外に出た時点で、その行番号とともに返す。
pub(crate) fn run(source: &str) -> Result<Document, ParseError> {
    let text = source.strip_prefix('\u{feff}').unwrap_or(source);
    let mut scanner = Scanner::start();
    for raw in text.split('\n') {
        scanner.feed(raw.strip_suffix('\r').unwrap_or(raw))?;
    }
    scanner.finish()
}

#[cfg(test)]
mod tests {
    use crate::hit::Hit;
    use crate::hit_kind::HitKind;
    use crate::malformed_input::MalformedInput;
    use crate::parse;
    use crate::parse_error::ParseError;
    use crate::parse_error_kind::ParseErrorKind;
    use crate::query::Query;
    use crate::unsupported_syntax::UnsupportedSyntax;
    use alloc::format;
    use alloc::vec::Vec;

    fn hits(source: &str, needle: &str) -> Vec<Hit> {
        parse(source).expect("読める").search(&Query::new(needle))
    }

    fn only(source: &str, needle: &str) -> Hit {
        let found = hits(source, needle);
        assert_eq!(found.len(), 1_usize, "ヒットは1件のはず");
        found.into_iter().next().expect("1件ある")
    }

    fn failure(source: &str) -> ParseError {
        parse(source).expect_err("読めないはず")
    }

    fn unsupported(syntax: UnsupportedSyntax) -> ParseErrorKind {
        ParseErrorKind::Unsupported(syntax)
    }

    fn malformed(input: MalformedInput) -> ParseErrorKind {
        ParseErrorKind::Malformed(input)
    }

    // ── 読めるもの ──────────────────────────────────────────────────────────

    #[test]
    fn a_comment_line_is_not_a_value() {
        assert!(hits("# target\na: b\n", "target").is_empty());
        assert!(hits("  # target\na: b\n", "target").is_empty());
    }

    #[test]
    fn a_trailing_comment_is_not_a_value() {
        let hit = only("a: b # target\n", "b");
        assert_eq!(hit.value(), "b");
        assert!(hits("a: b # target\n", "target").is_empty());
    }

    #[test]
    fn a_hash_inside_quotes_is_a_value() {
        let hit = only("a: \"x # target\"\n", "target");
        assert_eq!(hit.value(), "\"x # target\"");
    }

    #[test]
    fn a_hash_inside_a_block_scalar_is_a_value() {
        let source = "run: |\n  echo one # target\n  echo two\n";
        let hit = only(source, "target");
        assert_eq!(hit.line().get(), 2_u32);
        assert_eq!(hit.value(), "echo one # target");
        // 内容は3桁目から始まり、一致はその 11 文字先にある。
        assert_eq!(hit.column().get(), 14_u32);
    }

    #[test]
    fn each_block_scalar_line_is_its_own_scalar() {
        let source = "run: |\n  one\n  two\n";
        assert_eq!(hits(source, "o").len(), 2_usize);
        assert_eq!(only(source, "two").path().pointer(), "/run");
    }

    #[test]
    fn a_folded_block_with_an_indent_indicator_is_read() {
        let source = "note: >2\n   kept\nnext: x\n";
        let hit = only(source, "kept");
        assert_eq!(hit.value(), " kept");
        assert_eq!(only(source, "x").path().pointer(), "/next");
    }

    /// ブロックスカラーは、桁が戻った行で終わる。次の行は普通に読める。
    #[test]
    fn a_block_scalar_ends_when_the_indent_goes_back() {
        let source = "steps:\n  - run: |\n      echo one\n    if: target\n";
        let hit = only(source, "target");
        assert_eq!(hit.path().pointer(), "/steps/0/if");
        assert_eq!(hit.line().get(), 4_u32);
        assert_eq!(only(source, "echo one").path().pointer(), "/steps/0/run");
    }

    #[test]
    fn a_sequence_may_start_at_the_same_column_as_its_key() {
        let source = "steps:\n- name: a\n  run: target\nafter: b\n";
        let hit = only(source, "target");
        assert_eq!(hit.path().pointer(), "/steps/0/run");
        assert_eq!(format!("{}", hit.path()), "steps[0] \"a\" .run");
        assert_eq!(only(source, "b").path().pointer(), "/after");
    }

    #[test]
    fn a_sequence_item_may_be_a_scalar() {
        let source = "on:\n  - push\n  - target\n";
        let hit = only(source, "target");
        assert_eq!(hit.path().pointer(), "/on/1");
        assert_eq!(format!("{}", hit.path()), "on[1]");
    }

    #[test]
    fn a_dash_alone_opens_a_nested_mapping() {
        let source = "steps:\n  -\n    run: target\n";
        let hit = only(source, "target");
        assert_eq!(hit.path().pointer(), "/steps/0/run");
    }

    #[test]
    fn a_single_line_flow_is_one_scalar() {
        let source = "matrix: [a, target, c]\n";
        let hit = only(source, "target");
        assert_eq!(hit.value(), "[a, target, c]");
        assert_eq!(hit.path().pointer(), "/matrix");
    }

    #[test]
    fn an_empty_value_is_never_a_hit() {
        let source = "on:\n  pull_request:\n";
        assert!(hits(source, "").is_empty());
    }

    #[test]
    fn a_leading_document_marker_is_skipped() {
        let hit = only("---\na: target\n", "target");
        assert_eq!(hit.line().get(), 2_u32);
        assert_eq!(hit.path().pointer(), "/a");
    }

    #[test]
    fn carriage_returns_and_a_bom_are_removed() {
        let hit = only("\u{feff}a: target\r\nb: x\r\n", "target");
        assert_eq!(hit.value(), "target");
        assert_eq!(hit.line().get(), 1_u32);
    }

    #[test]
    fn a_single_quoted_scalar_keeps_its_escapes() {
        let hit = only("a: 'it''s target'\n", "target");
        assert_eq!(hit.value(), "'it''s target'");
    }

    #[test]
    fn a_key_may_be_quoted() {
        let hit = only("\"weird key\":\n  x: target\n", "target");
        assert_eq!(hit.path().pointer(), "/weird key/x");
        assert_eq!(format!("{}", hit.path()), "\"weird key\".x");
    }

    /// 🔴 桁は**文字数**であって、バイト数ではない（設計メモ「検索の意味」）。
    #[test]
    fn the_column_counts_characters_not_bytes() {
        // バイト数なら 19 桁目になる。文字数で数えるので 9 桁目である。
        let hit = only("説明: あいう target\n", "target");
        assert_eq!(hit.column().get(), 9_u32);
        assert_eq!(hit.path().pointer(), "/説明");
    }

    /// `name:` が後に来ても、その要素のラベルになる。
    #[test]
    fn a_label_is_found_even_when_the_name_comes_last() {
        let source = "steps:\n  - if: target\n    name: Later\n";
        let hit = only(source, "target");
        assert_eq!(hit.path().label(), Some("Later"));
        assert_eq!(format!("{}", hit.path()), "steps[0] \"Later\" .if");
    }

    /// ラベルはキーと同じく**クォートを1枚外す**（設計メモ D-1）。
    /// 中の `''` エスケープは解かない。
    #[test]
    fn a_quoted_name_loses_one_layer_of_quotes() {
        let source = "steps:\n  - name: \"Audit (fail)\"\n    if: target\n";
        let hit = only(source, "target");
        assert_eq!(hit.path().label(), Some("Audit (fail)"));
        assert_eq!(format!("{}", hit.path()), "steps[0] \"Audit (fail)\" .if");

        let single = "steps:\n  - name: 'A ''b'''\n    if: target\n";
        assert_eq!(only(single, "target").path().label(), Some("A ''b''"));
    }

    // ── 複数行にまたがるフロー記法（v1.1）──────────────────────────────────

    /// `key:` の次の行に `[` が来る形。compose の `healthcheck.test:` がこれである
    /// （実ファイル計測で 11 件。読めなかった 18 件の中で最多の形）。
    #[test]
    fn a_flow_may_open_on_the_line_after_its_key() {
        let source = concat!(
            "healthcheck:\n",
            "  test:\n",
            "    [\n",
            "      \"CMD-SHELL\",\n",
            "      \"curl -f http://localhost/ || exit 1\"\n",
            "    ]\n",
        );
        let hit = only(source, "CMD-SHELL");
        assert_eq!(hit.path().pointer(), "/healthcheck/test");
        assert_eq!(hit.line().get(), 4_u32);
        assert_eq!(hit.value(), "\"CMD-SHELL\",");
        // 内容は7桁目（`"`）から始まり、一致はその次の桁で始まる。
        assert_eq!(hit.column().get(), 8_u32);
    }

    /// 行内で開いて、次の行で閉じる形（実ファイル計測で 3 件）。
    /// **各行が同じパスの別のスカラー行**になる（ブロックスカラーと同じ扱い）。
    #[test]
    fn a_flow_opened_in_line_continues_on_the_next_line() {
        let source = "  other: [\"CMD\", \"ping\",\n          \"-p${PASSWORD}\"]\n";
        let first = only(source, "ping");
        assert_eq!(first.path().pointer(), "/other");
        assert_eq!(first.line().get(), 1_u32);
        assert_eq!(first.value(), "[\"CMD\", \"ping\",");

        let second = only(source, "PASSWORD");
        assert_eq!(second.path().pointer(), "/other");
        assert_eq!(second.line().get(), 2_u32);
        assert_eq!(second.value(), "\"-p${PASSWORD}\"]");
    }

    /// 括弧の深さとクォートを行をまたいで追う。**クォートの中の `]` は閉じない。**
    #[test]
    fn brackets_and_quotes_are_tracked_across_lines() {
        let source = "a: [\n  \"x ] y\",\n  [1, 2],\n]\nb: target\n";
        assert_eq!(only(source, "x ] y").path().pointer(), "/a");
        assert_eq!(only(source, "[1, 2]").path().pointer(), "/a");
        // フローが閉じた後は、普通に読み続ける。
        assert_eq!(only(source, "target").path().pointer(), "/b");
    }

    /// 🔴 フローの**中**の `#` はコメントではない。値の一部である。
    #[test]
    fn a_hash_inside_a_multi_line_flow_is_a_value() {
        let source = "a: [\n  \"x # target\",\n]\n";
        let hit = only(source, "target");
        assert_eq!(hit.value(), "\"x # target\",");
        assert_eq!(with_comments(source, "target").len(), 1_usize);
    }

    /// 閉じ括弧の**後ろ**の `#` は行末コメントである（`--comments` の対象）。
    #[test]
    fn a_comment_after_the_closing_bracket_is_a_trailing_comment() {
        let source = "a: [\n  1,\n] # target\n";
        assert!(hits(source, "target").is_empty());
        let hit = sole(source, "target");
        assert_eq!(hit.kind(), HitKind::Comment);
        assert_eq!(hit.path().pointer(), "/a");
        assert_eq!(hit.line().get(), 3_u32);
    }

    /// シーケンスの要素として書かれたフローも、複数行にまたがれる。
    #[test]
    fn a_sequence_item_may_hold_a_multi_line_flow() {
        let source = "ports:\n  - [\n      \"8080:80\"\n    ]\n";
        let hit = only(source, "8080:80");
        assert_eq!(hit.path().pointer(), "/ports/0");
        assert_eq!(hit.line().get(), 3_u32);
    }

    /// 閉じないまま終わったら**エラーのまま**である。行番号は `[` を開いた行。
    #[test]
    fn a_flow_that_never_closes_is_rejected() {
        let error = failure("a: [one,\n  two\n");
        assert_eq!(error.kind(), unsupported(UnsupportedSyntax::UnclosedFlow));
        assert_eq!(error.line().get(), 1_u32);
    }

    /// 閉じないまま桁が親より浅くなったら、その時点で「閉じていない」と分かる。
    /// **行番号は括弧を開いた行**である（直すべきなのはそこだから）。
    #[test]
    fn a_flow_that_loses_its_indent_is_rejected() {
        let error = failure("a:\n  b: [one,\nc: two\n");
        assert_eq!(error.kind(), unsupported(UnsupportedSyntax::UnclosedFlow));
        assert_eq!(error.line().get(), 2_u32);
    }

    // ── タグ（v1.1）────────────────────────────────────────────────────────

    /// `!override` の後ろの入れ子を通常どおり読む。
    #[test]
    fn a_tag_before_a_nested_mapping_is_skipped() {
        let source = "environment: !override\n  POSTGRES_DB: target\n";
        let hit = only(source, "target");
        assert_eq!(hit.path().pointer(), "/environment/POSTGRES_DB");
    }

    /// タグの後ろに値があれば、それが値である。
    #[test]
    fn a_tag_before_a_value_is_skipped() {
        assert_eq!(
            only("ports: !reset [target]\n", "target").value(),
            "[target]"
        );
        assert_eq!(only("name: !!str target\n", "target").value(), "target");
    }

    /// 🔴 タグ名は値ではないので、検索に当たらない。
    #[test]
    fn a_tag_name_is_never_a_hit() {
        assert!(hits("name: !!str 123\n", "str").is_empty());
        assert!(hits("environment: !override\n  a: 1\n", "override").is_empty());
    }

    /// シーケンス要素に付いたタグも読み飛ばし、その後ろの入れ子を受ける。
    #[test]
    fn a_tag_on_a_sequence_item_is_skipped() {
        let source = "ports:\n  - !override\n    - 'target'\n";
        let hit = only(source, "target");
        assert_eq!(hit.path().pointer(), "/ports/0/0");
    }

    // ── フローを要素・値に持つ形（v1 のバグ）────────────────────────────────

    /// 🔴 v1 は `{ $ref` を `key: value` の始まりと誤読して「余分な文字」にしていた。
    #[test]
    fn a_sequence_item_may_be_a_flow_mapping() {
        let source = "parameters:\n  - { $ref: '#/components/parameters/IdPath' }\n";
        let hit = only(source, "IdPath");
        assert_eq!(hit.path().pointer(), "/parameters/0");
        assert_eq!(hit.value(), "{ $ref: '#/components/parameters/IdPath' }");
    }

    #[test]
    fn a_sequence_item_may_be_a_flow_sequence_or_a_quoted_scalar() {
        assert_eq!(
            only("on:\n  - [a, target]\n", "target").value(),
            "[a, target]"
        );
        assert_eq!(
            only("on:\n  - \"a: target\"\n", "target").value(),
            "\"a: target\""
        );
    }

    /// マッピングの値側の1行フローは v1 でも読めていた。**テストで固定する。**
    #[test]
    fn a_mapping_value_may_be_a_flow_mapping() {
        let hit = only("a: { b: target }\n", "target");
        assert_eq!(hit.path().pointer(), "/a");
        assert_eq!(hit.value(), "{ b: target }");
    }

    // ── コメント（`Query::including_comments`）────────────────────────

    fn with_comments(source: &str, needle: &str) -> Vec<Hit> {
        parse(source)
            .expect("読める")
            .search(&Query::new(needle).including_comments())
    }

    fn sole(source: &str, needle: &str) -> Hit {
        let found = with_comments(source, needle);
        assert_eq!(found.len(), 1_usize, "ヒットは1件のはず");
        found.into_iter().next().expect("1件ある")
    }

    /// 🔴 既定では今までどおり返さない。**旗を付けたときだけ**種別付きで返る。
    #[test]
    fn a_comment_is_only_returned_when_the_scope_asks_for_it() {
        assert!(hits("# target\na: b\n", "target").is_empty());
        assert_eq!(sole("# target\na: b\n", "target").kind(), HitKind::Comment);
    }

    /// 何も開いていない桁のコメントは**文書全体**に属する。
    /// JSON Pointer では空文字列が文書全体を指す（RFC 6901）。
    #[test]
    fn a_comment_at_the_root_belongs_to_the_whole_document() {
        let hit = sole("# target\na: b\n", "target");
        assert_eq!(hit.path().pointer(), "");
        assert_eq!(format!("{}", hit.path()), "");
        assert_eq!(hit.line().get(), 1_u32);
        assert_eq!(hit.column().get(), 3_u32);
        assert_eq!(hit.value(), "# target");
    }

    /// その桁で開いているコンテナに属する。要素の桁（`- ` の桁）ならシーケンス自身。
    #[test]
    fn a_comment_belongs_to_the_container_open_at_its_indent() {
        let source = "jobs:\n  build:\n    steps:\n      # target\n      - name: x\n";
        let hit = sole(source, "target");
        assert_eq!(hit.path().pointer(), "/jobs/build/steps");
        assert_eq!(format!("{}", hit.path()), "jobs.build.steps");
    }

    /// 要素の中のキーの桁に書かれたコメントは、その要素に属する。
    /// ラベルは値のヒットと同じ表から当てる（同じ場所を2通りに呼ばない）。
    #[test]
    fn a_comment_inside_an_element_belongs_to_that_element() {
        let source = "steps:\n  - name: A\n    # target\n    run: x\n";
        let hit = sole(source, "target");
        assert_eq!(hit.path().pointer(), "/steps/0");
        assert_eq!(format!("{}", hit.path()), "steps[0] \"A\"");
    }

    /// 🔴 段は次の行を読むまで積まれない。素朴に実装すると、**同じ位置のコメントが
    /// 「最初の要素の前」と「要素の間」で違う所属になる**。同じでなければならない。
    #[test]
    fn a_comment_before_the_first_item_belongs_where_one_between_items_does() {
        let before = sole("steps:\n  # target\n  - name: A\n", "target");
        let between = sole("steps:\n  - name: A\n  # target\n  - name: B\n", "target");
        assert_eq!(before.path().pointer(), "/steps");
        assert_eq!(between.path().pointer(), "/steps");
    }

    /// 🔑 どのコンテナの桁とも一致しない桁は**より浅い方＝外側**に付ける。
    /// 深い方に寄せると、その桁ではまだ開いていない入れ子の中に置くことになる。
    #[test]
    fn a_comment_between_two_levels_belongs_to_the_outer_one() {
        let hit = sole("a:\n    b: 1\n   # target\n", "target");
        assert_eq!(hit.path().pointer(), "");
    }

    /// 行末コメントは**その値のパス**に属する（書かれた桁ではない）。
    #[test]
    fn a_trailing_comment_belongs_to_the_value_it_follows() {
        let hit = sole("steps:\n  - name: A\n    run: x # target\n", "target");
        assert_eq!(hit.path().pointer(), "/steps/0/run");
        assert_eq!(hit.line().get(), 3_u32);
        // 桁は**一致の先頭**である（`#` の位置ではない）。
        assert_eq!(hit.column().get(), 14_u32);
        assert_eq!(hit.value(), "# target");
    }

    /// 値が空でも、ブロックの始まりでも、行末コメントは同じように返る。
    #[test]
    fn a_trailing_comment_is_found_after_every_kind_of_value() {
        assert_eq!(sole("on: # target\n", "target").path().pointer(), "/on");
        assert_eq!(
            sole("run: | # target\n  echo\n", "target").path().pointer(),
            "/run"
        );
        assert_eq!(
            sole("on:\n  - # target\n", "target").path().pointer(),
            "/on/0"
        );
    }

    /// 🔴 ブロックスカラーの内容の `#` は**コメントではない**。値のままである。
    #[test]
    fn a_hash_inside_a_block_scalar_stays_a_value() {
        let hit = sole("run: |\n  echo one # target\n", "target");
        assert_eq!(hit.kind(), HitKind::Value);
        assert_eq!(hit.value(), "echo one # target");
    }

    /// 値とコメントは別の表にあるが、返る並びは**行 → 桁**で1本にまとまる。
    #[test]
    fn values_and_comments_come_back_merged_in_source_order() {
        let source = "# target 1\na: target 2 # target 3\nb: target 4\n";
        let found = with_comments(source, "target");
        let places: Vec<(u32, u32)> = found
            .iter()
            .map(|hit| (hit.line().get(), hit.column().get()))
            .collect();
        assert_eq!(places, [(1, 3), (2, 4), (2, 15), (3, 4)]);
        let kinds: Vec<HitKind> = found.iter().map(Hit::kind).collect();
        assert_eq!(
            kinds,
            [
                HitKind::Comment,
                HitKind::Value,
                HitKind::Comment,
                HitKind::Value
            ]
        );
    }

    // ── 読めないもの ────────────────────────────────────────────────────────

    #[test]
    fn an_anchor_is_rejected() {
        let error = failure("a: &base\n  b: 1\n");
        assert_eq!(error.kind(), unsupported(UnsupportedSyntax::Anchor));
        assert_eq!(error.line().get(), 1_u32);
    }

    #[test]
    fn an_alias_is_rejected() {
        let error = failure("a: 1\nb: *base\n");
        assert_eq!(error.kind(), unsupported(UnsupportedSyntax::Alias));
        assert_eq!(error.line().get(), 2_u32);
    }

    #[test]
    fn a_merge_key_is_rejected() {
        let error = failure("a:\n  <<: *base\n");
        assert_eq!(error.kind(), unsupported(UnsupportedSyntax::MergeKey));
        assert_eq!(error.line().get(), 2_u32);
    }

    #[test]
    fn a_continuation_line_is_rejected() {
        let error = failure("a: one\n  two\n");
        assert_eq!(
            error.kind(),
            unsupported(UnsupportedSyntax::MultiLineScalar)
        );
        assert_eq!(error.line().get(), 2_u32);
    }

    #[test]
    fn a_quote_that_does_not_close_is_rejected() {
        let error = failure("a: \"one\n");
        assert_eq!(
            error.kind(),
            unsupported(UnsupportedSyntax::MultiLineScalar)
        );
        assert_eq!(error.line().get(), 1_u32);
    }

    #[test]
    fn a_second_document_is_rejected() {
        let error = failure("a: 1\n---\nb: 2\n");
        assert_eq!(
            error.kind(),
            unsupported(UnsupportedSyntax::MultipleDocuments)
        );
        assert_eq!(error.line().get(), 2_u32);
    }

    #[test]
    fn an_end_of_document_marker_is_rejected() {
        let error = failure("a: 1\n...\n");
        assert_eq!(
            error.kind(),
            unsupported(UnsupportedSyntax::MultipleDocuments)
        );
        assert_eq!(error.line().get(), 2_u32);
    }

    #[test]
    fn a_directive_is_rejected() {
        let error = failure("%YAML 1.2\n---\na: 1\n");
        assert_eq!(error.kind(), unsupported(UnsupportedSyntax::Directive));
        assert_eq!(error.line().get(), 1_u32);
    }

    #[test]
    fn a_complex_key_is_rejected() {
        let error = failure("? a\n: b\n");
        assert_eq!(error.kind(), unsupported(UnsupportedSyntax::ComplexKey));
        assert_eq!(error.line().get(), 1_u32);
    }

    #[test]
    fn a_sequence_nested_inline_is_rejected() {
        let error = failure("a:\n  - - b\n");
        assert_eq!(
            error.kind(),
            unsupported(UnsupportedSyntax::NestedInlineSequence)
        );
        assert_eq!(error.line().get(), 2_u32);
    }

    #[test]
    fn a_tab_indent_is_rejected() {
        let error = failure("a:\n\tb: 1\n");
        assert_eq!(error.kind(), malformed(MalformedInput::TabIndentation));
        assert_eq!(error.line().get(), 2_u32);
    }

    #[test]
    fn a_child_of_a_scalar_is_rejected() {
        let error = failure("a: 1\n  b: 2\n");
        assert_eq!(
            error.kind(),
            malformed(MalformedInput::InconsistentIndentation)
        );
        assert_eq!(error.line().get(), 2_u32);
    }

    #[test]
    fn a_key_that_lands_between_two_levels_is_rejected() {
        let error = failure("a:\n    b: 1\n  c: 2\n");
        assert_eq!(
            error.kind(),
            malformed(MalformedInput::InconsistentIndentation)
        );
        assert_eq!(error.line().get(), 3_u32);
    }

    #[test]
    fn text_after_a_quoted_value_is_rejected() {
        let error = failure("a: \"one\" two\n");
        assert_eq!(error.kind(), malformed(MalformedInput::TrailingContent));
        assert_eq!(error.line().get(), 1_u32);
    }

    #[test]
    fn an_unknown_block_indicator_is_rejected() {
        let error = failure("a: |x\n  one\n");
        assert_eq!(error.kind(), malformed(MalformedInput::BlockScalarHeader));
        assert_eq!(error.line().get(), 1_u32);
    }
}
