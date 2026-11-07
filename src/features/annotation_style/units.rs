//! 【标注样式】【原图换算】属性数值使用原图像素，持久几何继续使用稳定画布坐标

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Units {
    image_per_canvas: f64,
    capture_compensation: f64,
}

impl Default for Units {
    /// 【标注样式】【默认单位】无参数，返回原图和画布一比一且没有截图比例补偿的单位
    fn default() -> Self {
        Self::for_image(1.0)
    }
}

/// 【标注样式】【有效比例】按value返回有限正比例，无效输入返回一比一比例
fn valid_scale(value: f64) -> f64 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        1.0
    }
}

impl Units {
    /// 【标注样式】【图片单位】按image_per_canvas构造文件图片单位，返回换算规则；文件入口的Qt源几何为空，不补偿截图比例
    pub fn for_image(image_per_canvas: f64) -> Self {
        Self {
            image_per_canvas: valid_scale(image_per_canvas),
            capture_compensation: 1.0,
        }
    }

    /// 【标注样式】【截图单位】按image_per_canvas构造冻结截图单位，返回换算规则；尺子使用原版零点五至四倍截图补偿
    pub fn for_capture(image_per_canvas: f64) -> Self {
        let image_per_canvas = valid_scale(image_per_canvas);
        Self {
            image_per_canvas,
            capture_compensation: image_per_canvas.clamp(0.5, 4.0),
        }
    }

    /// 【标注样式】【原图到画布】按image_value返回稳定画布中的尺寸，不加入持续导航倍率
    pub fn canvas(self, image_value: f32) -> f32 {
        image_value / self.image_per_canvas as f32
    }

    /// 【标注样式】【画布到原图】按canvas_value返回属性控件使用的原图数值，不对分数笔宽取整
    pub fn image(self, canvas_value: f32) -> f32 {
        canvas_value * self.image_per_canvas as f32
    }

    /// 【标注样式】【固定原图尺寸】无参数，返回一个原图像素在稳定画布中所占的尺寸
    pub fn canvas_per_image(self) -> f32 {
        self.canvas(1.0)
    }

    /// 【标注样式】【尺子尺寸】无参数，返回刻度、数字及标签的画布比例，保留原版截图比例补偿
    pub fn ruler_scale(self) -> f64 {
        self.capture_compensation / self.image_per_canvas
    }
}
