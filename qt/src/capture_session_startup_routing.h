#pragma once

#include <QPointer>
#include <QVector>

class QApplication;
class ShotWindow;

namespace markshot::capture_session {

/**
 * 【截图会话】【工具转移】为覆盖层连接同一会话中的工具转移信号。
 * @param app 应用对象，作为信号接收上下文。
 * @param window 当前覆盖层。
 * @param windows 同一会话的覆盖层列表。
 * @return 无返回值。
 */
void connectStartupToolScreenRouting(QApplication *app, ShotWindow *window,
                                     const QVector<QPointer<ShotWindow>> &windows);

}  // namespace markshot::capture_session
