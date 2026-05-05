#include "ui/icons.h"

#include <QFont>
#include <QPainter>
#include <QPainterPath>
#include <QPen>
#include <QPixmap>
#include <QPolygonF>
#include <QStringLiteral>

namespace markshot::ui {

namespace {

constexpr int kIconSize = 32;

// All glyphs share a single ink color so the toolbar reads as one family.
// Active-state inversion is handled by the button stylesheet (teal background +
// dark text), so a glyph drawn in slate-200 stays legible across normal,
// hover, and active states.
const QColor kInk(229, 231, 235);            // slate-200
const QColor kInkSoft(229, 231, 235, 130);   // slate-200 @ 50%
const QColor kInkFaint(229, 231, 235, 80);   // slate-200 @ 30%
const QColor kSaveInk(31, 19, 0);            // dark amber for primary button

QPen makePen(QColor color, qreal width = 1.75)
{
    return QPen(color, width, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin);
}

}  // namespace

QString actionName(ShotWindow::Action action)
{
    switch (action) {
    case ShotWindow::Action::ToolMove:
        return QStringLiteral("Move");
    case ShotWindow::Action::ToolSelect:
        return QStringLiteral("Select");
    case ShotWindow::Action::ToolPen:
        return QStringLiteral("Pen");
    case ShotWindow::Action::ToolLine:
        return QStringLiteral("Line");
    case ShotWindow::Action::ToolHighlighter:
        return QStringLiteral("Highlighter");
    case ShotWindow::Action::ToolRectangle:
        return QStringLiteral("Rect");
    case ShotWindow::Action::ToolEllipse:
        return QStringLiteral("Ellipse");
    case ShotWindow::Action::ToolArrow:
        return QStringLiteral("Arrow");
    case ShotWindow::Action::ToolText:
        return QStringLiteral("Text");
    case ShotWindow::Action::ToolNumber:
        return QStringLiteral("Number");
    case ShotWindow::Action::ToolMosaic:
        return QStringLiteral("Mosaic");
    case ShotWindow::Action::ToolLaser:
        return QStringLiteral("Laser");
    case ShotWindow::Action::ToggleCaptureScope:
        return QStringLiteral("Scope");
    case ShotWindow::Action::ToggleToolbarLayout:
        return QStringLiteral("Layout");
    case ShotWindow::Action::Clear:
        return QStringLiteral("Clear");
    case ShotWindow::Action::Undo:
        return QStringLiteral("Undo");
    case ShotWindow::Action::Redo:
        return QStringLiteral("Redo");
    case ShotWindow::Action::OpenWith:
        return QStringLiteral("Open With");
    case ShotWindow::Action::Pin:
        return QStringLiteral("Pin");
    case ShotWindow::Action::Copy:
        return QStringLiteral("Copy");
    case ShotWindow::Action::Save:
        return QStringLiteral("Save");
    case ShotWindow::Action::Cancel:
        return QStringLiteral("Cancel");
    }
    return {};
}

QIcon makeToolIcon(ShotWindow::Action action)
{
    QPixmap pixmap(kIconSize, kIconSize);
    pixmap.fill(Qt::transparent);

    QPainter p(&pixmap);
    p.setRenderHint(QPainter::Antialiasing, true);
    p.setRenderHint(QPainter::TextAntialiasing, true);
    p.setBrush(Qt::NoBrush);
    p.setPen(makePen(kInk));

    switch (action) {
    case ShotWindow::Action::ToolMove: {
        // Four-direction cross with a small anchor dot at the pivot — reads
        // as "move with respect to a reference point" rather than just an
        // arrow cluster.
        p.setPen(makePen(kInk, 1.8));
        p.drawLine(QPointF(16, 7), QPointF(16, 25));
        p.drawLine(QPointF(7, 16), QPointF(25, 16));
        QPainterPath head;
        head.moveTo(12, 11); head.lineTo(16, 7);  head.lineTo(20, 11);
        head.moveTo(12, 21); head.lineTo(16, 25); head.lineTo(20, 21);
        head.moveTo(11, 12); head.lineTo(7, 16);  head.lineTo(11, 20);
        head.moveTo(21, 12); head.lineTo(25, 16); head.lineTo(21, 20);
        p.drawPath(head);
        // anchor dot
        p.setPen(Qt::NoPen);
        p.setBrush(kInk);
        p.drawEllipse(QPointF(16, 16), 1.4, 1.4);
        p.setBrush(Qt::NoBrush);
        break;
    }
    case ShotWindow::Action::ToolSelect: {
        // Classic mouse cursor. Filled body keeps it readable at 32px.
        QPainterPath cursor;
        cursor.moveTo(9.0, 6.5);
        cursor.lineTo(9.0, 23.0);
        cursor.lineTo(13.2, 18.8);
        cursor.lineTo(15.7, 25.0);
        cursor.lineTo(18.0, 24.0);
        cursor.lineTo(15.5, 17.9);
        cursor.lineTo(21.5, 17.0);
        cursor.closeSubpath();
        p.setPen(makePen(kInk, 1.5));
        p.setBrush(QColor(229, 231, 235, 50));
        p.drawPath(cursor);
        p.setBrush(Qt::NoBrush);
        break;
    }
    case ShotWindow::Action::ToolPen: {
        // Tilted fountain-pen body with a wedge nib and a small ink dot at
        // the tip. Drawn in a rotated frame so the body, nib, and cap all
        // line up on the same axis.
        p.save();
        p.translate(16, 16);
        p.rotate(-45);

        // pen body — outlined rectangle with a divider near the cap
        p.setPen(makePen(kInk, 1.6));
        QRectF body(-2.4, -10.0, 4.8, 14.0);
        p.drawRect(body);
        p.drawLine(QPointF(-2.4, -6.0), QPointF(2.4, -6.0));

        // cap — solid block at the rear
        p.setPen(Qt::NoPen);
        p.setBrush(kInk);
        p.drawRect(QRectF(-2.4, -10.0, 4.8, 2.4));

        // nib — solid triangle at the front
        QPainterPath nib;
        nib.moveTo(-2.4, 4.0);
        nib.lineTo(0.0, 8.6);
        nib.lineTo(2.4, 4.0);
        nib.closeSubpath();
        p.drawPath(nib);

        // ink slit on the nib
        p.setPen(makePen(QColor(15, 17, 23, 200), 0.9));
        p.drawLine(QPointF(0.0, 4.6), QPointF(0.0, 7.6));

        p.setBrush(Qt::NoBrush);
        p.restore();
        break;
    }
    case ShotWindow::Action::ToolLine:
        // Single clean diagonal stroke with subtle round endpoints to imply
        // "draw a line between two points".
        p.setPen(makePen(kInk, 2.0));
        p.drawLine(QPointF(8, 24), QPointF(24, 8));
        p.setPen(Qt::NoPen);
        p.setBrush(kInk);
        p.drawEllipse(QPointF(8, 24), 1.6, 1.6);
        p.drawEllipse(QPointF(24, 8), 1.6, 1.6);
        p.setBrush(Qt::NoBrush);
        break;
    case ShotWindow::Action::ToolHighlighter: {
        // Translucent stroke laid down underneath, then a marker rendered in
        // a rotated frame so the chisel tip points along the trail.
        p.setPen(QPen(kInkFaint, 5.5, Qt::SolidLine, Qt::RoundCap));
        p.drawLine(QPointF(7, 27), QPointF(22, 27));

        p.save();
        p.translate(15.5, 15.5);
        p.rotate(-45);

        // body — outlined rectangle, slightly wider than the pen
        p.setPen(makePen(kInk, 1.6));
        QRectF body(-3.4, -8.6, 6.8, 12.0);
        p.drawRect(body);
        p.drawLine(QPointF(-3.4, -4.6), QPointF(3.4, -4.6));

        // chisel tip — wider trapezoid that flares past the body
        QPainterPath tip;
        tip.moveTo(-3.4, 3.4);
        tip.lineTo(-4.6, 6.4);
        tip.lineTo(4.6, 6.4);
        tip.lineTo(3.4, 3.4);
        tip.closeSubpath();
        p.setPen(Qt::NoPen);
        p.setBrush(kInk);
        p.drawPath(tip);

        // small protruding writing tip
        p.setBrush(Qt::NoBrush);
        p.setPen(makePen(kInk, 1.4));
        p.drawLine(QPointF(-1.6, 6.4), QPointF(-1.6, 8.4));
        p.drawLine(QPointF(1.6, 6.4), QPointF(1.6, 8.4));
        p.drawLine(QPointF(-1.6, 8.4), QPointF(1.6, 8.4));

        p.restore();
        break;
    }
    case ShotWindow::Action::ToolRectangle:
        p.setPen(makePen(kInk, 1.9));
        p.drawRoundedRect(QRectF(7.5, 10.5, 17, 13), 2.6, 2.6);
        break;
    case ShotWindow::Action::ToolEllipse:
        p.setPen(makePen(kInk, 1.9));
        p.drawEllipse(QRectF(7, 10, 18, 13));
        break;
    case ShotWindow::Action::ToolArrow: {
        // Diagonal shaft tucked under a solid arrowhead. The head reaches
        // the corner so the silhouette reads as "arrow" even at small sizes.
        p.setPen(makePen(kInk, 2.0));
        p.drawLine(QPointF(8, 24), QPointF(20.5, 11.5));
        QPainterPath head;
        head.moveTo(25.0, 7.0);
        head.lineTo(15.5, 8.0);
        head.lineTo(24.0, 16.5);
        head.closeSubpath();
        p.setPen(makePen(kInk, 1.4));
        p.setBrush(kInk);
        p.drawPath(head);
        p.setBrush(Qt::NoBrush);
        break;
    }
    case ShotWindow::Action::ToolText: {
        // Capital T with serif feet. The thicker vertical reads as a glyph
        // rather than a plus sign at small sizes.
        p.setPen(makePen(kInk, 2.2));
        p.drawLine(QPointF(8, 9.5), QPointF(24, 9.5));
        p.setPen(makePen(kInk, 2.4));
        p.drawLine(QPointF(16, 10), QPointF(16, 23.5));
        p.setPen(makePen(kInk, 1.6));
        // top serifs
        p.drawLine(QPointF(8, 8), QPointF(8, 11));
        p.drawLine(QPointF(24, 8), QPointF(24, 11));
        // bottom serif
        p.drawLine(QPointF(13, 23.5), QPointF(19, 23.5));
        break;
    }
    case ShotWindow::Action::ToolNumber: {
        // Outlined disc with a numeral 1 — replaces the filled orange chip.
        p.setPen(makePen(kInk, 1.9));
        p.drawEllipse(QRectF(5.5, 5.5, 21, 21));
        QFont font(QStringLiteral("Sans Serif"), 13, QFont::Bold);
        p.setFont(font);
        p.setPen(kInk);
        p.drawText(QRectF(5.5, 5.0, 21, 21), Qt::AlignCenter, QStringLiteral("1"));
        break;
    }
    case ShotWindow::Action::ToolMosaic: {
        // 3x3 grid with alternating tile brightness — the staggered fills
        // imply pixelation rather than a plain checkerboard.
        p.setPen(Qt::NoPen);
        const qreal tile = 5.0;
        const qreal gap = 1.5;
        const qreal origin = 16.0 - (tile * 3 + gap * 2) / 2.0;
        for (int row = 0; row < 3; ++row) {
            for (int col = 0; col < 3; ++col) {
                const bool bright = ((row + col) % 2) == 0;
                const int alpha = bright ? 235 : 95;
                p.setBrush(QColor(229, 231, 235, alpha));
                const qreal x = origin + col * (tile + gap);
                const qreal y = origin + row * (tile + gap);
                p.drawRoundedRect(QRectF(x, y, tile, tile), 1.0, 1.0);
            }
        }
        break;
    }
    case ShotWindow::Action::ToolLaser: {
        p.setPen(QPen(QColor(248, 113, 113, 90), 9.0, Qt::SolidLine, Qt::RoundCap));
        p.drawLine(QPointF(7, 22), QPointF(24, 10));
        p.setPen(QPen(QColor(248, 113, 113, 210), 3.6, Qt::SolidLine, Qt::RoundCap));
        p.drawLine(QPointF(7, 22), QPointF(24, 10));
        p.setPen(QPen(QColor(255, 255, 255, 210), 1.2, Qt::SolidLine, Qt::RoundCap));
        p.drawLine(QPointF(7, 22), QPointF(24, 10));
        p.setPen(Qt::NoPen);
        p.setBrush(QColor(248, 113, 113, 220));
        p.drawEllipse(QPointF(24, 10), 3.0, 3.0);
        p.setBrush(Qt::NoBrush);
        break;
    }
    case ShotWindow::Action::ToggleCaptureScope: {
        p.setPen(makePen(kInk, 1.7));
        p.drawRoundedRect(QRectF(6.5, 8.5, 19.0, 15.0), 2.4, 2.4);
        p.setPen(makePen(kInkSoft, 1.3));
        p.drawRoundedRect(QRectF(10.0, 12.0, 12.0, 8.0), 1.8, 1.8);
        p.setPen(makePen(kInk, 1.6));
        QPainterPath arrows;
        arrows.moveTo(9.0, 6.5);
        arrows.lineTo(6.5, 6.5);
        arrows.lineTo(6.5, 9.0);
        arrows.moveTo(23.0, 25.5);
        arrows.lineTo(25.5, 25.5);
        arrows.lineTo(25.5, 23.0);
        p.drawPath(arrows);
        break;
    }
    case ShotWindow::Action::ToggleToolbarLayout: {
        p.setPen(makePen(kInk, 1.6));
        p.drawRoundedRect(QRectF(7.0, 8.0, 18.0, 5.0), 1.5, 1.5);
        p.drawRoundedRect(QRectF(7.0, 19.0, 18.0, 5.0), 1.5, 1.5);
        p.setPen(makePen(kInkSoft, 1.4));
        p.drawRoundedRect(QRectF(9.0, 6.5, 5.0, 19.0), 1.5, 1.5);
        p.drawRoundedRect(QRectF(18.0, 6.5, 5.0, 19.0), 1.5, 1.5);
        break;
    }
    case ShotWindow::Action::Clear: {
        // Trash can: handle bracket on top, lid bar, then a slightly tapered
        // body with three vertical ribs.
        p.setPen(makePen(kInk, 1.8));
        // handle
        p.drawLine(QPointF(13, 8), QPointF(19, 8));
        p.drawLine(QPointF(13, 8), QPointF(13, 10));
        p.drawLine(QPointF(19, 8), QPointF(19, 10));
        // lid
        p.drawLine(QPointF(8, 10.5), QPointF(24, 10.5));
        // body — slight taper toward the bottom
        QPainterPath body;
        body.moveTo(10.2, 12.4);
        body.lineTo(11.4, 25.0);
        body.lineTo(20.6, 25.0);
        body.lineTo(21.8, 12.4);
        p.drawPath(body);
        // bottom curve to close the can
        p.drawLine(QPointF(11.4, 25.0), QPointF(20.6, 25.0));
        // ribs
        p.setPen(makePen(kInkSoft, 1.3));
        p.drawLine(QPointF(13.6, 14.0), QPointF(13.9, 23.0));
        p.drawLine(QPointF(16.0, 14.0), QPointF(16.0, 23.0));
        p.drawLine(QPointF(18.4, 14.0), QPointF(18.1, 23.0));
        break;
    }
    case ShotWindow::Action::Undo: {
        // Three-quarter arc sweeping clockwise from upper-left, terminating
        // in a solid arrowhead that points back toward the start.
        p.setPen(makePen(kInk, 2.1));
        QPainterPath arc;
        arc.moveTo(9, 13);
        arc.cubicTo(12, 7, 22, 7, 24.5, 14.5);
        arc.cubicTo(26.5, 21.5, 18.5, 25, 14, 22);
        p.drawPath(arc);
        // arrowhead at the start of the arc
        QPainterPath head;
        head.moveTo(5.0, 9.0);
        head.lineTo(9.5, 13.6);
        head.lineTo(13.5, 10.5);
        head.closeSubpath();
        p.setBrush(kInk);
        p.setPen(Qt::NoPen);
        p.drawPath(head);
        p.setBrush(Qt::NoBrush);
        break;
    }
    case ShotWindow::Action::Redo: {
        p.setPen(makePen(kInk, 2.1));
        QPainterPath arc;
        arc.moveTo(23, 13);
        arc.cubicTo(20, 7, 10, 7, 7.5, 14.5);
        arc.cubicTo(5.5, 21.5, 13.5, 25, 18, 22);
        p.drawPath(arc);
        QPainterPath head;
        head.moveTo(27.0, 9.0);
        head.lineTo(22.5, 13.6);
        head.lineTo(18.5, 10.5);
        head.closeSubpath();
        p.setBrush(kInk);
        p.setPen(Qt::NoPen);
        p.drawPath(head);
        p.setBrush(Qt::NoBrush);
        break;
    }
    case ShotWindow::Action::OpenWith: {
        // Box with an arrow leaving the upper-right corner.
        p.setPen(makePen(kInk, 1.8));
        p.drawRoundedRect(QRectF(7, 11, 13, 13), 2.5, 2.5);
        p.drawLine(QPointF(14, 18), QPointF(24.5, 7.5));
        // arrow head (open angle)
        p.drawLine(QPointF(18, 7.5), QPointF(24.5, 7.5));
        p.drawLine(QPointF(24.5, 7.5), QPointF(24.5, 14));
        break;
    }
    case ShotWindow::Action::Pin: {
        p.setPen(makePen(kInk, 1.8));
        p.drawLine(QPointF(16, 17), QPointF(16, 26));
        p.drawLine(QPointF(12.5, 26), QPointF(19.5, 26));
        QPainterPath pin;
        pin.moveTo(10.0, 7.0);
        pin.lineTo(22.0, 7.0);
        pin.lineTo(20.2, 15.0);
        pin.lineTo(24.0, 18.5);
        pin.lineTo(8.0, 18.5);
        pin.lineTo(11.8, 15.0);
        pin.closeSubpath();
        p.setBrush(QColor(229, 231, 235, 70));
        p.drawPath(pin);
        p.setBrush(Qt::NoBrush);
        break;
    }
    case ShotWindow::Action::Copy: {
        // Two stacked rounded rectangles. The back card is dimmed so the
        // overlap reads as depth.
        p.setPen(makePen(kInkSoft, 1.5));
        p.drawRoundedRect(QRectF(11.5, 7.5, 13, 14), 2.5, 2.5);
        p.setPen(makePen(kInk, 1.8));
        p.drawRoundedRect(QRectF(7.5, 11.5, 13, 14), 2.5, 2.5);
        break;
    }
    case ShotWindow::Action::Save: {
        // Floppy disk silhouette in dark amber to contrast the orange button.
        p.setPen(makePen(kSaveInk, 2.0));
        p.drawRoundedRect(QRectF(7, 7, 18, 18), 2.4, 2.4);
        // sliding cover (top)
        p.drawLine(QPointF(11, 7), QPointF(11, 11.5));
        p.drawLine(QPointF(20, 7), QPointF(20, 11.5));
        p.drawLine(QPointF(11, 11.5), QPointF(20, 11.5));
        // label area (bottom)
        p.setPen(makePen(kSaveInk, 1.5));
        p.drawRoundedRect(QRectF(10, 15, 12, 9), 1.0, 1.0);
        p.drawLine(QPointF(12, 18.5), QPointF(20, 18.5));
        p.drawLine(QPointF(12, 21), QPointF(20, 21));
        break;
    }
    case ShotWindow::Action::Cancel: {
        // Slate cross — danger hue is delivered by the button background on
        // hover, not by the glyph itself.
        p.setPen(makePen(kInk, 2.1));
        p.drawLine(QPointF(10, 10), QPointF(22, 22));
        p.drawLine(QPointF(22, 10), QPointF(10, 22));
        break;
    }
    }

    p.end();
    return QIcon(pixmap);
}

QIcon makeFillIcon(bool filled)
{
    QPixmap pixmap(kIconSize, kIconSize);
    pixmap.fill(Qt::transparent);

    QPainter p(&pixmap);
    p.setRenderHint(QPainter::Antialiasing, true);

    const QRectF outer(6.5, 6.5, 19, 19);
    p.setPen(makePen(kInk, 1.9));

    if (filled) {
        // Outline plus a slightly inset solid plate so the silhouette reads
        // as "filled" without merging into the button border.
        p.setBrush(Qt::NoBrush);
        p.drawRoundedRect(outer, 3.4, 3.4);
        p.setPen(Qt::NoPen);
        p.setBrush(kInk);
        p.drawRoundedRect(outer.adjusted(2.4, 2.4, -2.4, -2.4), 1.8, 1.8);
    } else {
        // Empty rounded rectangle with a faint diagonal ghost stroke to
        // suggest "no fill" without making the icon feel decorative.
        p.setBrush(Qt::NoBrush);
        p.drawRoundedRect(outer, 3.4, 3.4);
        p.setPen(makePen(kInkFaint, 1.4));
        p.drawLine(QPointF(9.5, 22.5), QPointF(22.5, 9.5));
    }

    p.end();
    return QIcon(pixmap);
}

}  // namespace markshot::ui
