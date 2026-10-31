#include "scroll/scroll_session_window_internal.h"

#include "annotation_launch.h"
#include "clipboard_image.h"
#include "debug_log.h"
#include "layer_shell_runtime.h"
#include "screen_capture.h"
#include "ui/i18n.h"
#include "ui/theme.h"
#include "windows_integration.h"

#include <QCloseEvent>
#include <QCoreApplication>
#include <QDateTime>
#ifdef MARK_SHOT_WITH_DBUS
#include <QDBusConnection>
#include <QDBusMessage>
#endif
#include <QDir>
#include <QEvent>
#include <QEventLoop>
#include <QFile>
#include <QFileDialog>
#include <QFont>
#include <QGuiApplication>
#include <QHBoxLayout>
#include <QIcon>
#include <QKeyEvent>
#include <QKeySequence>
#include <QMouseEvent>
#include <QPainter>
#include <QPainterPath>
#include <QPaintEvent>
#include <QPixmap>
#include <QPushButton>
#include <QRegion>
#include <QResizeEvent>
#include <QScreen>
#include <QShortcut>
#include <QSize>
#include <QShowEvent>
#include <QStandardPaths>
#include <QTimer>
#include <QVBoxLayout>
#include <QWheelEvent>
#include <QWindow>

#include <algorithm>
#include <cmath>
#include <cstdarg>
#include <cstdio>
#include <cstring>
#include <optional>
#include <utility>

namespace markshot::scroll {

std::uint8_t grayPixel(const QImage &frame, int x, int y)
{
    const QRgb px = frame.pixel(x, y);
    const int gray =
        static_cast<int>(0.299 * qRed(px) + 0.587 * qGreen(px) + 0.114 * qBlue(px));
    return static_cast<std::uint8_t>(gray);
}

// A coarse grayscale grid used to tell whether the latest capture is the same
// as the previous one (the user has not scrolled yet). Mirrors wayscrollshot's
// frame_signature. Duplicate frames are skipped so idle captures are not sent
// into the stitcher as real scroll movement.
/// @brief Generates a downsampled grayscale grid signature for a given frame.
/// @param frame The input image frame.
/// @param cols Number of columns in the grid.
/// @param rows Number of rows in the grid.
/// @return A vector representing the downsampled grayscale values.
QVector<std::uint8_t> frameSignature(const QImage &frame, int cols, int rows)
{
    QVector<std::uint8_t> signature;
    const int w = std::max(1, frame.width());
    const int h = std::max(1, frame.height());
    cols = std::max(1, cols);
    rows = std::max(1, rows);
    signature.reserve(cols * rows);

    for (int row = 0; row < rows; ++row) {
        const int y = std::min((row * h) / rows, h - 1);
        for (int col = 0; col < cols; ++col) {
            const int x = std::min((col * w) / cols, w - 1);
            signature.push_back(grayPixel(frame, x, y));
        }
    }
    return signature;
}

/// @brief Checks if two frame signatures are sufficiently similar to be considered duplicates.
/// @param previous The signature of the previous frame.
/// @param current The signature of the current frame.
/// @return True if the signatures are duplicates, false otherwise.
bool isDuplicateSignature(const QVector<std::uint8_t> &previous,
                          const QVector<std::uint8_t> &current)
{
    if (previous.size() != current.size() || previous.isEmpty()) {
        return false;
    }
    float sum = 0.0f;
    int maxDiff = 0;
    for (int i = 0; i < previous.size(); ++i) {
        const int diff = std::abs(static_cast<int>(previous[i]) - static_cast<int>(current[i]));
        maxDiff = std::max(maxDiff, diff);
        sum += static_cast<float>(diff);
    }
    const float avg = sum / static_cast<float>(previous.size());
    return avg <= kDuplicateAvgDiff && maxDiff <= kDuplicateMaxDiff;
}

QString scrollSavePath()
{
    QString pictures = QStandardPaths::writableLocation(QStandardPaths::PicturesLocation);
    if (pictures.isEmpty()) {
        pictures = QDir::homePath();
    }
    const QString filename =
        QStringLiteral("mark-shot-scroll-%1.png")
            .arg(QDateTime::currentDateTime().toString(QStringLiteral("yyyyMMdd-hhmmss")));
    return QDir(pictures).filePath(filename);
}

const char *algorithmDebugName()
{
    return "col-sample";
}

bool isWaylandPlatform()
{
    return QGuiApplication::platformName().compare(QStringLiteral("wayland"),
                                                   Qt::CaseInsensitive) == 0;
}

const char *axisDebugName(ScrollAxis axis)
{
    return axis == ScrollAxis::Horizontal ? "horizontal" : "vertical";
}

const char *statusDebugName(StitchStatus status)
{
    switch (status) {
    case StitchStatus::FirstFrame:
        return "first-frame";
    case StitchStatus::Appended:
        return "appended";
    case StitchStatus::NoProgress:
        return "no-progress";
    case StitchStatus::NoMatch:
        return "no-match";
    }
    return "unknown";
}

const char *edgeDebugName(StitchEdge edge)
{
    switch (edge) {
    case StitchEdge::Start:
        return "start";
    case StitchEdge::End:
        return "end";
    case StitchEdge::None:
        return "none";
    }
    return "unknown";
}

void logScrollDebug(const char *format, ...)
{
    if (!markshot::debugEnabled()) {
        return;
    }
    va_list args;
    va_start(args, format);
    markshot::debugLogV("session", format, args);
    va_end(args);
}

bool nearColor(QRgb pixel, int red, int green, int blue)
{
    const int dr = qRed(pixel) - red;
    const int dg = qGreen(pixel) - green;
    const int db = qBlue(pixel) - blue;
    return dr * dr + dg * dg + db * db
        <= kCaptureFrameArtifactDistance * kCaptureFrameArtifactDistance;
}

bool isCaptureFrameArtifactPixel(QRgb pixel)
{
    return nearColor(pixel, 45, 212, 191)
        || nearColor(pixel, 250, 204, 21);
}

bool rowLooksLikeCaptureFrameArtifact(const QImage &frame, int y)
{
    if (y < 0 || y >= frame.height() || frame.width() <= 0) {
        return false;
    }

    const auto *row = reinterpret_cast<const QRgb *>(frame.constScanLine(y));
    int hits = 0;
    for (int x = 0; x < frame.width(); ++x) {
        if (isCaptureFrameArtifactPixel(row[x])) {
            ++hits;
        }
    }
    return hits >= std::max(12, frame.width() * 35 / 100);
}

bool columnLooksLikeCaptureFrameArtifact(const QImage &frame, int x)
{
    if (x < 0 || x >= frame.width() || frame.height() <= 0) {
        return false;
    }

    int hits = 0;
    for (int y = 0; y < frame.height(); ++y) {
        const auto *row = reinterpret_cast<const QRgb *>(frame.constScanLine(y));
        if (isCaptureFrameArtifactPixel(row[x])) {
            ++hits;
        }
    }
    return hits >= std::max(12, frame.height() * 35 / 100);
}

std::optional<int> replacementRowForEdgeArtifact(const QImage &frame, bool topEdge)
{
    if (topEdge) {
        for (int y = std::min(kCaptureFrameArtifactScanPx, frame.height() - 1);
             y < frame.height();
             ++y) {
            if (!rowLooksLikeCaptureFrameArtifact(frame, y)) {
                return y;
            }
        }
        return std::nullopt;
    }

    for (int y = std::max(0, frame.height() - kCaptureFrameArtifactScanPx - 1);
         y >= 0;
         --y) {
        if (!rowLooksLikeCaptureFrameArtifact(frame, y)) {
            return y;
        }
    }
    return std::nullopt;
}

std::optional<int> replacementColumnForEdgeArtifact(const QImage &frame, bool leftEdge)
{
    if (leftEdge) {
        for (int x = std::min(kCaptureFrameArtifactScanPx, frame.width() - 1);
             x < frame.width();
             ++x) {
            if (!columnLooksLikeCaptureFrameArtifact(frame, x)) {
                return x;
            }
        }
        return std::nullopt;
    }

    for (int x = std::max(0, frame.width() - kCaptureFrameArtifactScanPx - 1);
         x >= 0;
         --x) {
        if (!columnLooksLikeCaptureFrameArtifact(frame, x)) {
            return x;
        }
    }
    return std::nullopt;
}

void copyImageRow(QImage *frame, int dstY, int srcY)
{
    if (!frame || dstY == srcY) {
        return;
    }
    std::memcpy(frame->scanLine(dstY), frame->constScanLine(srcY), frame->bytesPerLine());
}

void copyImageColumn(QImage *frame, int dstX, int srcX)
{
    if (!frame || dstX == srcX) {
        return;
    }
    for (int y = 0; y < frame->height(); ++y) {
        auto *row = reinterpret_cast<QRgb *>(frame->scanLine(y));
        row[dstX] = row[srcX];
    }
}

int scrubCaptureFrameArtifacts(QImage *frame)
{
    if (!frame || frame->isNull()) {
        return 0;
    }
    if (frame->format() != QImage::Format_ARGB32_Premultiplied
        && frame->format() != QImage::Format_ARGB32) {
        *frame = frame->convertToFormat(QImage::Format_ARGB32_Premultiplied);
    }

    int scrubbedEdges = 0;
    const int scanRows = std::min(kCaptureFrameArtifactScanPx, frame->height());
    if (const std::optional<int> replacement = replacementRowForEdgeArtifact(*frame, true)) {
        bool changed = false;
        for (int y = 0; y < scanRows; ++y) {
            if (rowLooksLikeCaptureFrameArtifact(*frame, y)) {
                copyImageRow(frame, y, *replacement);
                changed = true;
            }
        }
        scrubbedEdges += changed ? 1 : 0;
    }
    if (const std::optional<int> replacement = replacementRowForEdgeArtifact(*frame, false)) {
        bool changed = false;
        for (int y = std::max(0, frame->height() - scanRows); y < frame->height(); ++y) {
            if (rowLooksLikeCaptureFrameArtifact(*frame, y)) {
                copyImageRow(frame, y, *replacement);
                changed = true;
            }
        }
        scrubbedEdges += changed ? 1 : 0;
    }

    const int scanColumns = std::min(kCaptureFrameArtifactScanPx, frame->width());
    if (const std::optional<int> replacement = replacementColumnForEdgeArtifact(*frame, true)) {
        bool changed = false;
        for (int x = 0; x < scanColumns; ++x) {
            if (columnLooksLikeCaptureFrameArtifact(*frame, x)) {
                copyImageColumn(frame, x, *replacement);
                changed = true;
            }
        }
        scrubbedEdges += changed ? 1 : 0;
    }
    if (const std::optional<int> replacement = replacementColumnForEdgeArtifact(*frame, false)) {
        bool changed = false;
        for (int x = std::max(0, frame->width() - scanColumns); x < frame->width(); ++x) {
            if (columnLooksLikeCaptureFrameArtifact(*frame, x)) {
                copyImageColumn(frame, x, *replacement);
                changed = true;
            }
        }
        scrubbedEdges += changed ? 1 : 0;
    }

    return scrubbedEdges;
}

QPen iconPen(const QColor &color, qreal width)
{
    return QPen(color, width, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin);
}

QRect captureFrameOuterRect(QRect region, int frameGap)
{
    frameGap = std::max(0, frameGap);
    return region.adjusted(-(frameGap + kCaptureFrameWidth),
                           -(frameGap + kCaptureFrameWidth),
                           frameGap + kCaptureFrameWidth,
                           frameGap + kCaptureFrameWidth);
}

QRect captureFrameInnerRect(QRect region, int frameGap)
{
    frameGap = std::max(0, frameGap);
    return region.adjusted(-frameGap, -frameGap, frameGap, frameGap);
}

QSize previewPanelSizeForAnchor(const QRect &anchor, const QRect &bounds)
{
    const int minHeight = kControlBarHeight + kStatusHeight + kPanelPadding * 3 + 120;
    int panelHeight = std::max(minHeight, anchor.height());
    const int availableHeight = std::max(1, bounds.height() - kPanelMargin * 2);
    panelHeight = std::min(panelHeight, std::max(minHeight, availableHeight));
    return QSize(kPanelWidth, panelHeight);
}

PreviewPanelPlacement choosePreviewPanelPlacement(const QRect &anchor,
                                                  const QRect &bounds,
                                                  const QSize &panelSize,
                                                  int previewGap)
{
    if (bounds.isEmpty() || panelSize.isEmpty()) {
        return {{}, false};
    }

    const int gap = std::max(0, previewGap);
    const int minLeft = bounds.left() + kPanelMargin;
    const int maxLeft = std::max(minLeft, bounds.right() - panelSize.width() - kPanelMargin + 1);
    const int minTop = bounds.top() + kPanelMargin;
    const int maxTop = std::max(minTop, bounds.bottom() - panelSize.height() - kPanelMargin + 1);

    auto makeRect = [&](int left, int top) {
        return QRect(QPoint(std::clamp(left, minLeft, maxLeft),
                            std::clamp(top, minTop, maxTop)),
                     panelSize);
    };

    QVector<QRect> candidates;
    candidates.reserve(12);
    candidates.append(makeRect(anchor.right() + 1 + gap, anchor.top()));
    candidates.append(makeRect(anchor.left() - gap - panelSize.width(), anchor.top()));
    candidates.append(makeRect(anchor.left(), anchor.bottom() + 1 + gap));
    candidates.append(makeRect(anchor.left(), anchor.top() - gap - panelSize.height()));
    candidates.append(makeRect(anchor.right() + 1 + gap, anchor.bottom() - panelSize.height() + 1));
    candidates.append(makeRect(anchor.left() - gap - panelSize.width(),
                               anchor.bottom() - panelSize.height() + 1));
    candidates.append(makeRect(anchor.right() - panelSize.width() + 1,
                               anchor.bottom() + 1 + gap));
    candidates.append(makeRect(anchor.right() - panelSize.width() + 1,
                               anchor.top() - gap - panelSize.height()));
    candidates.append(makeRect(bounds.right() - panelSize.width() - kPanelMargin + 1,
                               bounds.top() + kPanelMargin));
    candidates.append(makeRect(bounds.left() + kPanelMargin, bounds.top() + kPanelMargin));
    candidates.append(makeRect(bounds.right() - panelSize.width() - kPanelMargin + 1,
                               bounds.bottom() - panelSize.height() - kPanelMargin + 1));
    candidates.append(makeRect(bounds.left() + kPanelMargin,
                               bounds.bottom() - panelSize.height() - kPanelMargin + 1));

    if (anchor.isEmpty()) {
        return {candidates.first(), true};
    }

    for (const QRect &candidate : std::as_const(candidates)) {
        if (!candidate.intersects(anchor)) {
            return {candidate, true};
        }
    }

    auto intersectionArea = [&](const QRect &candidate) {
        const QRect overlap = candidate.intersected(anchor);
        return overlap.isEmpty() ? 0 : overlap.width() * overlap.height();
    };
    const QRect *best = &candidates.first();
    for (const QRect &candidate : std::as_const(candidates)) {
        if (intersectionArea(candidate) < intersectionArea(*best)) {
            best = &candidate;
        }
    }
    return {*best, false};
}

QSize controlButtonSize()
{
    return QSize(kControlButtonWidth, kControlButtonHeight);
}

QRect chooseFloatingDragHandleRect(const QRect &anchor, const QRect &bounds)
{
    const QSize handleSize = controlButtonSize();
    if (bounds.isEmpty() || anchor.isEmpty() || handleSize.isEmpty()) {
        return {};
    }

    const int minLeft = bounds.left() + kPanelMargin;
    const int maxLeft = std::max(minLeft, bounds.right() - handleSize.width() - kPanelMargin + 1);
    const int minTop = bounds.top() + kPanelMargin;
    const int maxTop = std::max(minTop, bounds.bottom() - handleSize.height() - kPanelMargin + 1);

    const int left = std::clamp(anchor.left(), minLeft, maxLeft);
    const int bottomTop = anchor.bottom() + 1 + kFloatingDragHandleGap;
    const int topTop = anchor.top() - kFloatingDragHandleGap - handleSize.height();
    if (bottomTop <= maxTop) {
        return QRect(QPoint(left, bottomTop), handleSize);
    }
    if (topTop >= minTop) {
        return QRect(QPoint(left, topTop), handleSize);
    }

    const int sideLeft = anchor.left() - kFloatingDragHandleGap - handleSize.width();
    if (sideLeft >= minLeft) {
        const int bottomAlignedTop = anchor.bottom() - handleSize.height() + 1;
        const int topAlignedTop = anchor.top();
        if (bottomAlignedTop >= minTop && bottomAlignedTop <= maxTop) {
            return QRect(QPoint(sideLeft, bottomAlignedTop), handleSize);
        }
        if (topAlignedTop >= minTop && topAlignedTop <= maxTop) {
            return QRect(QPoint(sideLeft, topAlignedTop), handleSize);
        }
        return QRect(QPoint(sideLeft, std::clamp(topAlignedTop, minTop, maxTop)), handleSize);
    }

    const int cornerTop = bottomTop > maxTop ? anchor.top() : anchor.bottom() - handleSize.height() + 1;
    return QRect(QPoint(left, std::clamp(cornerTop, minTop, maxTop)), handleSize);
}

QIcon makeControlIcon(ControlIcon icon, const ui::ToolbarPalette &palette)
{
    constexpr int size = 32;
    const QColor ink = palette.text;
    QColor soft = ink;
    soft.setAlpha(130);
    QPixmap pixmap(size, size);
    pixmap.fill(Qt::transparent);

    QPainter p(&pixmap);
    p.setRenderHint(QPainter::Antialiasing, true);
    p.setBrush(Qt::NoBrush);
    p.setPen(iconPen(ink));

    switch (icon) {
    case ControlIcon::AxisVertical:
        p.drawLine(QPointF(16, 7), QPointF(16, 25));
        p.drawLine(QPointF(11, 12), QPointF(16, 7));
        p.drawLine(QPointF(21, 12), QPointF(16, 7));
        p.drawLine(QPointF(11, 20), QPointF(16, 25));
        p.drawLine(QPointF(21, 20), QPointF(16, 25));
        break;
    case ControlIcon::AxisHorizontal:
        p.drawLine(QPointF(7, 16), QPointF(25, 16));
        p.drawLine(QPointF(12, 11), QPointF(7, 16));
        p.drawLine(QPointF(12, 21), QPointF(7, 16));
        p.drawLine(QPointF(20, 11), QPointF(25, 16));
        p.drawLine(QPointF(20, 21), QPointF(25, 16));
        break;
    case ControlIcon::Pause:
        p.setPen(Qt::NoPen);
        p.setBrush(ink);
        p.drawRoundedRect(QRectF(10, 8, 4.5, 16), 1.2, 1.2);
        p.drawRoundedRect(QRectF(17.5, 8, 4.5, 16), 1.2, 1.2);
        break;
    case ControlIcon::Resume: {
        QPainterPath play;
        play.moveTo(11, 8);
        play.lineTo(24, 16);
        play.lineTo(11, 24);
        play.closeSubpath();
        p.setPen(iconPen(ink, 1.4));
        p.setBrush(palette.controlHoverFill);
        p.drawPath(play);
        break;
    }
    case ControlIcon::Annotate: {
        p.save();
        p.translate(16, 16);
        p.rotate(-45);
        p.setPen(iconPen(ink, 1.6));
        QPainterPath pen;
        pen.moveTo(-2.4, -10.5);
        pen.lineTo(2.4, -10.5);
        pen.lineTo(2.4, 3.0);
        pen.lineTo(0.0, 9.5);
        pen.lineTo(-2.4, 3.0);
        pen.closeSubpath();
        p.drawPath(pen);
        p.drawLine(QPointF(0.0, 3.0), QPointF(0.0, 9.5));
        p.restore();
        break;
    }
    case ControlIcon::Save:
        p.setPen(iconPen(ink, 1.6));
        p.drawRoundedRect(QRectF(7.5, 7.5, 17.0, 17.0), 2.0, 2.0);
        p.drawRoundedRect(QRectF(11.0, 7.5, 10.0, 5.0), 0.6, 0.6);
        p.drawRoundedRect(QRectF(10.0, 15.5, 12.0, 9.0), 0.9, 0.9);
        p.drawLine(QPointF(12.5, 18.5), QPointF(19.5, 18.5));
        break;
    case ControlIcon::Copy:
        p.setPen(iconPen(soft, 1.5));
        p.drawRoundedRect(QRectF(11.5, 7.5, 13, 14), 2.5, 2.5);
        p.setPen(iconPen(ink, 1.8));
        p.drawRoundedRect(QRectF(7.5, 11.5, 13, 14), 2.5, 2.5);
        break;
    case ControlIcon::Cancel:
        p.setPen(iconPen(ink, 1.8));
        p.drawLine(QPointF(10, 10), QPointF(22, 22));
        p.drawLine(QPointF(22, 10), QPointF(10, 22));
        break;
    }

    p.end();
    return QIcon(pixmap);
}

void configureIconButton(QPushButton *button, const QIcon &icon, const QString &label)
{
    button->setText(QString());
    button->setIcon(icon);
    button->setIconSize(QSize(21, 21));
    button->setToolTip(label);
    button->setAccessibleName(label);
}

void applyControlButtonChrome(QPushButton *button, const ui::ToolbarPalette &palette)
{
    button->setFixedSize(controlButtonSize());
    button->setFocusPolicy(Qt::NoFocus);
    button->setCursor(Qt::PointingHandCursor);
    button->setStyleSheet(QStringLiteral(
        "QPushButton { color: %1; background: %2; border: 1px solid %3; border-radius: 8px;"
        " padding: 0; min-width: 40px; max-width: 40px; min-height: 36px; max-height: 36px; }"
        "QPushButton:hover { background: %4; border-color: %5; }"
        "QPushButton[role=\"primary\"] { background: %6; border-color: %7; }"
        "QPushButton[role=\"primary\"]:hover { background: %4; border-color: %7; }"
        "QPushButton[role=\"danger\"]:hover { background: %8; border-color: %9; }"
        "QPushButton:disabled { color: %10; background: transparent; }")
        .arg(ui::cssColor(palette.text), ui::cssColor(palette.controlFill),
             ui::cssColor(palette.controlBorder), ui::cssColor(palette.softHoverFill),
             ui::cssColor(palette.softHoverBorder), ui::cssColor(palette.activeFill),
             ui::cssColor(palette.activeBorder), ui::cssColor(palette.dangerHoverFill),
             ui::cssColor(palette.dangerHoverBorder))
        .arg(ui::cssColor(palette.mutedText)));
}

}  // namespace markshot::scroll
