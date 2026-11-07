//! 【外部动作】【模块入口】桌面应用和扩展命令按需加载，共用私有图片交付与独立进程启动

pub mod desktop_apps;
pub mod desktop_exec;
pub mod extensions;
pub mod launch;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Desktop(desktop_apps::DesktopApp),
    Extension(extensions::ExtensionCommand),
}

impl Action {
    /// 【外部动作】【标题读取】返回当前应用或扩展名称，不修改用户配置，无参数
    pub fn name(&self) -> &str {
        match self {
            Self::Desktop(app) => &app.name,
            Self::Extension(command) => &command.name,
        }
    }

    /// 【外部动作】【关闭策略】返回启动成功后是否关闭编辑器；桌面应用始终关闭，扩展读取独立配置，无参数
    pub fn close_on_start(&self) -> bool {
        match self {
            Self::Desktop(_) => true,
            Self::Extension(command) => command.close_on_start,
        }
    }

    /// 【外部动作】【图像依赖】返回启动前是否需要导出图片；仅含区域参数的扩展不编码图像，无参数
    pub fn needs_image(&self) -> bool {
        match self {
            Self::Desktop(_) => true,
            Self::Extension(command) => command.save_image,
        }
    }
}
