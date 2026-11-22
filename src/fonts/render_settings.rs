//! 【界面字体】【渲染配置】按实际字体与物理字号读取 Fontconfig 配置，配置对象只属于当前绘制线程

use std::{
    ffi::{CStr, CString, c_char, c_int, c_void},
    path::Path,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Settings {
    pub antialias: bool,
    pub hinting: bool,
    pub hintstyle: i32,
    pub autohint: bool,
    pub rgba: i32,
    pub lcd_filter: i32,
    pub embedded_bitmap: bool,
}

impl Default for Settings {
    /// 【界面字体】【默认配置】无参数，返回抗锯齿与轻微微调的灰度配置，Fontconfig 读取失败时使用
    fn default() -> Self {
        Self {
            antialias: true,
            hinting: true,
            hintstyle: 1,
            autohint: false,
            rgba: 0,
            lcd_filter: 1,
            embedded_bitmap: true,
        }
    }
}

#[link(name = "fontconfig")]
unsafe extern "C" {
    fn FcInitLoadConfigAndFonts() -> *mut c_void;
    fn FcConfigDestroy(config: *mut c_void);
    fn FcPatternCreate() -> *mut c_void;
    fn FcPatternDestroy(pattern: *mut c_void);
    fn FcPatternAddString(pattern: *mut c_void, object: *const c_char, value: *const u8) -> c_int;
    fn FcPatternAddInteger(pattern: *mut c_void, object: *const c_char, value: c_int) -> c_int;
    fn FcPatternAddDouble(pattern: *mut c_void, object: *const c_char, value: f64) -> c_int;
    fn FcConfigSubstitute(config: *mut c_void, pattern: *mut c_void, kind: c_int) -> c_int;
    fn FcDefaultSubstitute(pattern: *mut c_void);
    fn FcFontMatch(config: *mut c_void, pattern: *mut c_void, result: *mut c_int) -> *mut c_void;
    fn FcPatternGetBool(
        pattern: *mut c_void,
        object: *const c_char,
        index: c_int,
        value: *mut c_int,
    ) -> c_int;
    fn FcPatternGetInteger(
        pattern: *mut c_void,
        object: *const c_char,
        index: c_int,
        value: *mut c_int,
    ) -> c_int;
    #[cfg(test)]
    fn FcConfigCreate() -> *mut c_void;
    #[cfg(test)]
    fn FcConfigParseAndLoad(config: *mut c_void, file: *const u8, complain: c_int) -> c_int;
    #[cfg(test)]
    fn FcConfigBuildFonts(config: *mut c_void) -> c_int;
}

pub(super) struct Config(*mut c_void);
struct Pattern(*mut c_void);

impl Drop for Config {
    /// 【界面字体】【配置释放】无参数，释放当前线程持有的配置与元数据，无返回值
    fn drop(&mut self) {
        unsafe { FcConfigDestroy(self.0) }
    }
}
impl Drop for Pattern {
    /// 【界面字体】【模式释放】无参数，释放本次匹配模式，不保留字体文件映射，无返回值
    fn drop(&mut self) {
        unsafe { FcPatternDestroy(self.0) }
    }
}

impl Config {
    /// 【界面字体】【配置创建】无参数，读取系统及用户 Fontconfig 配置，返回对象，初始化失败返回空值
    pub(super) fn new() -> Option<Self> {
        let config = unsafe { FcInitLoadConfigAndFonts() };
        (!config.is_null()).then_some(Self(config))
    }

    /// 【界面字体】【实际匹配】按 file、index、family 与 pixels 返回当前字体的渲染设置，匹配失败返回默认配置
    pub(super) fn settings(&self, file: &Path, index: u32, family: &str, pixels: f32) -> Settings {
        let Some(pattern) = request(file, index, family, pixels) else {
            return Settings::default();
        };
        let mut status = 0;
        // 1. 【界面字体】【规则应用】先执行请求规则，再匹配实际字体，字体规则覆盖默认渲染属性
        let matched = unsafe {
            FcConfigSubstitute(self.0, pattern.0, 0);
            FcDefaultSubstitute(pattern.0);
            FcFontMatch(self.0, pattern.0, &mut status)
        };
        if matched.is_null() {
            return Settings::default();
        }
        let matched = Pattern(matched);
        Settings {
            antialias: boolean(matched.0, c"antialias", true),
            hinting: boolean(matched.0, c"hinting", true),
            hintstyle: integer(matched.0, c"hintstyle", 1),
            autohint: boolean(matched.0, c"autohint", false),
            rgba: integer(matched.0, c"rgba", 0),
            lcd_filter: integer(matched.0, c"lcdfilter", 1),
            embedded_bitmap: boolean(matched.0, c"embeddedbitmap", true),
        }
    }

    /// 【界面字体测试】【隔离配置】按 path 创建独立配置供测试，不修改进程环境或用户配置，返回对象或空值
    #[cfg(test)]
    pub(super) fn from_file(path: &Path) -> Option<Self> {
        let file = CString::new(path.as_os_str().as_encoded_bytes()).ok()?;
        let config = unsafe { FcConfigCreate() };
        if config.is_null() {
            return None;
        }
        let config = Self(config);
        unsafe {
            if FcConfigParseAndLoad(config.0, file.as_ptr().cast(), 1) == 0
                || FcConfigBuildFonts(config.0) == 0
            {
                return None;
            }
        }
        Some(config)
    }
}

/// 【界面字体】【匹配请求】按 file、index、family 与 pixels 创建独立模式，返回请求，非法路径或分配失败返回空值
fn request(file: &Path, index: u32, family: &str, pixels: f32) -> Option<Pattern> {
    let file = CString::new(file.as_os_str().as_encoded_bytes()).ok()?;
    let family = CString::new(family).ok()?;
    let pattern = unsafe { FcPatternCreate() };
    if pattern.is_null() {
        return None;
    }
    let pattern = Pattern(pattern);
    unsafe {
        FcPatternAddString(pattern.0, c"file".as_ptr(), file.as_ptr().cast());
        FcPatternAddString(pattern.0, c"family".as_ptr(), family.as_ptr().cast());
        FcPatternAddInteger(pattern.0, c"index".as_ptr(), index as i32);
        FcPatternAddDouble(pattern.0, c"pixelsize".as_ptr(), f64::from(pixels));
    }
    Some(pattern)
}

/// 【界面字体】【布尔属性】按 pattern、name 返回布尔属性，属性缺失时返回 fallback，不保留外部指针
fn boolean(pattern: *mut c_void, name: &CStr, fallback: bool) -> bool {
    let mut value = 0;
    if unsafe { FcPatternGetBool(pattern, name.as_ptr(), 0, &mut value) } == 0 {
        value != 0
    } else {
        fallback
    }
}

/// 【界面字体】【整数属性】按 pattern、name 返回整数属性，属性缺失时返回 fallback，不保留外部指针
fn integer(pattern: *mut c_void, name: &CStr, fallback: i32) -> i32 {
    let mut value = 0;
    if unsafe { FcPatternGetInteger(pattern, name.as_ptr(), 0, &mut value) } == 0 {
        value
    } else {
        fallback
    }
}
