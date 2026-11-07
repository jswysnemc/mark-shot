// Selection over recognized text.
//
// The selection is represented as two points: start and end (line + character position).
// Text between them includes full middle lines and partial outer lines.
//
// Uses `best_line` for click/hover detection so mouse clicks and hover effects
// always target the exact same line.

use tiny_skia::Rect;

use super::OcrLine;
use super::bidi;
use super::layout::{self, Block};

/// Character offset `ch` (0..=char count) within `line`, ordered by reading
/// position then offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Caret {
    line: usize,
    ch: usize,
}

/// A line's selected span, for the renderer.
pub struct LineSelection {
    /// Global x of span's left and right edges.
    pub x: (f32, f32),
    /// Vertical extent (line's own top/bottom).
    pub y: (f32, f32),
}

/// Recognized lines, their blocks, and the selection over them. Lives in
/// `EditorState` while the OCR tool holds a result; cleared when the tool is
/// left or a new recognition starts. All coordinates are global.
#[derive(Default)]
pub struct OcrView {
    pub lines: Vec<OcrLine>,
    /// The scanned area (global). Shaded as a whole so it is obvious what was
    /// read - and, when no selection was drawn, which monitor it came from.
    region: Option<Rect>,
    blocks: Vec<Block>,
    /// line index -> block index
    block_of: Vec<usize>,
    /// reading position -> line index
    order: Vec<usize>,
    /// line index -> reading position
    rank: Vec<usize>,
    /// (anchor, focus), not normalized; `ordered()` sorts them.
    sel: Option<(Caret, Caret)>,
    /// Block the current drag began in (confines a sideways drag).
    anchor_block: Option<usize>,
    /// for right-to-left languages
    rtl: Vec<bool>,
}

impl OcrView {
    /// Remember the area a scan covers. Set when recognition starts, so the
    /// progress overlay and the finished result shade the same rectangle.
    pub fn set_region(&mut self, region: Rect) {
        self.region = Some(region);
    }

    pub fn region(&self) -> Option<Rect> {
        self.region
    }

    /// Group lines into blocks and work out the reading order.
    pub fn set_lines(&mut self, lines: Vec<OcrLine>) {
        let blocks = layout::group_blocks(&lines);
        let n = lines.len();

        let mut block_of = vec![0usize; n];
        let mut order = Vec::with_capacity(n);
        for (bi, block) in blocks.iter().enumerate() {
            for &li in &block.lines {
                block_of[li] = bi;
                order.push(li);
            }
        }
        let mut rank = vec![0usize; n];
        for (pos, &li) in order.iter().enumerate() {
            rank[li] = pos;
        }

        self.rtl = lines.iter().map(|line| bidi::is_rtl(&line.text)).collect();
        self.lines = lines;
        self.blocks = blocks;
        self.block_of = block_of;
        self.order = order;
        self.rank = rank;
        self.sel = None;
        self.anchor_block = None;
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn is_active(&self) -> bool {
        !self.lines.is_empty()
    }

    // ── hit testing ──────────────────────────────────────────────────────────

    /// Line under a global point, if close enough to count as hit.
    pub fn line_at(&self, p: (f64, f64)) -> Option<usize> {
        let (x, y) = (p.0 as f32, p.1 as f32);
        let (line, distance) = self.best_line(x, y, false, false)?;
        (distance <= crate::config::get().ocr.hit_slack).then_some(line)
    }

    /// Nearest line, then nearest character boundary within it.
    fn caret_at(&self, p: (f64, f64), confine: bool) -> Option<Caret> {
        let (x, y) = (p.0 as f32, p.1 as f32);
        let (line, _) = self.best_line(x, y, confine, confine)?;
        Some(Caret {
            line,
            ch: nearest_boundary(&self.lines[line].char_x, x),
        })
    }

    // Finds which line a point belongs to and its distance from that line.
    //
    // When `by_row` is true, vertical distance is checked before horizontal distance.
    // This matches how text selection works (dragging diagonally moves to the next line
    // rather than sticking to the horizontally nearest line).
    //
    // If `confine` is set, lines in other blocks are ignored while the pointer stays
    // vertically level with the anchor block. Ties go to the line closer in reading order.
    fn best_line(&self, x: f32, y: f32, by_row: bool, confine: bool) -> Option<(usize, f32)> {
        let slack = crate::config::get().ocr.vertical_slack;
        let anchored = confine && self.anchor_block.is_some();
        let level_with_block = anchored
            && self
                .anchor_block
                .and_then(|bi| self.blocks.get(bi))
                .is_some_and(|b| y >= b.bounds.top() - slack && y <= b.bounds.bottom() + slack);

        let anchor_rank = self.sel.map(|(a, _)| self.rank[a.line]).unwrap_or(0);
        let mut best = None;
        let mut best_key = (f32::MAX, f32::MAX, usize::MAX);
        for (i, line) in self.lines.iter().enumerate() {
            let block = self.block_of[i];
            if anchored && Some(block) != self.anchor_block {
                let over_block = !level_with_block
                    && self.blocks.get(block).is_some_and(|b| {
                        let (bx, by) = gaps(x, y, &b.bounds);
                        bx <= slack && by <= slack
                    });
                if !over_block {
                    continue;
                }
            }
            let (dx, dy) = gaps(x, y, &line.bounds);
            let rank = self.rank[i].abs_diff(anchor_rank);
            let key = if by_row {
                (dy, dx, rank)
            } else {
                (dx.hypot(dy), 0.0, rank)
            };
            if key < best_key {
                best_key = key;
                best = Some(i);
            }
        }
        best.map(|i| (i, best_key.0))
    }

    // ── what the renderer draws ──────────────────────────────────────────────

    pub fn block_bounds(&self) -> impl Iterator<Item = Rect> + '_ {
        self.blocks.iter().map(|block| block.bounds)
    }

    /// Selected span on line `i`, or `None` if untouched. Middle lines give
    /// their whole width, the first and last a partial span.
    pub fn line_selection(&self, i: usize) -> Option<LineSelection> {
        let (lo, hi) = self.span(self.sel, i)?;
        let line = &self.lines[i];
        Some(LineSelection {
            x: (
                line.char_x.get(lo).copied().unwrap_or(line.bounds.left()),
                line.char_x.get(hi).copied().unwrap_or(line.bounds.right()),
            ),
            y: (line.bounds.top(), line.bounds.bottom()),
        })
    }

    /// Everything the overlay covers: the shaded region plus any line a
    /// backend left outside it. Used when the whole overlay has to be repainted -
    /// results arriving, the tool being left.
    pub fn bounds(&self) -> Option<Rect> {
        self.blocks
            .iter()
            .map(|b| b.bounds)
            .chain(self.region)
            .fold(None, layout::union_rect)
    }

    // ── changing the selection ───────────────────────────────────────────────

    // Every mutator reports the area that has to be repainted rather than a
    // bare `changed` flag: the overlay is translucent, so the caller has to
    // restore exactly that area from the dim layer before it is drawn again.
    // Repainting the whole result instead would mean filling the entire
    // scanned region on every mouse move.

    /// Start a drag selection at the caret nearest the pointer.
    pub fn begin_drag(&mut self, pointer: (f64, f64)) -> Option<Rect> {
        let caret = self.caret_at(pointer, false)?;
        let old = self.sel_bounds(self.sel);
        self.anchor_block = self.block_of.get(caret.line).copied();
        self.sel = Some((caret, caret));
        union(old, self.sel_bounds(self.sel))
    }

    /// Expands the selection to the nearest line position under the mouse.
    ///
    /// While dragging horizontally inside the current block, only that block's lines
    /// can be selected (preventing accidental column grabs).
    ///
    /// Once the mouse moves past the top or bottom of the block, selection flows
    /// normally in reading order.
    pub fn extend_drag(&mut self, pointer: (f64, f64)) -> Option<Rect> {
        let (anchor, _) = self.sel?;
        let focus = self.caret_at(pointer, true)?;
        let next = Some((anchor, focus));
        if self.sel == next {
            return None;
        }
        let old = self.sel_bounds(self.sel);
        self.sel = next;
        union(old, self.sel_bounds(self.sel))
    }

    /// Select the whole block a line belongs to (double-click).
    pub fn select_block(&mut self, line: usize) -> Option<Rect> {
        let &bi = self.block_of.get(line)?;
        let block = &self.blocks[bi];
        let (Some(&first), Some(&last)) = (block.lines.first(), block.lines.last()) else {
            return None;
        };
        self.anchor_block = Some(bi);
        self.set_span(first, last)
    }

    /// 【贴图文字】【整行选择】按 line 只选择当前显示行，返回需重绘区域，双击不扩大到识别段落的其他行
    pub fn select_line(&mut self, line: usize) -> Option<Rect> {
        self.lines.get(line)?;
        self.anchor_block = self.block_of.get(line).copied();
        self.set_span(line, line)
    }

    /// 【贴图文字】【逐行复制】按 selected 决定复制当前选择或全文，返回单换行行文本，不在段落之间额外插入空行
    pub fn line_text(&self, selected: bool) -> String {
        self.copy_text_with_separator(if selected { self.sel } else { None }, "\n")
    }

    pub fn select_all(&mut self) -> Option<Rect> {
        self.anchor_block = None;
        let (Some(&first), Some(&last)) = (self.order.first(), self.order.last()) else {
            return None;
        };
        self.set_span(first, last)
    }

    pub fn deselect(&mut self) -> Option<Rect> {
        self.anchor_block = None;
        let old = self.sel_bounds(self.sel);
        self.sel = None;
        old
    }

    /// Select from the start of `first` to the end of `last`.
    fn set_span(&mut self, first: usize, last: usize) -> Option<Rect> {
        let next = Some((
            Caret {
                line: first,
                ch: self.line_start(first),
            },
            Caret {
                line: last,
                ch: self.line_end(last),
            },
        ));
        if self.sel == next {
            return None;
        }
        let old = self.sel_bounds(self.sel);
        self.sel = next;
        union(old, self.sel_bounds(next))
    }

    /// for right-to-left text: the start of the line in reading order is its rightmost character
    fn line_start(&self, line: usize) -> usize {
        if self.rtl[line] {
            self.lines[line].char_count()
        } else {
            0
        }
    }

    fn line_end(&self, line: usize) -> usize {
        if self.rtl[line] {
            0
        } else {
            self.lines[line].char_count()
        }
    }

    /// Bounds of every line a caret pair touches.
    fn sel_bounds(&self, sel: Option<(Caret, Caret)>) -> Option<Rect> {
        let (a, b) = self.ordered(sel)?;
        let (lo, hi) = (self.rank[a.line], self.rank[b.line]);
        self.order[lo..=hi].iter().fold(None, |acc, &li| {
            layout::union_rect(acc, self.lines[li].bounds)
        })
    }

    // ── reading the selection out ────────────────────────────────────────────

    /// Selected text in reading order, blocks separated by a blank line.
    /// With nothing selected, every line in full.
    pub fn text_to_copy(&self) -> String {
        let sel = self.sel.filter(|&(a, b)| a != b);
        self.copy_text(sel)
    }

    /// 【文字识别】【完整原文】按阅读顺序返回全部识别文字；不受当前字符选择影响
    pub fn all_text(&self) -> String {
        self.copy_text(None)
    }

    /// 【文字识别】【非空选择】返回是否实际选中了字符，不把没有选择时的全文复制当作选中文字
    pub fn has_selection(&self) -> bool {
        self.sel.is_some_and(|(anchor, focus)| anchor != focus)
    }

    /// 【文字识别】【文字提取】按 sel 提取原文；空选择提取全部文字，段落之间保留空行
    fn copy_text(&self, sel: Option<(Caret, Caret)>) -> String {
        self.copy_text_with_separator(sel, "\n\n")
    }

    /// 【文字识别】【分隔复制】按 sel 和 paragraph_break 提取当前原文，返回文本，不改变字符选择或双向文字顺序
    fn copy_text_with_separator(
        &self,
        sel: Option<(Caret, Caret)>,
        paragraph_break: &str,
    ) -> String {
        let mut out = String::new();
        let mut prev_block: Option<usize> = None;
        for &li in &self.order {
            let segment = if sel.is_none() {
                bidi::to_logical(&self.lines[li].text)
            } else {
                let Some((lo, hi)) = self.span(sel, li) else {
                    continue;
                };
                let visual: String = self.lines[li].text.chars().skip(lo).take(hi - lo).collect();
                bidi::to_logical(&visual)
            };
            let block = self.block_of[li];
            match prev_block {
                Some(pb) if pb == block => out.push('\n'),
                Some(_) => out.push_str(paragraph_break),
                None => {}
            }
            out.push_str(&segment);
            prev_block = Some(block);
        }
        out
    }

    /// Half-open character range of line `i` covered by `sel`. The only place
    /// a caret pair is interpreted, so drawing and copying stay in step.
    fn span(&self, sel: Option<(Caret, Caret)>, i: usize) -> Option<(usize, usize)> {
        let (a, b) = self.ordered(sel)?;
        let r = *self.rank.get(i)?;
        if r < self.rank[a.line] || r > self.rank[b.line] {
            return None;
        }
        let n = self.lines[i].char_count();
        // for right-to-left languages
        let (lo, hi) = if self.rtl[i] {
            (
                if i == b.line { b.ch } else { 0 },
                if i == a.line { a.ch } else { n },
            )
        } else {
            (
                if i == a.line { a.ch } else { 0 },
                if i == b.line { b.ch } else { n },
            )
        };
        (lo < hi).then_some((lo, hi))
    }

    /// Normalized so the first caret precedes the second.
    fn ordered(&self, sel: Option<(Caret, Caret)>) -> Option<(Caret, Caret)> {
        let (a, b) = sel?;
        if self.caret_key(a) <= self.caret_key(b) {
            Some((a, b))
        } else {
            Some((b, a))
        }
    }

    fn caret_key(&self, c: Caret) -> (usize, usize) {
        let rank = self.rank.get(c.line).copied().unwrap_or(0);
        if self.rtl.get(c.line).copied().unwrap_or(false) {
            (rank, self.lines[c.line].char_count().saturating_sub(c.ch))
        } else {
            (rank, c.ch)
        }
    }
}

fn union(a: Option<Rect>, b: Option<Rect>) -> Option<Rect> {
    match (a, b) {
        (Some(a), Some(b)) => layout::union_rect(Some(a), b),
        (some, None) | (None, some) => some,
    }
}

/// Horizontal and vertical gap from a point to a rectangle, 0 on an axis the
/// point already falls inside.
fn gaps(x: f32, y: f32, r: &Rect) -> (f32, f32) {
    (
        (r.left() - x).max(x - r.right()).max(0.0),
        (r.top() - y).max(y - r.bottom()).max(0.0),
    )
}

/// Index of the character boundary in `char_x` closest to `x`.
fn nearest_boundary(char_x: &[f32], x: f32) -> usize {
    let mut best = 0;
    let mut best_d = f32::MAX;
    for (k, &bx) in char_x.iter().enumerate() {
        let d = (x - bx).abs();
        if d < best_d {
            best_d = d;
            best = k;
        }
    }
    best
}
