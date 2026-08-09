#include "shortcuts/global_shortcut_keymap.h"

namespace markshot::shortcuts {
namespace {

// X11 修饰键掩码，取自 X.h。这里直接定义常量而不包含 X11 头文件，
// 使该转换逻辑在没有 X11 开发包的环境下也能编译和单元测试。
constexpr std::uint16_t kShiftMask = 1 << 0;
constexpr std::uint16_t kControlMask = 1 << 2;
constexpr std::uint16_t kMod1Mask = 1 << 3;  // 通常绑定 Alt
constexpr std::uint16_t kMod4Mask = 1 << 6;  // 通常绑定 Super / Meta

// 常用功能键的 X11 keysym，取自 X11/keysymdef.h
constexpr std::uint32_t kKeysymBackSpace = 0xff08;
constexpr std::uint32_t kKeysymTab = 0xff09;
constexpr std::uint32_t kKeysymReturn = 0xff0d;
constexpr std::uint32_t kKeysymPause = 0xff13;
constexpr std::uint32_t kKeysymScrollLock = 0xff14;
constexpr std::uint32_t kKeysymEscape = 0xff1b;
constexpr std::uint32_t kKeysymDelete = 0xffff;
constexpr std::uint32_t kKeysymHome = 0xff50;
constexpr std::uint32_t kKeysymLeft = 0xff51;
constexpr std::uint32_t kKeysymUp = 0xff52;
constexpr std::uint32_t kKeysymRight = 0xff53;
constexpr std::uint32_t kKeysymDown = 0xff54;
constexpr std::uint32_t kKeysymPageUp = 0xff55;
constexpr std::uint32_t kKeysymPageDown = 0xff56;
constexpr std::uint32_t kKeysymEnd = 0xff57;
constexpr std::uint32_t kKeysymPrint = 0xff61;
constexpr std::uint32_t kKeysymInsert = 0xff63;
constexpr std::uint32_t kKeysymNumLock = 0xff7f;
constexpr std::uint32_t kKeysymF1 = 0xffbe;
constexpr std::uint32_t kKeysymCapsLock = 0xffe5;

}  // namespace

std::uint32_t x11KeysymForQtKey(int key)
{
    // 1. 字母键：X11 抓取的是未加 Shift 的小写 keysym，其值与 ASCII 小写字母一致
    if (key >= Qt::Key_A && key <= Qt::Key_Z) {
        return static_cast<std::uint32_t>('a' + (key - Qt::Key_A));
    }
    // 2. 数字键：keysym 与 ASCII 数字一致
    if (key >= Qt::Key_0 && key <= Qt::Key_9) {
        return static_cast<std::uint32_t>('0' + (key - Qt::Key_0));
    }
    // 3. 功能键：F1 起连续编号
    if (key >= Qt::Key_F1 && key <= Qt::Key_F24) {
        return kKeysymF1 + static_cast<std::uint32_t>(key - Qt::Key_F1);
    }

    // 4. 其余按键逐个映射；标点类 keysym 与 ASCII 编码一致
    switch (key) {
    case Qt::Key_Backspace: return kKeysymBackSpace;
    case Qt::Key_Tab: return kKeysymTab;
    case Qt::Key_Return:
    case Qt::Key_Enter: return kKeysymReturn;
    case Qt::Key_Escape: return kKeysymEscape;
    case Qt::Key_Space: return 0x0020;
    case Qt::Key_PageUp: return kKeysymPageUp;
    case Qt::Key_PageDown: return kKeysymPageDown;
    case Qt::Key_End: return kKeysymEnd;
    case Qt::Key_Home: return kKeysymHome;
    case Qt::Key_Left: return kKeysymLeft;
    case Qt::Key_Up: return kKeysymUp;
    case Qt::Key_Right: return kKeysymRight;
    case Qt::Key_Down: return kKeysymDown;
    case Qt::Key_Insert: return kKeysymInsert;
    case Qt::Key_Delete: return kKeysymDelete;
    case Qt::Key_Print: return kKeysymPrint;
    case Qt::Key_Pause: return kKeysymPause;
    case Qt::Key_CapsLock: return kKeysymCapsLock;
    case Qt::Key_NumLock: return kKeysymNumLock;
    case Qt::Key_ScrollLock: return kKeysymScrollLock;
    case Qt::Key_Plus: return 0x002b;
    case Qt::Key_Comma: return 0x002c;
    case Qt::Key_Minus: return 0x002d;
    case Qt::Key_Period: return 0x002e;
    case Qt::Key_Slash: return 0x002f;
    case Qt::Key_Semicolon: return 0x003b;
    case Qt::Key_BracketLeft: return 0x005b;
    case Qt::Key_Backslash: return 0x005c;
    case Qt::Key_BracketRight: return 0x005d;
    case Qt::Key_QuoteLeft: return 0x0060;
    case Qt::Key_Apostrophe: return 0x0027;
    default: return 0;
    }
}

X11KeyBinding x11BindingForSequence(const QKeySequence &sequence)
{
    X11KeyBinding binding;
    if (sequence.isEmpty()) {
        return binding;
    }

    // 1. X11 的按键抓取只能表达单个组合，多段快捷键取首段
    const QKeyCombination combination = sequence[0];
    binding.keysym = x11KeysymForQtKey(combination.key());
    if (binding.keysym == 0) {
        return binding;
    }

    // 2. 累加修饰键掩码
    const Qt::KeyboardModifiers modifiers = combination.keyboardModifiers();
    if (modifiers.testFlag(Qt::ShiftModifier)) {
        binding.modifiers |= kShiftMask;
    }
    if (modifiers.testFlag(Qt::ControlModifier)) {
        binding.modifiers |= kControlMask;
    }
    if (modifiers.testFlag(Qt::AltModifier)) {
        binding.modifiers |= kMod1Mask;
    }
    if (modifiers.testFlag(Qt::MetaModifier)) {
        binding.modifiers |= kMod4Mask;
    }

    // 3. 不带任何修饰键的组合会抢占普通输入，拒绝注册
    if (binding.modifiers == 0) {
        return binding;
    }

    binding.valid = true;
    return binding;
}

}  // namespace markshot::shortcuts
