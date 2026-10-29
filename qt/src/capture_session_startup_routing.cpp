#include "capture_session_startup_routing.h"

#include "shot_window.h"

#include <QApplication>
#include <QCursor>
#include <QGuiApplication>
#include <QScreen>

namespace markshot::capture_session {

/**
 * 【截图会话】【工具转移】为覆盖层连接同一会话中的工具转移信号，录制工具连同待选区配置一起转移。
 * @param app 应用对象，作为信号接收上下文。
 * @param window 当前覆盖层。
 * @param windows 同一会话的覆盖层列表。
 * @return 无返回值。
 */
void connectStartupToolScreenRouting(QApplication *app, ShotWindow *window,
                                     const QVector<QPointer<ShotWindow>> &windows)
{
    if (!app || !window) {
        return;
    }
    // 1. 【截图会话】【工具转移】快捷键落点与指针屏幕不一致时转移原工具，录制工具连同配置一起转移
    QObject::connect(window, &ShotWindow::startupToolRouteRequested, app,
                     [windows](ShotWindow *source) {
        if (!source || !source->startupToolCanFollowPointer()) {
            return;
        }
        QScreen *cursorScreen = QGuiApplication::screenAt(QCursor::pos());
        if (!cursorScreen) {
            return;
        }
        for (const QPointer<ShotWindow> &peerWindow : windows) {
            ShotWindow *peer = peerWindow.data();
            if (!peer || peer == source || peer->captureScreen() != cursorScreen) {
                continue;
            }
            if (source->transferStartupToolTo(peer)) {
                peer->raise();
            }
            return;
        }
    });
    // 2. 【截图会话】【工具转移】指针进入空闲覆盖层时接收其他屏幕上的可转移工具
    QObject::connect(window, &ShotWindow::startupToolClaimRequested, app,
                     [windows](ShotWindow *claimer) {
        if (!claimer) {
            return;
        }
        ShotWindow *donor = nullptr;
        for (const QPointer<ShotWindow> &peerWindow : windows) {
            ShotWindow *peer = peerWindow.data();
            if (peer && peer != claimer
                && peer->startupToolCanFollowPointer()) {
                donor = peer;
                break;
            }
        }
        if (!donor) {
            return;
        }
        donor->transferStartupToolTo(claimer);
    });
}

}  // namespace markshot::capture_session
