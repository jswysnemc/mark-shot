#include "recording/ui/recording_control_icons.h"

#include <QGuiApplication>
#include <QPainter>
#include <QPainterPath>
#include <QPixmap>

namespace markshot::recording::ui {
namespace {

// 控制条按钮的逻辑图标边长
constexpr int kIconSize = 18;

/**
 * 创建按设备像素比缩放的透明画布。
 * @return 透明像素图。
 */
QPixmap makeCanvas()
{
    const qreal ratio = qGuiApp ? qGuiApp->devicePixelRatio() : 1.0;
    QPixmap pixmap(QSize(kIconSize, kIconSize) * ratio);
    pixmap.setDevicePixelRatio(ratio);
    pixmap.fill(Qt::transparent);
    return pixmap;
}

/**
 * 在画布上准备抗锯齿画笔。
 * @param painter 画笔。
 * @param ink 填充颜色。
 * @return 无返回值。
 */
void prepare(QPainter &painter, const QColor &ink)
{
    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.setPen(Qt::NoPen);
    painter.setBrush(ink);
}

}  // namespace

QIcon makeRecordingPauseIcon(const QColor &ink)
{
    QPixmap pixmap = makeCanvas();
    QPainter painter(&pixmap);
    prepare(painter, ink);
    // 两条等宽竖条表示暂停
    const qreal barWidth = 4.0;
    const qreal gap = 4.0;
    const qreal height = 12.0;
    const qreal top = (kIconSize - height) / 2.0;
    const qreal left = (kIconSize - barWidth * 2 - gap) / 2.0;
    painter.drawRoundedRect(QRectF(left, top, barWidth, height), 1.2, 1.2);
    painter.drawRoundedRect(QRectF(left + barWidth + gap, top, barWidth, height), 1.2, 1.2);
    painter.end();
    return QIcon(pixmap);
}

QIcon makeRecordingResumeIcon(const QColor &ink)
{
    QPixmap pixmap = makeCanvas();
    QPainter painter(&pixmap);
    prepare(painter, ink);
    // 右向三角表示继续
    QPainterPath path;
    path.moveTo(5.5, 3.5);
    path.lineTo(14.0, kIconSize / 2.0);
    path.lineTo(5.5, kIconSize - 3.5);
    path.closeSubpath();
    painter.drawPath(path);
    painter.end();
    return QIcon(pixmap);
}

QIcon makeRecordingStopIcon(const QColor &ink)
{
    QPixmap pixmap = makeCanvas();
    QPainter painter(&pixmap);
    prepare(painter, ink);
    // 圆角方块表示停止
    const qreal side = 11.0;
    const qreal offset = (kIconSize - side) / 2.0;
    painter.drawRoundedRect(QRectF(offset, offset, side, side), 2.0, 2.0);
    painter.end();
    return QIcon(pixmap);
}

}  // namespace markshot::recording::ui
