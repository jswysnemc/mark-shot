//! 【识别模型】【首次引导】复用模型选择组件，明确完成后返回模型文件，取消不继续原动作

mod window;
use crate::ocr::{
    engine_choice::{self, Source},
    models::OcrModels,
};
pub(crate) use window::dialog;

/// 【识别模型】【引导入口】在无已安装模型时打开原生下载引导，返回用户选择且已安装的模型，取消返回空值
pub fn run() -> crate::AppResult<Option<Source>> {
    let models = OcrModels::load();
    if let Some(files) = engine_choice::source(&models) {
        return Ok(Some(files));
    }
    window::run(models)
}
