//! 【选区尺寸】【比例列表】六种固定比例独占输入，右键不选择，左键释放与确认键遵循原版行为

use crate::{features::selection_size::ratio, keys::Key, types::MouseButton};
use tiny_skia::{Pixmap, Rect};

pub struct Popup {
    pub id: u64,
    pub monitor: usize,
    pub rect: Rect,
    pub current: Option<usize>,
    pub active: usize,
    pub native: bool,
    pub cache: Option<Pixmap>,
    pub dirty: bool,
}

pub enum Event {
    None,
    Close,
    Choose(f64),
}

impl Popup {
    /// 【选区尺寸】【列表构造】按 id、monitor、rect 和 current_ratio 返回初始列表，自定义比例从自由行开始导航
    pub fn new(id: u64, monitor: usize, rect: Rect, current_ratio: f64) -> Self {
        let current = ratio::preset(current_ratio);
        Self {
            id,
            monitor,
            rect,
            current,
            active: current.unwrap_or(0),
            native: false,
            cache: None,
            dirty: true,
        }
    }

    /// 【选区尺寸】【列表行】按 index 返回原版四像素内边距与二十六像素行高的本地矩形
    pub fn row(&self, index: usize) -> Rect {
        Rect::from_xywh(4.0, 4.0 + index as f32 * 26.0, 120.0, 26.0).unwrap()
    }

    /// 【选区尺寸】【行命中】按 point 返回本地语言行下标，内边距返回空值
    pub fn hit(&self, point: (f64, f64)) -> Option<usize> {
        (0..ratio::PRESETS.len()).find(|index| super::state::contains(self.row(*index), point))
    }

    /// 【选区尺寸】【列表指针】按 point 更新有效行悬停，返回是否变化，离开行时保留当前键盘行
    pub fn pointer(&mut self, point: (f64, f64)) -> bool {
        if let Some(index) = self.hit(point)
            && index != self.active
        {
            self.active = index;
            self.dirty = true;
            return true;
        }
        false
    }

    /// 【选区尺寸】【列表按钮】按 point、button 和 pressed 处理原版释放选择，右键与按下阶段返回无动作
    pub fn button(&self, point: (f64, f64), button: MouseButton, pressed: bool) -> Event {
        if pressed || !matches!(button, MouseButton::Left) {
            return Event::None;
        }
        self.hit(point)
            .map_or(Event::None, |index| Event::Choose(ratio::PRESETS[index].1))
    }

    /// 【选区尺寸】【列表按键】按 key 和 shift 返回导航或选择，Shift+Tab 对应原版 Backtab，不移动行
    pub fn key(&mut self, key: Key, shift: bool) -> Event {
        let count = ratio::PRESETS.len();
        match key {
            Key::Up => self.active = (self.active + count - 1) % count,
            Key::Down => self.active = (self.active + 1) % count,
            Key::Tab if !shift => self.active = (self.active + 1) % count,
            Key::Enter | Key::Space => return Event::Choose(ratio::PRESETS[self.active].1),
            Key::Escape => return Event::Close,
            _ => return Event::None,
        }
        self.dirty = true;
        Event::None
    }
}
