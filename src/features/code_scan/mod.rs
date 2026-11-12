//! 【扫码】【业务边界】按需识别本地图像，只显示与复制结果，不自动打开内容

pub mod decoder;
pub mod transport;
mod window;

#[cfg(test)]
mod tests;

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Code {
    pub format: String,
    pub text: String,
}

/// 【扫码】【结果文本】按 codes 生成保留码制和原始内容的结果正文，返回可选择的多行文本
pub fn text(codes: &[Code]) -> String {
    codes
        .iter()
        .enumerate()
        .map(|(index, code)| format!("{}. {}\n{}", index + 1, code.format, code.text))
        .collect::<Vec<_>>()
        .join("\n\n")
}
