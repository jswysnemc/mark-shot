//! 【字体目录】【系统枚举】用独立 Fontconfig 配置读取元数据，不初始化全局字体库或加载字形

use std::ffi::{CStr, c_char, c_int, c_void};

#[repr(C)]
struct FontSet {
    count: c_int,
    capacity: c_int,
    fonts: *mut *mut c_void,
}

#[link(name = "fontconfig")]
unsafe extern "C" {
    fn FcInitLoadConfigAndFonts() -> *mut c_void;
    fn FcConfigDestroy(config: *mut c_void);
    fn FcPatternCreate() -> *mut c_void;
    fn FcPatternDestroy(pattern: *mut c_void);
    fn FcPatternAddBool(pattern: *mut c_void, object: *const c_char, value: c_int) -> c_int;
    fn FcObjectSetCreate() -> *mut c_void;
    fn FcObjectSetAdd(set: *mut c_void, object: *const c_char) -> c_int;
    fn FcObjectSetDestroy(set: *mut c_void);
    fn FcFontList(config: *mut c_void, pattern: *mut c_void, objects: *mut c_void) -> *mut FontSet;
    fn FcFontSetDestroy(set: *mut FontSet);
    fn FcPatternGetString(
        pattern: *mut c_void,
        object: *const c_char,
        index: c_int,
        value: *mut *mut u8,
    ) -> c_int;
    fn FcPatternGetInteger(
        pattern: *mut c_void,
        object: *const c_char,
        index: c_int,
        value: *mut c_int,
    ) -> c_int;
}

#[derive(Clone, Debug)]
pub(super) struct Record {
    pub families: Vec<String>,
    pub languages: Vec<String>,
    pub styles: Vec<String>,
    pub foundry: String,
    pub file: String,
    pub index: u32,
}

struct Config(*mut c_void);
impl Drop for Config {
    /// 【字体目录】【配置释放】释放当前配置和字体元数据，无参数和返回值
    fn drop(&mut self) {
        unsafe {
            FcConfigDestroy(self.0);
        }
    }
}

/// 【字体目录】【字符串数组】读取 pattern 的 object 属性并返回独立字符串数组，不保留外部指针
unsafe fn strings(pattern: *mut c_void, object: &CStr) -> Vec<String> {
    let mut result = Vec::new();
    loop {
        let mut value = std::ptr::null_mut();
        let status = unsafe {
            FcPatternGetString(pattern, object.as_ptr(), result.len() as c_int, &mut value)
        };
        if status != 0 || value.is_null() {
            break;
        }
        result.push(
            unsafe { CStr::from_ptr(value.cast()) }
                .to_string_lossy()
                .into_owned(),
        );
    }
    result
}

/// 【字体目录】【记录枚举】返回系统字体元数据，配置或枚举失败返回空数组；不扫描字形内容
pub(super) fn records() -> Vec<Record> {
    // 1. 【字体目录】【配置隔离】局部配置在枚举后释放，字体弹层不会把所有模式永久留在主编辑器中
    let config = Config(unsafe { FcInitLoadConfigAndFonts() });
    if config.0.is_null() {
        return Vec::new();
    }
    unsafe {
        let pattern = FcPatternCreate();
        let objects = FcObjectSetCreate();
        if pattern.is_null() || objects.is_null() {
            if !pattern.is_null() {
                FcPatternDestroy(pattern);
            }
            if !objects.is_null() {
                FcObjectSetDestroy(objects);
            }
            return Vec::new();
        }
        FcPatternAddBool(pattern, c"variable".as_ptr(), 0);
        for object in [
            c"family",
            c"style",
            c"weight",
            c"slant",
            c"spacing",
            c"file",
            c"index",
            c"lang",
            c"charset",
            c"foundry",
            c"scalable",
            c"pixelsize",
            c"width",
            c"familylang",
            c"capability",
            c"color",
        ] {
            FcObjectSetAdd(objects, object.as_ptr());
        }
        let fonts = FcFontList(config.0, pattern, objects);
        FcPatternDestroy(pattern);
        FcObjectSetDestroy(objects);
        if fonts.is_null() {
            return Vec::new();
        }
        let mut result = Vec::with_capacity((*fonts).count.max(0) as usize);
        for row in 0..(*fonts).count.max(0) as usize {
            let pattern = *(*fonts).fonts.add(row);
            let mut index = 0;
            FcPatternGetInteger(pattern, c"index".as_ptr(), 0, &mut index);
            result.push(Record {
                families: strings(pattern, c"family"),
                languages: strings(pattern, c"familylang"),
                styles: strings(pattern, c"style"),
                foundry: strings(pattern, c"foundry")
                    .into_iter()
                    .next()
                    .unwrap_or_default(),
                file: strings(pattern, c"file")
                    .into_iter()
                    .next()
                    .unwrap_or_default(),
                index: index.max(0) as u32,
            });
        }
        FcFontSetDestroy(fonts);
        result
    }
}
