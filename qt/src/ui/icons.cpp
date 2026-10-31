#include "ui/icons.h"

#include "shot_window_marker_shapes.h"
#include "ui/icon_manifest.h"
#include "ui/svg_icon.h"

#include <QPainter>
#include <QPainterPath>
#include <QPen>

namespace markshot::ui {

namespace {

/// @brief 深色截图面板默认图标颜色，其他主题可通过参数覆盖
const QColor kOverlayInk(229, 231, 235);

/// @brief 主操作保存按钮位于强调色背景上，图标保持纯白
const QColor kPrimaryInk(255, 255, 255);

}  // namespace

QIcon makeToolIcon(ShotWindow::Action action)
{
    return makeToolIcon(action, QColor());
}

QIcon makeToolIcon(ShotWindow::Action action, QColor ink, QColor checkedInk)
{
    if (!ink.isValid()) {
        ink = action == ShotWindow::Action::Save ? kPrimaryInk : kOverlayInk;
    }
    return makeSvgIcon(toolIconName(action), SvgIconPalette{ink, checkedInk, QColor()});
}

QIcon makePropertyIcon(PropertyIcon icon, QColor ink)
{
    return makeSvgIcon(propertyIconName(icon), ink.isValid() ? ink : kOverlayInk);
}

QIcon makeFillIcon(bool filled, QColor ink)
{
    return makeSvgIcon(filled ? QStringLiteral("fill-on") : QStringLiteral("fill-off"),
                       ink.isValid() ? ink : kOverlayInk);
}

QIcon makeMarkerShapeIcon(ShotWindow::MarkerShape shape, QColor ink, bool filled)
{
    if (!ink.isValid()) {
        ink = kOverlayInk;
    }
    // 1. 复用实际标记几何，按 24 网格绘制，保证预览与最终标记轮廓一致
    const QPainterPath path = markshot::marker::pathForShape(shape, QRectF(4, 4, 16, 16));
    QIcon icon;
    // 2. 组合形状属于动态几何，提供多档高清位图，工具与属性图标由矢量引擎按需渲染
    for (int size : {16, 24, 32, 48, 64, 96}) {
        QPixmap pixmap(size, size);
        pixmap.fill(Qt::transparent);
        QPainter painter(&pixmap);
        painter.setRenderHint(QPainter::Antialiasing, true);
        painter.scale(size / 24.0, size / 24.0);
        if (filled) {
            painter.setPen(Qt::NoPen);
            painter.setBrush(ink);
        } else {
            painter.setPen(QPen(ink, 1.75, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
            painter.setBrush(Qt::NoBrush);
        }
        painter.drawPath(path);
        painter.end();
        icon.addPixmap(pixmap);
    }
    return icon;
}

}  // namespace markshot::ui
