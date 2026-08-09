#pragma once

#include <QKeySequence>

#include <cstdint>

namespace markshot::shortcuts {

/// @brief 一条快捷键拆解出的 X11 keysym 与修饰键掩码。
struct X11KeyBinding {
    /// @brief 主键对应的 X11 keysym 值。
    std::uint32_t keysym = 0;
    /// @brief X11 修饰键掩码，由 ShiftMask/ControlMask/Mod1Mask/Mod4Mask 组合而成。
    std::uint16_t modifiers = 0;
    /// @brief 是否解析成功。
    bool valid = false;
};

/**
 * 将 Qt 键值转换为 X11 keysym。
 *
 * @param key Qt::Key 键值。
 * @return 对应的 X11 keysym；无法映射时返回 0。
 */
std::uint32_t x11KeysymForQtKey(int key);

/**
 * 将 Qt 快捷键序列拆解为 X11 keysym 与修饰键掩码。
 *
 * 仅取序列中的第一个组合，X11 的 XGrabKey 无法表达多段快捷键。
 *
 * @param sequence Qt 快捷键序列。
 * @return 拆解结果；无法转换时 valid 为 false。
 */
X11KeyBinding x11BindingForSequence(const QKeySequence &sequence);

}  // namespace markshot::shortcuts
