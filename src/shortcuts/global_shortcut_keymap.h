#pragma once

#include <QKeySequence>
#include <QList>

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

/**
 * 返回抓取某主键时需要尝试的 X11 keysym 候选（如 Print / Sys_Req）。
 *
 * @param keysym 主 keysym。
 * @return 候选列表，至少包含自身。
 */
QList<std::uint32_t> x11KeysymCandidates(std::uint32_t keysym);

/**
 * 将 Qt 快捷键序列转换为 GNOME/GTK accelerator 字符串。
 *
 * GNOME 的 gsettings 自定义快捷键（media-keys custom-keybinding 的 binding
 * 键）使用 gtk_accelerator_parse 的格式，例如 "<Control><Alt>s"、
 * "<Super>Print"。仅取序列中的第一个组合；无修饰键的普通按键会抢占输入，
 * 但 Print 屏幕截图键例外（与 X11 后端一致）。
 *
 * @param sequence Qt 快捷键序列。
 * @return accelerator 字符串；无法转换时返回空字符串。
 */
QString gnomeAcceleratorForSequence(const QKeySequence &sequence);

}  // namespace markshot::shortcuts
