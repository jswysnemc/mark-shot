#include "capture_own_windows_guard.h"

#include "debug_log.h"

#include <QApplication>
#include <QEventLoop>
#include <QWidget>

namespace markshot {

CaptureOwnWindowsGuard::CaptureOwnWindowsGuard(bool hideWindows)
{
    if (!hideWindows || !QApplication::instance()) {
        m_restored = true;
        return;
    }

    const QWidgetList topLevelWidgets = QApplication::topLevelWidgets();
    for (QWidget *widget : topLevelWidgets) {
        if (!widget || !widget->isVisible()
            || widget->testAttribute(Qt::WA_DontShowOnScreen)) {
            continue;
        }
        m_hiddenWindows.append({widget, widget->windowState()});
        widget->hide();
    }

    if (!m_hiddenWindows.isEmpty()) {
        QApplication::processEvents(QEventLoop::ExcludeUserInputEvents);
    }
    markshot::debugLog("capture",
                       "【截图】【自身窗口策略】temporarily-hidden=%d",
                       m_hiddenWindows.size());
}

CaptureOwnWindowsGuard::~CaptureOwnWindowsGuard()
{
    restore();
}

void CaptureOwnWindowsGuard::restore()
{
    if (m_restored) {
        return;
    }
    m_restored = true;

    int restoredCount = 0;
    for (const HiddenWindowState &hiddenWindow : m_hiddenWindows) {
        QWidget *widget = hiddenWindow.widget.data();
        if (!widget) {
            continue;
        }
        widget->show();
        const Qt::WindowStates restorableStates = hiddenWindow.windowStates
            & (Qt::WindowMinimized | Qt::WindowMaximized | Qt::WindowFullScreen);
        if (restorableStates.testFlag(Qt::WindowFullScreen)) {
            widget->showFullScreen();
        } else if (restorableStates.testFlag(Qt::WindowMaximized)) {
            widget->showMaximized();
        } else if (restorableStates.testFlag(Qt::WindowMinimized)) {
            widget->showMinimized();
        }
        ++restoredCount;
    }
    if (restoredCount > 0 && QApplication::instance()) {
        QApplication::processEvents(QEventLoop::ExcludeUserInputEvents);
    }
    markshot::debugLog("capture",
                       "【截图】【自身窗口策略】restored=%d",
                       restoredCount);
    m_hiddenWindows.clear();
}

}  // namespace markshot
