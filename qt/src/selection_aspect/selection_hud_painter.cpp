#include "selection_aspect/selection_hud_painter.h"

#include "ui/theme.h"

#include <QPainter>
#include <QPainterPath>
#include <QPen>

#include <algorithm>

namespace markshot::selection_aspect::hud_style {

QFont hudFont()
{
    return markshot::theme::uiFont(10, QFont::DemiBold);
}

void paintPanelBackground(QPainter &painter, const QRectF &rect, const HudColors &colors)
{
    painter.save();
    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.setPen(QPen(colors.panelBorder, 1.0));
    painter.setBrush(colors.panelFill);
    painter.drawRoundedRect(rect.adjusted(0.5, 0.5, -0.5, -0.5), kHudRadius, kHudRadius);
    painter.restore();
}

void paintLockGlyph(QPainter &painter, const QRectF &box, bool locked, const QColor &color)
{
    painter.save();
    painter.setRenderHint(QPainter::Antialiasing, true);
    const qreal unit = std::min(box.width(), box.height());
    const QRectF square(box.center().x() - unit / 2.0, box.center().y() - unit / 2.0, unit, unit);

    // 1. 锁体占下半部分
    const QRectF body(square.left() + unit * 0.18, square.top() + unit * 0.46, unit * 0.64, unit * 0.46);
    painter.setPen(Qt::NoPen);
    painter.setBrush(color);
    painter.drawRoundedRect(body, unit * 0.1, unit * 0.1);

    // 2. 锁梁：闭合时两端落在锁体上，打开时右端抬起
    const qreal shackleWidth = unit * 0.4;
    const qreal shackleLeft = body.center().x() - shackleWidth / 2.0;
    const qreal shackleTop = square.top() + unit * 0.08;
    const qreal openLift = locked ? 0.0 : unit * 0.14;
    QPainterPath shackle;
    shackle.moveTo(shackleLeft, body.top());
    shackle.lineTo(shackleLeft, shackleTop + shackleWidth / 2.0);
    shackle.arcTo(QRectF(shackleLeft, shackleTop, shackleWidth, shackleWidth), 180.0, -180.0);
    shackle.lineTo(shackleLeft + shackleWidth, body.top() - openLift);
    QPen pen(color, std::max<qreal>(1.4, unit * 0.12));
    pen.setCapStyle(Qt::RoundCap);
    painter.setPen(pen);
    painter.setBrush(Qt::NoBrush);
    painter.drawPath(shackle);
    painter.restore();
}

void paintRatioGlyph(QPainter &painter, const QRectF &box, qreal ratio, const QColor &color)
{
    painter.save();
    painter.setRenderHint(QPainter::Antialiasing, true);
    QPen pen(color, 1.3);

    // 1. 自由比例绘制虚线方框
    if (ratio <= 0.0) {
        pen.setStyle(Qt::DashLine);
        pen.setDashPattern({2.0, 2.0});
        painter.setPen(pen);
        painter.setBrush(Qt::NoBrush);
        const QRectF frame = box.adjusted(box.width() * 0.12, box.height() * 0.12, -box.width() * 0.12, -box.height() * 0.12);
        painter.drawRoundedRect(frame, 1.5, 1.5);
        painter.restore();
        return;
    }

    // 2. 固定比例时把矩形等比放进绘制框
    const QRectF inner = box.adjusted(1.0, 1.0, -1.0, -1.0);
    qreal width = inner.width();
    qreal height = width / ratio;
    if (height > inner.height()) {
        height = inner.height();
        width = height * ratio;
    }
    const QRectF frame(inner.center().x() - width / 2.0, inner.center().y() - height / 2.0, width, height);
    painter.setPen(pen);
    painter.setBrush(Qt::NoBrush);
    painter.drawRoundedRect(frame, 1.5, 1.5);
    painter.restore();
}

void paintChevronGlyph(QPainter &painter, const QRectF &box, const QColor &color)
{
    painter.save();
    painter.setRenderHint(QPainter::Antialiasing, true);
    QPen pen(color, 1.4);
    pen.setCapStyle(Qt::RoundCap);
    pen.setJoinStyle(Qt::RoundJoin);
    painter.setPen(pen);
    const QPointF center = box.center();
    const qreal half = std::min(box.width(), box.height()) * 0.24;
    const QPointF points[] = {
        QPointF(center.x() - half, center.y() - half / 2.0),
        QPointF(center.x(), center.y() + half / 2.0),
        QPointF(center.x() + half, center.y() - half / 2.0),
    };
    painter.drawPolyline(points, 3);
    painter.restore();
}

void paintCheckGlyph(QPainter &painter, const QRectF &box, const QColor &color)
{
    painter.save();
    painter.setRenderHint(QPainter::Antialiasing, true);
    QPen pen(color, 1.6);
    pen.setCapStyle(Qt::RoundCap);
    pen.setJoinStyle(Qt::RoundJoin);
    painter.setPen(pen);
    const qreal unit = std::min(box.width(), box.height());
    const QPointF origin(box.center().x() - unit / 2.0, box.center().y() - unit / 2.0);
    const QPointF points[] = {
        origin + QPointF(unit * 0.2, unit * 0.52),
        origin + QPointF(unit * 0.42, unit * 0.72),
        origin + QPointF(unit * 0.8, unit * 0.3),
    };
    painter.drawPolyline(points, 3);
    painter.restore();
}

}  // namespace markshot::selection_aspect::hud_style
