//! 【标注样式】【持久数值】原图笔宽随标注保存，不依赖当前窗口、显示器或导航状态

use super::Annotation;
use crate::features::annotation_style::units::Units;

impl Annotation {
    /// 【标注样式】【自身单位】无参数，返回当前坐标相对原图的换算规则，异常比例使用一比一单位
    pub fn style_units(&self) -> Units {
        Units::for_image(1.0 / f64::from(self.style_scale))
    }

    /// 【标注样式】【原图笔宽】无参数，返回属性控件与默认值使用的原图笔宽，不加入持续导航倍率
    pub fn image_stroke_width(&self) -> f32 {
        self.style_units().image(self.stroke_width)
    }

    /// 【标注样式】【原图笔宽写入】按width更新当前坐标中的笔宽，无返回值；范围限制和历史由调用方处理
    pub fn set_image_stroke_width(&mut self, width: f32) {
        self.stroke_width = self.style_units().canvas(width);
    }
}
