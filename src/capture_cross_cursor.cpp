#include "capture_cross_cursor.h"

#include <QPainter>
#include <QPixmap>

namespace markshot::shot {

QCursor captureCrossCursor()
{
    // 1. 使用 64 像素宽度，为 32 位硬件光标缓冲提供 256 字节行步长
    constexpr int canvasSize = 64;
    constexpr int crossSize = 33;
    constexpr int offset = (canvasSize - crossSize) / 2;
    constexpr int center = crossSize / 2;
    QPixmap pixmap(canvasSize, canvasSize);
    pixmap.fill(Qt::transparent);

    // 2. 保留原图案的裁剪范围，避免方形笔帽随画布扩大而伸长
    QPainter painter(&pixmap);
    painter.setRenderHint(QPainter::Antialiasing, false);
    painter.translate(offset, offset);
    painter.setClipRect(QRect(0, 0, crossSize, crossSize));
    painter.setPen(QPen(QColor(15, 23, 42, 235), 5, Qt::SolidLine, Qt::SquareCap));
    painter.drawLine(center, 0, center, crossSize - 1);
    painter.drawLine(0, center, crossSize - 1, center);
    painter.setPen(QPen(QColor(255, 255, 255, 245), 3, Qt::SolidLine, Qt::SquareCap));
    painter.drawLine(center, 0, center, crossSize - 1);
    painter.drawLine(0, center, crossSize - 1, center);
    painter.setPen(QPen(QColor(45, 212, 191, 255), 1, Qt::SolidLine, Qt::SquareCap));
    painter.drawLine(center, 0, center, crossSize - 1);
    painter.drawLine(0, center, crossSize - 1, center);
    painter.end();

    // 3. 热点随图案平移，保持选择位置与十字交点一致
    return QCursor(pixmap, offset + center, offset + center);
}

}
