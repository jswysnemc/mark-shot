#pragma once

#include <QPointer>
#include <QVector>
#include <QWidget>

namespace markshot {

/**
 * 【截图】【自身窗口策略】在一次截图回退链路中临时隐藏本进程顶层窗口。
 *
 * 该守卫只保存原本可见的窗口，并在销毁或显式恢复时重新显示这些窗口。
 */
class CaptureOwnWindowsGuard final {
public:
    /**
     * 创建自身窗口可见性守卫。
     * @param hideWindows 是否立即隐藏本进程可见顶层窗口。
     */
    explicit CaptureOwnWindowsGuard(bool hideWindows = true);

    /**
     * 恢复创建守卫前可见的顶层窗口。
     * @return 无返回值。
     */
    void restore();

    /**
     * 销毁自身窗口可见性守卫并恢复窗口。
     */
    ~CaptureOwnWindowsGuard();

    CaptureOwnWindowsGuard(const CaptureOwnWindowsGuard &) = delete;
    CaptureOwnWindowsGuard &operator=(const CaptureOwnWindowsGuard &) = delete;

private:
    struct HiddenWindowState {
        QPointer<QWidget> widget;
        Qt::WindowStates windowStates = Qt::WindowNoState;
    };

    QVector<HiddenWindowState> m_hiddenWindows;
    bool m_restored = false;
};

}  // namespace markshot
