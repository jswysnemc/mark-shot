//! 【选区尺寸】【面板状态】实际尺寸、待提交字段与固定位置分开保存，隐藏时结束编辑，不保存工具历史

use super::{field::NumberField, layout::Layout};
use crate::{features::selection_size::ratio::Field, ui::language::Language};
use cosmic_text::{FontSystem, SwashCache};
use tiny_skia::{Pixmap, Rect};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    Field(Field),
    Lock,
    Ratio,
    Background,
}

pub struct Panel {
    pub rect: Option<Rect>,
    pub monitor: usize,
    pub layout: Layout,
    pub fields: [NumberField; 2],
    pub values: (i32, i32),
    pub maximum: (i32, i32),
    pub ratio: f64,
    pub hovered: Option<Hit>,
    pub language: Language,
    pub fonts: FontSystem,
    pub swash: SwashCache,
    pub cache: Option<Pixmap>,
    pub dirty: bool,
    pub under_toolbars: bool,
}

impl Panel {
    /// 【选区尺寸】【面板构造】按 language 返回按需字体和固定布局，初始隐藏，不加载识别模型
    pub fn new(language: Language) -> Self {
        let mut fonts = crate::fonts::selection_ui::new();
        let layout = Layout::new(&mut fonts, language);
        Self {
            rect: None,
            monitor: 0,
            layout,
            fields: [NumberField::new(1, 1), NumberField::new(1, 1)],
            values: (1, 1),
            maximum: (1, 1),
            ratio: 0.0,
            hovered: None,
            language,
            fonts,
            swash: SwashCache::new(),
            cache: None,
            dirty: true,
            under_toolbars: false,
        }
    }

    /// 【选区尺寸】【字段借用】按 field 返回宽度或高度字段，不改变编辑状态
    pub fn field(&self, field: Field) -> &NumberField {
        &self.fields[usize::from(field == Field::Height)]
    }

    /// 【选区尺寸】【可变字段】按 field 返回宽度或高度字段，供明确输入动作使用
    pub fn field_mut(&mut self, field: Field) -> &mut NumberField {
        &mut self.fields[usize::from(field == Field::Height)]
    }

    /// 【选区尺寸】【修改请求】按 field 和 value 返回联动后的目标宽高，不提前覆盖实际选区尺寸
    pub fn requested(&self, field: Field, value: i32) -> (i32, i32) {
        let requested = match field {
            Field::Width => (value, self.values.1),
            Field::Height => (self.values.0, value),
        };
        crate::features::selection_size::ratio::linked(requested, field, self.ratio, self.maximum)
    }

    /// 【选区尺寸】【当前字段】返回正在编辑的一侧，没有活动输入时返回空值，无参数
    pub fn editing(&self) -> Option<Field> {
        if self.fields[0].editing {
            Some(Field::Width)
        } else if self.fields[1].editing {
            Some(Field::Height)
        } else {
            None
        }
    }

    /// 【选区尺寸】【显示同步】按 values、maximum、ratio 和 language 更新显示，保留用户字段文字，返回是否变化
    pub fn sync(
        &mut self,
        values: (i32, i32),
        maximum: (i32, i32),
        ratio: f64,
        language: Language,
    ) -> bool {
        let values = (values.0.max(1), values.1.max(1));
        let maximum = (maximum.0.max(1), maximum.1.max(1));
        let ratio = if ratio.is_finite() {
            ratio.max(0.0)
        } else {
            0.0
        };
        let changed = self.values != values
            || self.maximum != maximum
            || self.ratio != ratio
            || self.language != language;
        self.values = values;
        self.maximum = maximum;
        self.ratio = ratio;
        self.fields[0].sync(values.0, maximum.0);
        self.fields[1].sync(values.1, maximum.1);
        if self.language != language {
            self.layout = Layout::new(&mut self.fonts, language);
            self.language = language;
        }
        self.dirty |= changed;
        changed
    }

    /// 【选区尺寸】【隐藏输入】结束两个字段并清除悬停，实际比例和尺寸保留在会话中，无参数、无返回值
    pub fn hide(&mut self) {
        self.rect = None;
        for field in &mut self.fields {
            field.cancel();
        }
        self.hovered = None;
        self.dirty = true;
    }

    /// 【选区尺寸】【面板命中】按 point 的显示器本地坐标返回字段或按钮，面板外返回空值
    pub fn hit(&self, point: (f64, f64)) -> Option<Hit> {
        let bounds = self.rect?;
        if !contains(bounds, point) {
            return None;
        }
        let local = (
            point.0 - f64::from(bounds.left()),
            point.1 - f64::from(bounds.top()),
        );
        for field in [Field::Width, Field::Height] {
            if contains(self.layout.field(field), local) {
                return Some(Hit::Field(field));
            }
        }
        if contains(self.layout.lock, local) {
            Some(Hit::Lock)
        } else if contains(self.layout.ratio, local) {
            Some(Hit::Ratio)
        } else {
            Some(Hit::Background)
        }
    }

    /// 【选区尺寸】【全局字段范围】按 field 返回显示器本地字段矩形，面板隐藏时返回空值
    pub fn field_rect(&self, field: Field) -> Option<Rect> {
        let bounds = self.rect?;
        let local = self.layout.field(field);
        Rect::from_xywh(
            bounds.left() + local.left(),
            bounds.top() + local.top(),
            local.width(),
            local.height(),
        )
    }
}

/// 【选区尺寸】【范围判断】按 rect 和 point 返回左闭右开的命中结果，不重复消费相邻边缘
pub fn contains(rect: Rect, point: (f64, f64)) -> bool {
    point.0 >= f64::from(rect.left())
        && point.0 < f64::from(rect.right())
        && point.1 >= f64::from(rect.top())
        && point.1 < f64::from(rect.bottom())
}
