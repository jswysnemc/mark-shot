//! 【识别安装】【资源控件】复用面板绘制、命中和状态，选择引擎不创建下载任务

use super::{ModelPopover, ModelPopoverElement, cfg};
use crate::ocr::{
    engine_choice::{self, Engine},
    models::{ModelStatus, OcrModels},
};
use tiny_skia::Rect;

pub use super::setup_draw::draw;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State {
    pub engine: Engine,
    pub runtime: ModelStatus,
    pub message: String,
}

impl Default for State {
    /// 【识别安装】【初始状态】无参数，返回未安装显示，不查询系统或启动网络任务
    fn default() -> Self {
        Self {
            engine: Engine::default(),
            runtime: ModelStatus::Missing,
            message: String::new(),
        }
    }
}

impl State {
    /// 【识别安装】【模型显示】无参数，返回当前选择是否显示第三方资源列表
    pub fn show_models(&self) -> bool {
        !cfg!(target_os = "windows") || self.engine == Engine::Paddle
    }
}

/// 【识别安装】【界面字体】为 fonts 加载 panel 中实际使用的中文与错误字形，无返回值，不枚举完整系统字库
pub fn prepare_text(fonts: &mut cosmic_text::FontSystem, panel: &ModelPopover) {
    if crate::ui::language::Language::current() == crate::ui::language::Language::Chinese {
        crate::fonts::ui_fallback::ensure_text(
            fonts,
            "引擎与资源自行选择下载不会自动推理库使用系统语言包无需本地模型手动一次安装后可离线识别已其他语言展开收起就绪需安装模型中文日文英文拉丁西里尔阿拉伯希腊韩语泰语推荐适合简单单语言文本",
        );
    }
    crate::fonts::ui_fallback::ensure_text(fonts, &panel.setup.message);
}

/// 【识别安装】【控件数量】无参数，返回模型列表之前的平台控件行数
pub fn rows() -> usize {
    if cfg!(target_os = "windows") { 3 } else { 1 }
}

/// 【识别安装】【行矩形】按 origin 与 index 返回统一设置行矩形，供绘制和命中共用
pub(super) fn rect(panel: &ModelPopover, origin: (f32, f32), index: usize) -> Rect {
    let config = cfg();
    Rect::from_xywh(
        origin.0 + config.padding,
        origin.1
            + config.padding
            + config.title_height
            + 32.0
            + (index + usize::from(panel.setup.show_models())) as f32 * config.row_height,
        config.width - config.padding * 2.0,
        config.row_height,
    )
    .unwrap()
}

/// 【识别安装】【输入命中】按 origin 和 point 返回引擎或推理库控件，范围外返回空值
pub fn hit(
    panel: &ModelPopover,
    origin: (f32, f32),
    point: (f64, f64),
) -> Option<ModelPopoverElement> {
    (0..super::layout::setup_rows(panel))
        .find(|index| super::state::point_in_rect(point, rect(panel, origin, *index)))
        .map(|index| match (cfg!(target_os = "windows"), index) {
            (true, 0) => ModelPopoverElement::SystemEngine,
            (true, 1) => ModelPopoverElement::PaddleEngine,
            _ => ModelPopoverElement::Runtime,
        })
}

/// 【识别安装】【状态同步】从 models 更新 panel 的资源状态与有效尺寸，返回显示是否改变
pub fn sync(panel: &mut ModelPopover, models: &OcrModels) -> bool {
    let old = panel.setup.clone();
    panel.setup.engine = engine_choice::current();
    panel.setup.runtime = models.library.status;
    if old.runtime != panel.setup.runtime {
        panel.setup.message.clear();
    }
    if panel.setup.show_models()
        && let Some(error) = &models.library.error
    {
        panel.setup.message = error.clone();
    }
    let old_height = panel.size.1;
    panel.size.1 = super::layout::height(panel);
    old != panel.setup || old_height != panel.size.1
}

/// 【识别安装】【显式操作】对 element 执行选择或下载，models 持有后台任务，返回是否处理；错误保存在 panel
pub fn activate(
    panel: &mut ModelPopover,
    models: &mut OcrModels,
    element: ModelPopoverElement,
) -> bool {
    let result = match element {
        ModelPopoverElement::Resources => {
            panel.resources_expanded = !panel.resources_expanded;
            Ok(())
        }
        ModelPopoverElement::Languages => {
            panel.languages_expanded = !panel.languages_expanded;
            Ok(())
        }
        ModelPopoverElement::SystemEngine => {
            #[cfg(target_os = "windows")]
            {
                crate::ocr::windows_backend::languages()
                    .and_then(|_| engine_choice::select(Engine::System))
            }
            #[cfg(not(target_os = "windows"))]
            {
                Err("Windows OCR unavailable".into())
            }
        }
        ModelPopoverElement::PaddleEngine => engine_choice::select(Engine::Paddle),
        ModelPopoverElement::Runtime if panel.setup.show_models() => {
            if models.library.needs_poll() {
                models.library.cancel();
            } else if models.library.status != ModelStatus::Installed {
                models.library.start();
            }
            Ok(())
        }
        _ => return false,
    };
    panel.setup.message = result.err().unwrap_or_default();
    sync(panel, models);
    panel.dirty = true;
    true
}
