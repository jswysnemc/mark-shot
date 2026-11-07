use crate::tools::Tool;

/// 【命令行】【工具名称】将原版工具名称或别名转换为原生工具；未知名称返回错误
pub(super) fn parse(value: &str) -> Result<Tool, String> {
    crate::tools::names::parse(value)
}
