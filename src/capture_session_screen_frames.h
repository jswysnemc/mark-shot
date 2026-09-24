#pragma once

#include "window_detection.h"

#include <QImage>
#include <QList>
#include <QPointer>
#include <QRect>
#include <QScreen>
#include <QString>
#include <QVector>

namespace markshot::capture_session {

/**
 * 单个显示器的冻结帧及其窗口检测结果。
 */
struct CapturedScreenFrame {
    QPointer<QScreen> screen;
    QImage image;
    QString outputName;
    QRect sourceGeometry;
    QVector<markshot::WindowInfo> windowInfos;
    bool detectWindows = false;
};

/**
 * 逐个显示器捕获冻结图，但暂不创建覆盖窗口。
 * @param screens 当前屏幕列表。
 * @param includeCursor 冻结图是否包含鼠标。
 * @param hideOwnWindows 是否让截屏后端隐藏 mark-shot 自身窗口。
 * @param error 输出错误信息。
 * @return 捕获成功的逐屏冻结帧列表，任一屏幕失败时返回空列表。
 */
QVector<CapturedScreenFrame> captureScreensIndividually(const QList<QScreen *> &screens,
                                                        bool includeCursor,
                                                        bool hideOwnWindows,
                                                        QString *error);

}  // namespace markshot::capture_session
