//! 【区域识别】【候选查询】窗口内控件优先、可靠边界排序与完整包含链

use super::candidate::{Candidate, PixelRect, Source, merge};

pub struct Detector {
    width: i32,
    height: i32,
    candidates: Vec<Candidate>,
}

impl Detector {
    /// 【区域识别】【冻结图像】按 width、height、rgba 与 premultiplied 分析原始像素，返回查询器或尺寸错误，候选最多五百一十二项
    pub fn from_rgba(
        width: i32,
        height: i32,
        rgba: &[u8],
        premultiplied: bool,
    ) -> Result<Self, &'static str> {
        if width < 0 || height < 0 {
            return Err("region image dimensions must be nonnegative");
        }
        let source = super::image::Rgb::new(width as usize, height as usize, rgba, premultiplied)?;
        let mut candidates = super::analysis::detect(&source);
        candidates.sort_by(|a, b| {
            b.quality
                .total_cmp(&a.quality)
                .then_with(|| a.rect.area().cmp(&b.rect.area()))
        });
        candidates.truncate(super::analysis::MAX_CANDIDATES);
        candidates.sort_by_key(|candidate| candidate.rect.area());
        Ok(Self {
            width,
            height,
            candidates,
        })
    }

    /// 【区域识别】【候选构造】按 width、height 保存 candidates，返回查询器，保留传入候选顺序
    pub fn from_candidates(width: i32, height: i32, candidates: Vec<Candidate>) -> Self {
        Self {
            width,
            height,
            candidates,
        }
    }

    /// 【区域识别】【原图边界】无参数，返回完整图像的像素范围
    pub fn bounds(&self) -> PixelRect {
        PixelRect::new(0, 0, self.width, self.height)
    }

    /// 【区域识别】【候选访问】无参数，返回只读候选，悬停查询不重新分析图像
    pub fn candidates(&self) -> &[Candidate] {
        &self.candidates
    }

    /// 【区域识别】【窗口注入】按 windows 裁切合成器窗口并合并近似候选，无返回值，八像素以下窗口不参与
    pub fn add_windows(&mut self, windows: &[PixelRect]) {
        for window in windows {
            let Some(rect) = window.intersect(self.bounds()) else {
                continue;
            };
            if rect.width() < 8 || rect.height() < 8 {
                continue;
            }
            merge(
                &mut self.candidates,
                Candidate {
                    rect,
                    quality: 0.92,
                    sides: [0.92; 4],
                    interior_edges: 0.0,
                    sources: vec![Source::Window],
                },
                2,
            );
        }
        self.candidates
            .sort_by_key(|candidate| candidate.rect.area());
    }

    /// 【区域识别】【归一化坐标】按 x、y 映射到原图整数像素，返回图像内坐标，最右与最下位置限制在最后一个像素
    pub fn point_from_normalized(&self, x: f32, y: f32) -> (i32, i32) {
        let px = (x.clamp(0.0, 1.0) * self.width as f32) as i32;
        let py = (y.clamp(0.0, 1.0) * self.height as f32) as i32;
        (
            px.min((self.width - 1).max(0)),
            py.min((self.height - 1).max(0)),
        )
    }

    /// 【区域识别】【区域命中】按 point 选择可靠控件或窗口，返回像素矩形，没有候选时返回整图
    pub fn region_at(&self, point: (i32, i32)) -> PixelRect {
        // 1. 【区域识别】【独立排名】图像控件与合成器窗口分别排名，窗口不能直接压过内部控件
        let window = self.best(point, |candidate| candidate.is_window());
        let mut ui = self.best(point, |candidate| !candidate.is_window());
        if let (Some(window), Some(candidate)) = (window, ui)
            && !window.rect.expanded(4).contains_rect(candidate.rect)
        {
            // 2. 【区域识别】【窗口约束】越出窗口的深色连通区域不能取代窗口，重新查找其内部控件
            ui = self.best(point, |candidate| {
                !candidate.is_window() && window.rect.expanded(4).contains_rect(candidate.rect)
            });
        }
        ui.or(window)
            .map_or(self.bounds(), |candidate| candidate.rect)
    }

    /// 【区域识别】【最优候选】按 point 与 eligible 筛选候选，返回排名最小者，相同排名保持已有顺序
    fn best(&self, point: (i32, i32), eligible: impl Fn(&Candidate) -> bool) -> Option<&Candidate> {
        let mut best: Option<&Candidate> = None;
        for candidate in &self.candidates {
            if candidate.rect.contains(point)
                && eligible(candidate)
                && best.is_none_or(|best| candidate.rank() < best.rank())
            {
                best = Some(candidate);
            }
        }
        best
    }

    /// 【区域识别】【父子查询】按 point 和 current 查找扩大或缩小一级的候选，parent 指定方向，返回候选矩形或空值
    pub(super) fn adjacent(
        &self,
        point: (i32, i32),
        current: PixelRect,
        parent: bool,
    ) -> Option<PixelRect> {
        let mut best: Option<PixelRect> = None;
        for candidate in &self.candidates {
            let rect = candidate.rect;
            if rect == current
                || !rect.contains(point)
                || !(if parent {
                    rect.contains_rect(current)
                } else {
                    current.contains_rect(rect)
                })
            {
                continue;
            }
            if best.is_none_or(|best| {
                if parent {
                    rect.area() < best.area()
                } else {
                    rect.area() > best.area()
                }
            }) {
                best = Some(rect);
            }
        }
        best
    }

    /// 【区域识别】【完整层级】按 point 构造真正包含的子区域到整图链，返回矩形列表，交叉候选不进入链
    pub fn regions_at(&self, point: (i32, i32)) -> Vec<PixelRect> {
        let current = self.region_at(point);
        let mut children = Vec::new();
        let mut child = current;
        while let Some(next) = self.adjacent(point, child, false) {
            children.push(next);
            child = next;
        }
        children.reverse();
        children.push(current);
        let mut parent = current;
        while let Some(next) = self.adjacent(point, parent, true) {
            children.push(next);
            parent = next;
        }
        if children.last() != Some(&self.bounds()) {
            children.push(self.bounds());
        }
        children
    }
}
