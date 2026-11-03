// Detection boxes -> readable structure, in two passes:
//
//   row_groups   boxes sharing a baseline belong to one line
//   group_blocks nearby lines become a block, blocks get a reading order
//
// Thresholds are in units of the median line height so they follow the text
// size, not the resolution.

use tiny_skia::Rect;

use crate::config::OcrLayout;

use super::OcrLine;

fn cfg() -> &'static OcrLayout {
    &crate::config::get().ocr.layout
}

/// A ensemble of lines that belong together: a paragraph, a column, a menu. Lines
/// inside a block are in reading order, and so are the blocks.
#[derive(Debug)]
pub(super) struct Block {
    /// Indices into `OcrView::lines`, in reading order.
    pub(super) lines: Vec<usize>,
    pub(super) bounds: Rect,
}

// Groups text boxes on the same line into a single crop area.
//
// The OCR detector splits lines at wide spaces (tabs, columns). Merging them back
// together prevents split selections and avoids making multiple slow OCR calls.
//
// How it works:
// 1. Groups boxes into rows by their vertical center.
// 2. Splits a row only if there is a huge gap (a real second column) or `divided` sees a rule in it.
// 3. Returns the grouped box indices, ordered left to right.
pub(super) fn row_groups(
    rects: &[Rect],
    divided: impl Fn(&Rect, &Rect) -> bool,
) -> Vec<Vec<usize>> {
    let n = rects.len();
    if n == 0 {
        return Vec::new();
    }
    let cfg = cfg();
    let unit = median_of(rects.iter().map(|r| r.height()));
    let centre_y = |r: &Rect| r.top() + r.height() / 2.0;

    // The row keeps the centre of its first box, so slowly drifting boxes
    // can't chain into one oversized row.
    let mut by_y: Vec<usize> = (0..n).collect();
    by_y.sort_by(|&a, &b| centre_y(&rects[a]).total_cmp(&centre_y(&rects[b])));
    let mut rows: Vec<Vec<usize>> = Vec::new();
    let mut row_centre: Vec<f32> = Vec::new();
    for &i in &by_y {
        let cy = centre_y(&rects[i]);
        if let Some(&c) = row_centre.last()
            && (cy - c).abs() < cfg.row_tolerance * unit
        {
            rows.last_mut().unwrap().push(i);
        } else {
            rows.push(vec![i]);
            row_centre.push(cy);
        }
    }

    let mut groups: Vec<Vec<usize>> = Vec::with_capacity(rows.len());
    for mut row in rows {
        row.sort_by(|&a, &b| rects[a].left().total_cmp(&rects[b].left()));
        let mut current: Vec<usize> = Vec::new();
        let mut reach_box = 0;
        for i in row {
            if !current.is_empty()
                && (rects[i].left() - rects[reach_box].right() >= cfg.row_split_gap * unit
                    || divided(&rects[reach_box], &rects[i]))
            {
                groups.push(std::mem::take(&mut current));
            }
            if current.is_empty() || rects[i].right() > rects[reach_box].right() {
                reach_box = i;
            }
            current.push(i);
        }
        if !current.is_empty() {
            groups.push(current);
        }
    }
    groups
}

/// groupped lines into blocks and order both the lines within a block and the
/// blocks themselves. Worst case every line is its own block.
pub(super) fn group_blocks(lines: &[OcrLine]) -> Vec<Block> {
    let n = lines.len();
    if n == 0 {
        return Vec::new();
    }
    let cfg = cfg();
    let unit = median_height(lines);

    // Top-to-bottom so the merge scan below can stop early.
    let mut sorted: Vec<usize> = (0..n).collect();
    sorted.sort_by(|&a, &b| cmp_reading(&lines[a].bounds, &lines[b].bounds));

    let mut uf = UnionFind::new(n);
    for (si, &i) in sorted.iter().enumerate() {
        let a = &lines[i].bounds;
        for &j in &sorted[si + 1..] {
            let b = &lines[j].bounds;
            let vgap = b.top() - a.bottom();
            if vgap > cfg.block_gap_max * unit {
                break; // sorted by top, so everything later is further still
            }
            if vgap > cfg.block_gap * unit {
                continue;
            }
            let overlap = a.right().min(b.right()) - a.left().max(b.left());
            let min_w = a.width().min(b.width()).max(1.0);
            let left_aligned = (a.left() - b.left()).abs() < cfg.block_align * unit;
            if overlap > 0.0 && (overlap / min_w > cfg.block_overlap || left_aligned) {
                uf.union(i, j);
            }
        }
    }

    // Gather line indices per block
    let mut group_of: Vec<Option<usize>> = vec![None; n];
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for i in 0..n {
        let root = uf.find(i);
        let g = *group_of[root].get_or_insert_with(|| {
            groups.push(Vec::new());
            groups.len() - 1
        });
        groups[g].push(i);
    }

    let blocks: Vec<Block> = groups
        .into_iter()
        .map(|mut idxs| {
            idxs.sort_by(|&a, &b| cmp_reading(&lines[a].bounds, &lines[b].bounds));
            let bounds = union_bounds(&idxs, lines);
            Block {
                lines: idxs,
                bounds,
            }
        })
        .collect();

    order_blocks(merge_touching(blocks, lines), unit)
}

fn merge_touching(blocks: Vec<Block>, lines: &[OcrLine]) -> Vec<Block> {
    let n = blocks.len();
    if n < 2 {
        return blocks;
    }

    let mut uf = UnionFind::new(n);
    let mut merged = false;
    for (i, first) in blocks.iter().enumerate() {
        let a = &first.bounds;
        for (j, second) in blocks.iter().enumerate().skip(i + 1) {
            let b = &second.bounds;
            let apart = a.right() <= b.left()
                || b.right() <= a.left()
                || a.bottom() <= b.top()
                || b.bottom() <= a.top();
            if !apart {
                uf.union(i, j);
                merged = true;
            }
        }
    }
    if !merged {
        return blocks;
    }

    let mut slot_of: Vec<Option<usize>> = vec![None; n];
    let mut out: Vec<Block> = Vec::new();
    for (i, block) in blocks.into_iter().enumerate() {
        let root = uf.find(i);
        match slot_of[root] {
            Some(s) => {
                let target: &mut Block = &mut out[s];
                target.lines.extend(block.lines);
                target.bounds =
                    union_rect(Some(target.bounds), block.bounds).unwrap_or(target.bounds);
            }
            None => {
                slot_of[root] = Some(out.len());
                out.push(block);
            }
        }
    }
    for block in &mut out {
        block
            .lines
            .sort_by(|&a, &b| cmp_reading(&lines[a].bounds, &lines[b].bounds));
    }

    merge_touching(out, lines)
}

// Sorts text blocks column by column (left to right, top to bottom).
// This prevents multi-column text from getting mixed up when copied.
fn order_blocks(blocks: Vec<Block>, unit: f32) -> Vec<Block> {
    let n = blocks.len();
    if n < 2 {
        return blocks;
    }

    let mut by_left: Vec<usize> = (0..n).collect();
    by_left.sort_by(|&a, &b| blocks[a].bounds.left().total_cmp(&blocks[b].bounds.left()));

    let gutter = cfg().column_gutter * unit;
    let mut column = vec![0usize; n];
    for w in 1..n {
        let prev = &blocks[by_left[w - 1]].bounds;
        let cur = &blocks[by_left[w]].bounds;
        let split = cur.left() - prev.left() > gutter && cur.left() > prev.right();
        column[by_left[w]] = column[by_left[w - 1]] + usize::from(split);
    }

    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| {
        column[a]
            .cmp(&column[b])
            .then(cmp_reading(&blocks[a].bounds, &blocks[b].bounds))
    });

    let mut slots: Vec<Option<Block>> = blocks.into_iter().map(Some).collect();
    order
        .into_iter()
        .map(|src| slots[src].take().expect("each block moved exactly once"))
        .collect()
}

/// Median line height, the unit for every threshold in this module.
pub(super) fn median_height(lines: &[OcrLine]) -> f32 {
    median_of(lines.iter().map(|l| l.bounds.height()))
}

fn median_of(values: impl Iterator<Item = f32>) -> f32 {
    let mut v: Vec<f32> = values.collect();
    if v.is_empty() {
        return 1.0;
    }
    v.sort_by(f32::total_cmp);
    v[v.len() / 2].max(1.0)
}

pub(super) fn cmp_reading(a: &Rect, b: &Rect) -> std::cmp::Ordering {
    a.top()
        .total_cmp(&b.top())
        .then(a.left().total_cmp(&b.left()))
}

/// Smallest rectangle containing both, or `a` when that somehow fails.
pub(super) fn union_rect(a: Option<Rect>, b: Rect) -> Option<Rect> {
    let Some(a) = a else { return Some(b) };
    Rect::from_ltrb(
        a.left().min(b.left()),
        a.top().min(b.top()),
        a.right().max(b.right()),
        a.bottom().max(b.bottom()),
    )
    .or(Some(a))
}

fn union_bounds(idxs: &[usize], lines: &[OcrLine]) -> Rect {
    let (mut l, mut t, mut r, mut b) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for &i in idxs {
        let bb = &lines[i].bounds;
        l = l.min(bb.left());
        t = t.min(bb.top());
        r = r.max(bb.right());
        b = b.max(bb.bottom());
    }
    Rect::from_ltrb(l, t, r, b).unwrap_or_else(|| lines[idxs[0]].bounds)
}

/// Disjoint-set forest, for groupping lines into blocks
struct UnionFind {
    parent: Vec<usize>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
        }
    }

    fn find(&mut self, x: usize) -> usize {
        let mut root = x;
        while self.parent[root] != root {
            root = self.parent[root];
        }
        let mut node = x;
        while self.parent[node] != root {
            let next = self.parent[node];
            self.parent[node] = root;
            node = next;
        }
        root
    }

    fn union(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            self.parent[ra] = rb;
        }
    }
}
