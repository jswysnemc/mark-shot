#include "ui/icons.h"

#include "ui/theme.h"

#include <QCoreApplication>
#include <QFileIconProvider>
#include <QFileInfo>
#include <QFont>
#include <QPainter>
#include <QPainterPath>
#include <QPen>
#include <QPixmap>
#include <QPolygonF>
#include <QStringLiteral>

namespace markshot::ui {

namespace {

/// @brief Default size for rendering icons.
constexpr int kIconSize = 32;

// All glyphs share a single ink color so the toolbar reads as one family.
// Active-state inversion is handled by the button stylesheet (teal background +
// dark text), so a glyph drawn in slate-200 stays legible across normal,
// hover, and active states.
    /// @brief Standard ink color (slate-200) shared by all glyphs.
    const QColor kInk(229, 231, 235);            // slate-200
    /// @brief Soft ink color variant (50% opacity).
    const QColor kInkSoft(229, 231, 235, 130);   // slate-200 @ 50%
    /// @brief Faint ink color variant (30% opacity).
    const QColor kInkFaint(229, 231, 235, 80);   // slate-200 @ 30%
    /// @brief Pure white ink used for the save/primary button.
    const QColor kSaveInk(255, 255, 255);        // pure white for primary button

/// @brief Helper to construct a standard QPen with rounded cap and join.
/// @param color Pen color.
/// @param width Pen width.
/// @return The constructed QPen object.
QPen makePen(QColor color, qreal width = 1.75)
{
    return QPen(color, width, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin);
}

/// @brief Returns a copy of the color with a specified alpha value.
/// @param color The original color.
/// @param alpha The new alpha value (0-255).
/// @return The updated QColor.
QColor withAlpha(QColor color, int alpha)
{
    color.setAlpha(alpha);
    return color;
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
    case ShotWindow::Action::ToolMagnifier:
        return QStringLiteral("Magnifier");
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
    case ShotWindow::Action::Extensions:
        return QStringLiteral("Extensions");
    case ShotWindow::Action::ScrollCapture:
        return QStringLiteral("Scroll Capture");
    case ShotWindow::Action::Pin:
        return QStringLiteral("Pin");
    case ShotWindow::Action::OcrCopy:
        return QStringLiteral("OCR Copy");
    case ShotWindow::Action::Copy:
        return QStringLiteral("Copy");
    case ShotWindow::Action::Save:
        return QStringLiteral("Save As");
    case ShotWindow::Action::Cancel:
        return QStringLiteral("Cancel");
    }
    return {};
}

QIcon applicationIcon()
{
#if defined(Q_OS_WIN)
    const QString applicationPath = QCoreApplication::applicationFilePath();
    if (!applicationPath.isEmpty()) {
        QFileIconProvider provider;
        const QIcon executableIcon = provider.icon(QFileInfo(applicationPath));
        if (!executableIcon.isNull()) {
            return executableIcon;
        }
    }
#endif
    const QIcon icon(QStringLiteral(":/icons/mark-shot.ico"));
    return icon.isNull() ? makeToolIcon(ShotWindow::Action::ToolSelect) : icon;
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
        // High-end minimalist 4-directional move cursor.
        p.setPen(makePen(kInk, 1.5));
        p.drawLine(QPointF(16, 7), QPointF(16, 25));
        p.drawLine(QPointF(7, 16), QPointF(25, 16));
        
        QPainterPath head;
        head.moveTo(12, 11); head.lineTo(16, 7); head.lineTo(20, 11); // Up
        head.moveTo(12, 21); head.lineTo(16, 25); head.lineTo(20, 21); // Down
        head.moveTo(11, 12); head.lineTo(7, 16); head.lineTo(11, 20); // Left
        head.moveTo(21, 12); head.lineTo(25, 16); head.lineTo(21, 20); // Right
        p.drawPath(head);
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
        // High-end elegant minimalist pen. A sleek diagonal stroke with a sharp nib and simple ink detail.
        p.save();
        p.translate(16, 16);
        p.rotate(-45);

        p.setPen(makePen(kInk, 1.5));
        
        QPainterPath penPath;
        penPath.moveTo(-2.0, -11.0);
        penPath.lineTo(2.0, -11.0);
        penPath.lineTo(2.0, 3.0);
        penPath.lineTo(0.0, 9.0);   // Tip
        penPath.lineTo(-2.0, 3.0);
        penPath.closeSubpath();
        p.drawPath(penPath);

        p.drawLine(QPointF(0.0, 3.0), QPointF(0.0, 9.0));
        
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
        // High-end minimalist highlighter. Elegantly curved soft brush line underneath a clean chisel pen.
        p.save();
        p.setPen(QPen(kInkFaint, 4.0, Qt::SolidLine, Qt::RoundCap));
        p.drawLine(QPointF(6, 25), QPointF(24, 25));
        p.restore();

        p.save();
        p.translate(15.5, 14.5);
        p.rotate(-45);

        p.setPen(makePen(kInk, 1.5));
        
        QRectF body(-2.5, -9.0, 5.0, 12.0);
        p.drawRect(body);
        p.drawLine(QPointF(-2.5, -5.0), QPointF(2.5, -5.0));

        QPainterPath tip;
        tip.moveTo(-2.5, 3.0);
        tip.lineTo(-1.5, 7.0);
        tip.lineTo(1.5, 7.0);
        tip.lineTo(2.5, 3.0);
        tip.closeSubpath();
        p.drawPath(tip);

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
        // Modern minimalist letter 'A' representing text annotation, sleek and extremely clean.
        p.setPen(makePen(kInk, 1.6));
        QPainterPath aPath;
        aPath.moveTo(16.0, 7.0);   // Top peak
        aPath.lineTo(9.5, 23.0);   // Bottom-left leg
        aPath.moveTo(16.0, 7.0);
        aPath.lineTo(22.5, 23.0);  // Bottom-right leg
        p.drawPath(aPath);
        p.drawLine(QPointF(11.5, 18.0), QPointF(20.5, 18.0)); // Crossbar
        break;
    }
    case ShotWindow::Action::ToolNumber: {
        // Outlined disc with a numeral 1 — replaces the filled orange chip.
        p.setPen(makePen(kInk, 1.9));
        p.drawEllipse(QRectF(5.5, 5.5, 21, 21));
        QFont font = markshot::theme::uiFont(13, QFont::Bold);
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
    case ShotWindow::Action::ToolMagnifier: {
        p.setPen(makePen(kInk, 1.9));
        p.drawEllipse(QRectF(6.5, 6.5, 15.0, 15.0));
        p.drawLine(QPointF(19.0, 19.0), QPointF(25.0, 25.0));
        p.setPen(makePen(kInkSoft, 1.3));
        p.drawLine(QPointF(10.5, 14.0), QPointF(17.5, 14.0));
        p.drawLine(QPointF(14.0, 10.5), QPointF(14.0, 17.5));
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
        // Highly recognizable minimalist Undo arrow. Sleek 45-degree tilted Feather/Lucide path perfectly snapping with corner head.
        p.setPen(makePen(kInk, 1.8));
        
        QPainterPath head;
        head.moveTo(6.0, 10.0);
        head.lineTo(6.0, 18.0);
        head.lineTo(14.0, 18.0);
        p.drawPath(head);

        QPainterPath arc;
        arc.moveTo(28.0, 22.0);
        arc.cubicTo(28.0, 14.0, 20.0, 10.0, 15.0, 10.0);
        arc.cubicTo(10.0, 10.0, 7.5, 14.0, 6.0, 18.0);
        p.drawPath(arc);
        break;
    }
    case ShotWindow::Action::Redo: {
        // Highly recognizable minimalist Redo arrow, symmetric to Undo.
        p.setPen(makePen(kInk, 1.8));
        
        QPainterPath head;
        head.moveTo(26.0, 10.0);
        head.lineTo(26.0, 18.0);
        head.lineTo(18.0, 18.0);
        p.drawPath(head);

        QPainterPath arc;
        arc.moveTo(4.0, 22.0);
        arc.cubicTo(4.0, 14.0, 12.0, 10.0, 17.0, 10.0);
        arc.cubicTo(22.0, 10.0, 24.5, 14.0, 26.0, 18.0);
        p.drawPath(arc);
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
    case ShotWindow::Action::Extensions: {
        // Terminal prompt with a spark mark: configurable commands.
        p.setPen(makePen(kInk, 1.7));
        p.drawRoundedRect(QRectF(6.5, 8.5, 19.0, 15.0), 2.6, 2.6);
        p.drawLine(QPointF(10.0, 13.0), QPointF(13.5, 16.0));
        p.drawLine(QPointF(10.0, 19.0), QPointF(13.5, 16.0));
        p.drawLine(QPointF(15.5, 19.0), QPointF(22.0, 19.0));
        p.setPen(makePen(kInkSoft, 1.3));
        p.drawLine(QPointF(23.5, 6.5), QPointF(23.5, 11.0));
        p.drawLine(QPointF(21.25, 8.75), QPointF(25.75, 8.75));
        break;
    }
    case ShotWindow::Action::ScrollCapture: {
        // Tall framed viewport with stacked content lines and a downward arrow,
        // suggesting a long capture that grows as the page scrolls down.
        p.setPen(makePen(kInk, 1.7));
        p.drawRoundedRect(QRectF(8.0, 5.5, 16.0, 21.0), 2.4, 2.4);
        p.setPen(makePen(kInkSoft, 1.3));
        p.drawLine(QPointF(11.5, 9.5), QPointF(20.5, 9.5));
        p.drawLine(QPointF(11.5, 12.5), QPointF(20.5, 12.5));
        // Downward arrow in the lower half implying the scroll direction.
        p.setPen(makePen(kInk, 1.7));
        p.drawLine(QPointF(16.0, 15.0), QPointF(16.0, 22.5));
        QPainterPath arrow;
        arrow.moveTo(12.5, 19.0);
        arrow.lineTo(16.0, 22.5);
        arrow.lineTo(19.5, 19.0);
        p.drawPath(arrow);
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
    case ShotWindow::Action::OcrCopy: {
        // Text recognition icon: document outline with "A" and scan lines
        p.setPen(makePen(kInk, 1.6));
        p.drawRoundedRect(QRectF(8.0, 6.5, 16.0, 19.0), 2.0, 2.0);
        // Letter "A" in center
        QPainterPath aPath;
        aPath.moveTo(16.0, 10.0);
        aPath.lineTo(12.5, 20.0);
        aPath.moveTo(16.0, 10.0);
        aPath.lineTo(19.5, 20.0);
        p.drawPath(aPath);
        p.drawLine(QPointF(13.8, 17.0), QPointF(18.2, 17.0));
        // Scan line hints
        p.setPen(makePen(kInkSoft, 1.2));
        p.drawLine(QPointF(11.0, 22.5), QPointF(21.0, 22.5));
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
        // Exquisite minimalist high-fidelity floppy disk representation.
        // Elegant geometry, thin lines and spacious negative space.
        p.setPen(makePen(kSaveInk, 1.6));
        p.drawRoundedRect(QRectF(7.5, 7.5, 17.0, 17.0), 2.0, 2.0);
        
        p.drawRoundedRect(QRectF(11.0, 7.5, 10.0, 5.0), 0.5, 0.5);
        p.drawLine(QPointF(13.5, 7.5), QPointF(13.5, 10.0));
        
        p.drawRoundedRect(QRectF(10.0, 15.5, 12.0, 9.0), 0.8, 0.8);
        p.drawLine(QPointF(12.5, 18.5), QPointF(19.5, 18.5));
        break;
    }
    case ShotWindow::Action::Cancel: {
        // Ultra-clean symmetrical cancel cross (1.6px stroke).
        p.setPen(makePen(kInk, 1.6));
        p.drawLine(QPointF(10, 10), QPointF(22, 22));
        p.drawLine(QPointF(22, 10), QPointF(10, 22));
        break;
    }
    }

    p.end();
    return QIcon(pixmap);
}

QIcon makePropertyIcon(PropertyIcon icon, QColor ink)
{
    const QColor iconInk = ink.isValid() ? ink : kInk;
    const QColor iconSoft = withAlpha(iconInk, 145);
    const QColor iconFaint = withAlpha(iconInk, 80);

    QPixmap pixmap(kIconSize, kIconSize);
    pixmap.fill(Qt::transparent);

    QPainter p(&pixmap);
    p.setRenderHint(QPainter::Antialiasing, true);
    p.setRenderHint(QPainter::TextAntialiasing, true);
    p.setBrush(Qt::NoBrush);
    p.setPen(makePen(iconInk));

    switch (icon) {
    case PropertyIcon::StrokeWidth:
        p.setPen(QPen(iconInk, 1.4, Qt::SolidLine, Qt::RoundCap));
        p.drawLine(QPointF(8, 9), QPointF(24, 9));
        p.setPen(QPen(iconInk, 2.2, Qt::SolidLine, Qt::RoundCap));
        p.drawLine(QPointF(8, 16), QPointF(24, 16));
        p.setPen(QPen(iconInk, 3.4, Qt::SolidLine, Qt::RoundCap));
        p.drawLine(QPointF(8, 23), QPointF(24, 23));
        break;
    case PropertyIcon::Opacity: {
        QPainterPath drop;
        drop.moveTo(16.0, 6.5);
        drop.cubicTo(11.5, 11.0, 9.0, 15.0, 9.0, 19.0);
        drop.cubicTo(9.0, 24.0, 12.6, 27.0, 16.0, 27.0);
        drop.cubicTo(19.4, 27.0, 23.0, 24.0, 23.0, 19.0);
        drop.cubicTo(23.0, 15.0, 20.5, 11.0, 16.0, 6.5);
        drop.closeSubpath();
        p.setPen(makePen(iconInk, 1.7));
        p.setBrush(withAlpha(iconInk, 42));
        p.drawPath(drop);
        p.setBrush(Qt::NoBrush);
        p.setPen(makePen(iconSoft, 1.3));
        p.drawLine(QPointF(11.5, 20.0), QPointF(20.5, 16.0));
        p.drawLine(QPointF(12.8, 23.0), QPointF(21.2, 19.2));
        break;
    }
    case PropertyIcon::Color: {
        QPainterPath brush;
        brush.moveTo(9.5, 22.5);
        brush.cubicTo(9.5, 18.8, 12.0, 16.6, 15.1, 16.2);
        brush.lineTo(22.7, 8.6);
        brush.cubicTo(23.6, 7.7, 25.0, 7.7, 25.8, 8.6);
        brush.cubicTo(26.6, 9.5, 26.6, 10.8, 25.7, 11.7);
        brush.lineTo(18.1, 19.3);
        brush.cubicTo(17.7, 22.6, 15.1, 25.0, 11.4, 25.0);
        brush.lineTo(8.0, 25.0);
        brush.cubicTo(8.9, 24.2, 9.5, 23.4, 9.5, 22.5);
        p.setPen(makePen(iconInk, 1.6));
        p.drawPath(brush);
        p.setPen(makePen(iconSoft, 1.2));
        p.drawLine(QPointF(17.1, 17.0), QPointF(20.9, 13.2));
        break;
    }
    case PropertyIcon::TextBackground: {
        p.setPen(makePen(iconSoft, 1.4));
        p.setBrush(withAlpha(iconInk, 34));
        p.drawRoundedRect(QRectF(7.0, 18.0, 18.0, 6.5), 2.0, 2.0);
        p.setBrush(Qt::NoBrush);
        p.setPen(makePen(iconInk, 1.6));
        QPainterPath textA;
        textA.moveTo(16.0, 7.0);
        textA.lineTo(10.5, 22.0);
        textA.moveTo(16.0, 7.0);
        textA.lineTo(21.5, 22.0);
        p.drawPath(textA);
        p.drawLine(QPointF(12.3, 17.0), QPointF(19.7, 17.0));
        break;
    }
    case PropertyIcon::CornerRadius: {
        p.setPen(makePen(iconFaint, 1.4));
        p.drawLine(QPointF(8.0, 24.0), QPointF(8.0, 13.5));
        p.drawLine(QPointF(18.5, 8.0), QPointF(24.0, 8.0));
        p.setPen(makePen(iconInk, 2.0));
        QPainterPath corner;
        corner.moveTo(8.0, 24.0);
        corner.lineTo(8.0, 15.5);
        corner.cubicTo(8.0, 10.5, 10.5, 8.0, 15.5, 8.0);
        corner.lineTo(24.0, 8.0);
        p.drawPath(corner);
        p.setPen(makePen(iconSoft, 1.2));
        p.drawRoundedRect(QRectF(15.0, 15.0, 9.0, 9.0), 2.5, 2.5);
        break;
    }
    case PropertyIcon::MagnifierScale: {
        p.setPen(makePen(iconInk, 1.7));
        p.drawEllipse(QRectF(7.0, 7.0, 12.5, 12.5));
        p.drawLine(QPointF(17.0, 17.0), QPointF(24.0, 24.0));
        p.setPen(makePen(iconSoft, 1.2));
        p.drawLine(QPointF(10.5, 13.2), QPointF(16.0, 13.2));
        p.drawLine(QPointF(13.25, 10.5), QPointF(13.25, 16.0));
        QFont scaleFont = p.font();
        scaleFont.setPointSizeF(8.0);
        scaleFont.setBold(true);
        p.setFont(scaleFont);
        p.setPen(makePen(iconInk, 1.0));
        p.drawText(QRectF(18.0, 6.5, 7.5, 9.5), Qt::AlignCenter, QStringLiteral("x"));
        break;
    }
    case PropertyIcon::Font: {
        p.setPen(makePen(iconInk, 1.7));
        p.drawLine(QPointF(8.0, 8.5), QPointF(24.0, 8.5));
        p.drawLine(QPointF(16.0, 8.5), QPointF(16.0, 24.0));
        p.drawLine(QPointF(12.0, 24.0), QPointF(20.0, 24.0));
        p.setPen(makePen(iconSoft, 1.3));
        p.drawLine(QPointF(9.5, 8.5), QPointF(9.5, 12.5));
        p.drawLine(QPointF(22.5, 8.5), QPointF(22.5, 12.5));
        break;
    }
    case PropertyIcon::EditText: {
        p.setPen(makePen(iconSoft, 1.3));
        p.drawRoundedRect(QRectF(7.5, 8.5, 13.5, 15.0), 2.2, 2.2);
        p.drawLine(QPointF(10.5, 13.0), QPointF(17.5, 13.0));
        p.drawLine(QPointF(10.5, 17.0), QPointF(15.5, 17.0));
        p.setPen(makePen(iconInk, 1.7));
        QPainterPath pencil;
        pencil.moveTo(16.0, 23.0);
        pencil.lineTo(24.6, 14.4);
        pencil.lineTo(26.5, 16.3);
        pencil.lineTo(17.9, 24.9);
        pencil.lineTo(15.2, 25.7);
        pencil.closeSubpath();
        p.drawPath(pencil);
        p.drawLine(QPointF(22.9, 12.7), QPointF(26.5, 16.3));
        break;
    }
    case PropertyIcon::ResetNumber: {
        QFont font = p.font();
        font.setPointSizeF(9.0);
        font.setBold(true);
        p.setFont(font);
        p.setPen(makePen(iconInk, 1.0));
        p.drawText(QRectF(6.5, 5.5, 10.0, 10.0), Qt::AlignCenter, QStringLiteral("1"));
        p.setPen(makePen(iconSoft, 1.4));
        p.drawArc(QRectF(8.0, 8.0, 16.0, 16.0), 35 * 16, 285 * 16);
        p.setPen(makePen(iconInk, 1.5));
        QPainterPath arrow;
        arrow.moveTo(21.5, 8.5);
        arrow.lineTo(24.5, 8.5);
        arrow.lineTo(23.8, 11.5);
        p.drawPath(arrow);
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
