//! 【界面字体】【可变字重】把排版字重映射到 FreeType wght 轴，保持度量与实际字形一致

use freetype::{Face, Library, ffi};

/// 【界面字体】【字重同步】按 library、face 与 weight 设置可变字体字重，静态字体保持原样，无返回值
pub(super) fn set_weight<B>(library: &Library, face: &Face<B>, weight: u16) {
    let raw = face.raw() as *const _ as ffi::FT_Face;
    let mut axes = std::ptr::null_mut();
    // 1. 【界面字体】【变体读取】FreeType 分配轴描述，失败表示静态字体或不支持的变体
    unsafe {
        if ffi::FT_Get_MM_Var(raw, &mut axes) != 0 || axes.is_null() {
            return;
        }
        let record = &*axes;
        let mut coordinates = vec![0; record.num_axis as usize];
        if ffi::FT_Get_Var_Design_Coordinates(raw, record.num_axis, coordinates.as_mut_ptr()) == 0 {
            let descriptions = std::slice::from_raw_parts(record.axis, record.num_axis as usize);
            for (coordinate, axis) in coordinates.iter_mut().zip(descriptions) {
                if axis.tag == u32::from_be_bytes(*b"wght") as ffi::FT_ULong {
                    *coordinate = (i64::from(weight) * 65536).clamp(axis.minimum, axis.maximum);
                }
            }
            ffi::FT_Set_Var_Design_Coordinates(raw, record.num_axis, coordinates.as_mut_ptr());
        }
        // 2. 【界面字体】【描述释放】坐标已经复制到字体对象，立即释放临时轴描述
        ffi::FT_Done_MM_Var(library.raw(), axes);
    }
}
