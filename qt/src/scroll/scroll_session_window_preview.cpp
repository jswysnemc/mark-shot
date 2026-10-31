#include "scroll/scroll_session_window_internal.h"
#include "scroll/scroll_preview_chrome.h"

// ===== 新增 import =====
#include <QFontMetrics>
#include <QLineF>

namespace markshot::scroll {

void ScrollSessionWindow::drawPreviewContent(QPainter &painter, const QRect &area) const
{
    if (area.height() <= 10) {
        return;
    }

    drawScrollPreviewSurface(painter, area, false, m_uiConfig.palette);

    const PreviewLayout layout = computePreviewLayout(area);
    if (!layout.valid) {
        return;
    }

    const QImage result = currentResult();
    const bool horizontal = m_stitcher.axis() == ScrollAxis::Horizontal;
    auto drawCaptureMarker = [&](const QRect &target,
                                 int sourceStart,
                                 int sourceLen,
                                 bool fillInterior) {
        if (m_captureLen <= 0 || sourceLen <= 0 || target.isEmpty()) {
            return;
        }

        const int sourceEnd = sourceStart + sourceLen;
        const int captureStart = std::max(m_capturePos, sourceStart);
        const int captureEnd = std::min(m_capturePos + m_captureLen, sourceEnd);
        if (captureEnd <= captureStart) {
            return;
        }

        QRect marker;
        if (horizontal) {
            const qreal scale = static_cast<qreal>(target.width()) / sourceLen;
            const int x = target.left()
                + static_cast<int>(std::lround((captureStart - sourceStart) * scale));
            const int w = std::max(3, static_cast<int>(
                                          std::lround((captureEnd - captureStart) * scale)));
            marker = QRect(x, target.top(), w, target.height());
        } else {
            const qreal scale = static_cast<qreal>(target.height()) / sourceLen;
            const int y = target.top()
                + static_cast<int>(std::lround((captureStart - sourceStart) * scale));
            const int h = std::max(3, static_cast<int>(
                                          std::lround((captureEnd - captureStart) * scale)));
            marker = QRect(target.left(), y, target.width(), h);
        }

        painter.setPen(QPen(QColor(250, 204, 21, 245), 2));
        painter.setBrush(fillInterior ? QColor(250, 204, 21, 42) : Qt::NoBrush);
        painter.drawRect(marker.intersected(target));
    };

    const QRect &detailRect = layout.detailRect;
    const int pos = std::clamp(m_scrubPos, 0, layout.maxScrub);
    QRect srcRect;
    QRect detailTarget;
    if (horizontal) {
        const int srcW = std::min(layout.viewportLen, result.width());
        srcRect = QRect(pos, 0, srcW, result.height());
        const int targetW = std::min(detailRect.width(),
            std::max(1, static_cast<int>(std::lround(srcW * layout.detailScale))));
        detailTarget = QRect(detailRect.left(), detailRect.top(), targetW, detailRect.height());
    } else {
        const int srcH = std::min(layout.viewportLen, result.height());
        srcRect = QRect(0, pos, result.width(), srcH);
        const int targetH = std::min(detailRect.height(),
            std::max(1, static_cast<int>(std::lround(srcH * layout.detailScale))));
        detailTarget = QRect(detailRect.left(), detailRect.top(), detailRect.width(), targetH);
    }
    painter.drawImage(detailTarget, result, srcRect);
    painter.setPen(QPen(m_uiConfig.palette.separator, 1));
    painter.setBrush(Qt::NoBrush);
    painter.drawRect(detailTarget);

    if (layout.globalRect.isEmpty()) {
        return;
    }

    const QRect overviewTarget = overviewTargetRect(layout, result);
    if (overviewTarget.isEmpty()) {
        return;
    }
    painter.drawImage(overviewTarget, result);
    painter.setPen(QPen(m_uiConfig.palette.separator, 1));
    painter.setBrush(Qt::NoBrush);
    painter.drawRect(overviewTarget);

    drawCaptureMarker(overviewTarget,
                      0,
                      horizontal ? result.width() : result.height(),
                      true);
    const QRect marker = overviewViewportRect(overviewTarget, layout);
    painter.setPen(QPen(QColor(45, 212, 191, m_overviewDragging ? 255 : 235), 2));
    painter.setBrush(QColor(45, 212, 191, m_overviewDragging ? 70 : 45));
    painter.drawRect(marker.intersected(overviewTarget));
}

void ScrollSessionWindow::syncPreviewScroll(const StitchResult &outcome)
{
    const PreviewLayout layout = computePreviewLayout();
    const int maxScrub = layout.maxScrub;
    const bool stitchedNewContent =
        outcome.status == StitchStatus::Appended && outcome.added > 0;

    if (stitchedNewContent) {
        // Manual overview scrubbing is only for browsing the current stitched image.
        // Once the image grows again, snap the detail view back to the live capture.
        m_following = true;
    }

    // Prepended content (reverse scroll) is inserted ahead of the current view,
    // so everything already on screen shifts by outcome.added; nudge the viewed
    // position to keep the user looking at the same content instead of jumping.
    if (!m_following && outcome.edge == StitchEdge::Start && outcome.added > 0) {
        m_scrubPos += outcome.added;
    }
    if (m_following) {
        if (outcome.frameLength > 0) {
            m_scrubPos = outcome.position;
            if (outcome.edge == StitchEdge::End) {
                m_scrubPos = outcome.position + outcome.frameLength - layout.viewportLen;
            }
        } else if (outcome.edge == StitchEdge::Start) {
            m_scrubPos = 0;
        } else if (outcome.edge == StitchEdge::End && outcome.added > 0) {
            m_scrubPos = maxScrub;
        }
    }
    m_scrubPos = std::clamp(m_scrubPos, 0, maxScrub);
    if (maxScrub == 0) {
        m_following = true;
    }
}

QRect ScrollSessionWindow::overviewTargetRect(const PreviewLayout &layout,
                                              const QImage &result) const
{
    if (layout.globalRect.isEmpty() || result.isNull()) {
        return {};
    }

    const QSize fitted = result.size().scaled(layout.globalRect.size(), Qt::KeepAspectRatio);
    if (fitted.isEmpty()) {
        return {};
    }

    return QRect(layout.globalRect.left() + (layout.globalRect.width() - fitted.width()) / 2,
                 layout.globalRect.top() + (layout.globalRect.height() - fitted.height()) / 2,
                 fitted.width(),
                 fitted.height());
}

QRect ScrollSessionWindow::overviewViewportRect(const QRect &target,
                                                const PreviewLayout &layout) const
{
    if (target.isEmpty() || layout.longLen <= 0 || layout.viewportLen <= 0) {
        return {};
    }

    const bool horizontal = m_stitcher.axis() == ScrollAxis::Horizontal;
    const int axisStart = horizontal ? target.left() : target.top();
    const int axisLen = horizontal ? target.width() : target.height();
    if (axisLen <= 0) {
        return {};
    }

    const int pos = std::clamp(m_scrubPos, 0, layout.maxScrub);
    const qreal scale = static_cast<qreal>(axisLen) / layout.longLen;
    const int markerLen = std::clamp(
        static_cast<int>(std::lround(layout.viewportLen * scale)),
        2,
        axisLen);
    const int markerStart = std::clamp(
        axisStart + static_cast<int>(std::lround(pos * scale)),
        axisStart,
        axisStart + axisLen - markerLen);

    if (horizontal) {
        return QRect(markerStart, target.top(), markerLen, target.height());
    }
    return QRect(target.left(), markerStart, target.width(), markerLen);
}

int ScrollSessionWindow::scrubPosFromOverviewPoint(const QPoint &point,
                                                   const QRect &target,
                                                   const PreviewLayout &layout,
                                                   int markerOffsetPx) const
{
    if (target.isEmpty() || layout.maxScrub <= 0 || layout.longLen <= 0) {
        return 0;
    }

    const bool horizontal = m_stitcher.axis() == ScrollAxis::Horizontal;
    const int axisPoint = horizontal ? point.x() : point.y();
    const int axisStart = horizontal ? target.left() : target.top();
    const int axisLen = horizontal ? target.width() : target.height();
    if (axisLen <= 0) {
        return 0;
    }

    const qreal scale = static_cast<qreal>(axisLen) / layout.longLen;
    const int pos = static_cast<int>(
        std::lround((axisPoint - axisStart - markerOffsetPx) / scale));
    return std::clamp(pos, 0, layout.maxScrub);
}

void ScrollSessionWindow::setScrubPosition(int pos, bool followAtEnd)
{
    const PreviewLayout layout = computePreviewLayout();
    const int maxScrub = layout.maxScrub;
    m_scrubPos = std::clamp(pos, 0, maxScrub);
    m_following = maxScrub == 0 || (followAtEnd && m_scrubPos >= maxScrub);
    update();
}

bool ScrollSessionWindow::beginOverviewDrag(const QPoint &point)
{
    const QImage result = currentResult();
    const PreviewLayout layout = computePreviewLayout();
    if (!layout.valid || layout.maxScrub <= 0) {
        return false;
    }

    const QRect target = overviewTargetRect(layout, result);
    if (!target.contains(point)) {
        return false;
    }

    const bool horizontal = m_stitcher.axis() == ScrollAxis::Horizontal;
    const QRect marker = overviewViewportRect(target, layout);
    const QRect hitRect = marker.adjusted(-6, -6, 6, 6);
    if (hitRect.contains(point)) {
        m_overviewDragOffsetPx = horizontal ? point.x() - marker.left()
                                            : point.y() - marker.top();
    } else {
        const int markerLen = horizontal ? marker.width() : marker.height();
        m_overviewDragOffsetPx = markerLen / 2;
    }

    m_overviewDragging = true;
    m_following = false;
    setCursor(Qt::ClosedHandCursor);
    updateOverviewDrag(point);
    return true;
}

void ScrollSessionWindow::updateOverviewDrag(const QPoint &point)
{
    const QImage result = currentResult();
    const PreviewLayout layout = computePreviewLayout();
    if (!layout.valid || layout.maxScrub <= 0) {
        return;
    }

    const QRect target = overviewTargetRect(layout, result);
    const int pos = scrubPosFromOverviewPoint(point, target, layout, m_overviewDragOffsetPx);
    setScrubPosition(pos, false);
}

void ScrollSessionWindow::refreshControlLabels()
{
    if (m_pauseButton) {
        const QString pauseLabel = m_autoPausedForPreview
            ? MS_TR("Continue Capture")
            : (m_paused ? MS_TR("Resume") : MS_TR("Pause"));
        configureIconButton(m_pauseButton,
                            makeControlIcon(m_paused ? ControlIcon::Resume : ControlIcon::Pause, m_uiConfig.palette),
                            pauseLabel);
    }
    const bool horizontal = m_stitcher.axis() == ScrollAxis::Horizontal;
    const QIcon axisIcon = makeControlIcon(horizontal ? ControlIcon::AxisHorizontal
                                                     : ControlIcon::AxisVertical, m_uiConfig.palette);
    const QString axisLabel = horizontal ? MS_TR("Dir: Horizontal") : MS_TR("Dir: Vertical");
    const QString dragLabel = horizontal ? MS_TR("Drag region horizontally")
                                         : MS_TR("Drag region vertically");
    const bool largeRangeMode =
        shouldAvoidPreviewOverlapForCapture() && !previewPanelFitsAvailableSpace();
    if (m_axisButton) {
        configureIconButton(m_axisButton,
                            axisIcon,
                            m_stitcher.axisLocked() ? dragLabel : axisLabel);
        m_axisButton->setEnabled(!largeRangeMode);
    }
    if (m_floatingAxisButton) {
        configureIconButton(m_floatingAxisButton,
                            axisIcon,
                            m_stitcher.axisLocked() ? dragLabel : axisLabel);
        m_floatingAxisButton->setEnabled(true);
    }
}

QImage ScrollSessionWindow::currentResult() const
{
    return m_stitcher.fullImage();
}

QString ScrollSessionWindow::stitchedLengthLabel() const
{
    const int length = m_stitcher.stats().totalHeight;
    if (length <= 0) {
        return {};
    }
    return QStringLiteral("%1 px").arg(length);
}

void ScrollSessionWindow::annotateResult()
{
    const QImage result = currentResult();
    if (m_timer) {
        m_timer->stop();
    }
    if (result.isNull()) {
        close();
        return;
    }
    markshot::openImageForAnnotation(result, QStringLiteral("scroll-capture.png"));
    close();
}

void ScrollSessionWindow::saveResult()
{
    const QImage result = currentResult();
    if (result.isNull()) {
        return;
    }
    m_paused = true;
    m_autoPausedForPreview = false;
    cancelScrollIdlePause();
    updatePreviewPanelVisibility();
    refreshControlLabels();

    auto *dialog = new QFileDialog(nullptr, MS_TR("Save Scrolling Screenshot"));
    dialog->setAttribute(Qt::WA_DeleteOnClose);
    dialog->setAcceptMode(QFileDialog::AcceptSave);
    dialog->setFileMode(QFileDialog::AnyFile);
    dialog->setNameFilter(MS_TR("PNG Images (*.png)"));
    dialog->setDefaultSuffix(QStringLiteral("png"));
    dialog->setOption(QFileDialog::DontUseNativeDialog, true);
    dialog->selectFile(scrollSavePath());

    connect(dialog, &QFileDialog::accepted, this, [this, dialog, result] {
        const QStringList files = dialog->selectedFiles();
        if (!files.isEmpty() && result.save(files.first(), "PNG")) {
            close();
            return;
        }
        m_paused = false;
        updatePreviewPanelVisibility();
        refreshControlLabels();
    });
    connect(dialog, &QFileDialog::rejected, this, [this] {
        m_paused = false;
        updatePreviewPanelVisibility();
        refreshControlLabels();
    });
    dialog->open();
}

void ScrollSessionWindow::copyResult()
{
    const QImage result = currentResult();
    if (result.isNull()) {
        return;
    }

    markshot::copyImageToClipboard(result);
    close();
}

void ScrollSessionWindow::paintEvent(QPaintEvent *)
{
    QPainter painter(this);
    painter.setCompositionMode(QPainter::CompositionMode_Source);
    painter.fillRect(rect(), Qt::transparent);
    painter.setCompositionMode(QPainter::CompositionMode_SourceOver);
    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.setRenderHint(QPainter::SmoothPixmapTransform, true);

    if (m_gnomeShellPreview) {
        drawFloatingDragHandle(painter);
        return;
    }

    if (m_panelTransparentForCapture) {
        return;
    }

    if (m_uiConfig.frameEnabled && !m_panelOnlyWindow && !captureFrameSuppressedForHandle()) {
        const QRect region = regionLocalRect();
#if defined(Q_OS_WIN)
        painter.setPen(QPen(QColor(94, 234, 212), 2.0));
        painter.setBrush(Qt::NoBrush);
        painter.drawRoundedRect(QRectF(region).adjusted(-1.0, -1.0, 1.0, 1.0), 3.0, 3.0);
#else
        QPainterPath framePath;
        framePath.setFillRule(Qt::OddEvenFill);
        framePath.addRect(QRectF(captureFrameOuterRect(region, m_uiConfig.frameGap)));
        framePath.addRect(QRectF(captureFrameInnerRect(region, m_uiConfig.frameGap)));

        const QColor frameColor = m_paused
            ? QColor(250, 204, 21, 235)
            : QColor(45, 212, 191, 255);
        painter.setPen(Qt::NoPen);
        painter.setBrush(frameColor);
        painter.drawPath(framePath);
#endif
    }

    if (m_previewPanelVisible) {
        const QRect panel = previewPanelRect();
        drawScrollPreviewSurface(painter, panel, true, m_uiConfig.palette);

        // 1. 【滚动截图】【预览状态】状态行与工具栏共用配色，仅绘制静态状态点和文字
        const QRect statusRect(panel.left() + kPanelPadding,
                               panel.top() + kPanelPadding,
                               panel.width() - kPanelPadding * 2,
                               kStatusHeight);
        drawScrollPreviewStatus(painter, statusRect,
            m_statusText.isEmpty() ? MS_TR("Scroll down to capture") : m_statusText,
            stitchedLengthLabel(), m_uiConfig.palette);

        // Preview image area: between the status row and the control bar.
        const QRect imageArea = imageAreaRect();
        drawPreviewContent(painter, imageArea);
    }

    drawFloatingDragHandle(painter);
    drawHandleAvoidanceChromeIfEnabled(painter);

    if (m_restoreMaskAfterPaint && !m_axisDragging) {
        m_restoreMaskAfterPaint = false;
        QTimer::singleShot(0, this, [this] {
            if (m_axisDragging) {
                return;
            }
            m_transientPaintMask = {};
            updateInputMask();
        });
    }
}

namespace {

constexpr bool FEATURE_HANDLE_FRAME_AVOIDANCE_ENABLED = true;
constexpr qreal kAvoidanceGap = 8.0;
constexpr qreal kAvoidanceArm = 18.0;

QFont avoidanceGuideFont()
{
    QFont font;
    font.setPixelSize(12);
    font.setWeight(QFont::DemiBold);
    return font;
}

// 线宽与原来的捕获线框相同，笔画只向选区外侧加粗
QRectF avoidanceMarkBand(const QLineF &line, const QRect &region, qreal thickness)
{
    const QRectF box = QRectF(line.p1(), line.p2()).normalized();
    const bool horizontal = std::abs(line.x2() - line.x1()) >= std::abs(line.y2() - line.y1());
    if (horizontal) {
        const qreal y = box.center().y() < region.top() ? box.top() - thickness : box.top();
        return QRectF(box.left(), y, std::max(box.width(), thickness), thickness);
    }
    const qreal x = box.center().x() < region.left() ? box.left() - thickness : box.left();
    return QRectF(x, box.top(), thickness, std::max(box.height(), thickness));
}

bool avoidanceMarkFits(const QLineF &line,
                       const QRect &region,
                       const QRect &panel,
                       const QRect &guide,
                       const QRect &bounds,
                       qreal thickness)
{
    const QRectF box = avoidanceMarkBand(line, region, thickness);
    if (box.intersects(QRectF(region)) || !QRectF(bounds).intersects(box)) {
        return false;
    }
    if (!panel.isEmpty() && box.intersects(QRectF(panel))) {
        return false;
    }
    if (!guide.isEmpty() && box.intersects(QRectF(guide))) {
        return false;
    }
    return true;
}

QRect avoidanceGuideRect(const QRect &region, const QRect &bounds, const QRect &panel, int outsideGap)
{
    if (region.isEmpty() || bounds.isEmpty()) {
        return {};
    }

    const QFont font = avoidanceGuideFont();
    const QString text = MS_TR("Drag the button");
    const int width = 10 + 14 + 6 + QFontMetrics(font).horizontalAdvance(text) + 10;
    const int height = 26;
    const int minLeft = bounds.left() + 4;
    const int maxLeft = bounds.right() - width - 3;
    if (maxLeft < minLeft) {
        return {};
    }

    const QRect guide(std::clamp(region.center().x() - width / 2, minLeft, maxLeft),
                      region.top() - outsideGap - height,
                      width,
                      height);
    if (!bounds.contains(guide) || guide.intersects(region)) {
        return {};
    }
    if (!panel.isEmpty() && guide.intersects(panel)) {
        return {};
    }
    return guide;
}

void appendOutwardArm(QVector<QLineF> *lines,
                      const QPointF &vertex,
                      const QPointF &outward,
                      const QRect &region,
                      const QRect &panel,
                      const QRect &guide,
                      const QRect &bounds,
                      qreal thickness)
{
    // 1. 从顶点向外缩短，直到笔画完全落在捕获矩形外面
    for (qreal length = kAvoidanceArm; length >= 4.0; length -= 1.0) {
        const QLineF line(vertex, vertex + outward * length);
        if (avoidanceMarkFits(line, region, panel, guide, bounds, thickness)) {
            lines->append(line);
            return;
        }
    }
}

QVector<QLineF> avoidanceMarkLines(const QRect &region,
                                   const QRect &panel,
                                   const QRect &guide,
                                   const QRect &bounds,
                                   qreal thickness)
{
    QVector<QLineF> lines;
    if (region.isEmpty()) {
        return lines;
    }

    // 2. 顶点在选区角的外侧，两臂继续向外，形成 J L / 7 |-
    const qreal top = region.top();
    const qreal bottom = region.bottom() + 1.0;
    const qreal left = region.left();
    const qreal right = region.right() + 1.0;
    const QPointF corners[] = {
        QPointF(left - kAvoidanceGap, top - kAvoidanceGap),
        QPointF(right + kAvoidanceGap, top - kAvoidanceGap),
        QPointF(left - kAvoidanceGap, bottom + kAvoidanceGap),
        QPointF(right + kAvoidanceGap, bottom + kAvoidanceGap),
    };
    const QPointF outwardX[] = {QPointF(-1.0, 0.0), QPointF(1.0, 0.0), QPointF(-1.0, 0.0), QPointF(1.0, 0.0)};
    const QPointF outwardY[] = {QPointF(0.0, -1.0), QPointF(0.0, -1.0), QPointF(0.0, 1.0), QPointF(0.0, 1.0)};
    for (int i = 0; i < 4; ++i) {
        appendOutwardArm(&lines, corners[i], outwardX[i], region, panel, guide, bounds, thickness);
        appendOutwardArm(&lines, corners[i], outwardY[i], region, panel, guide, bounds, thickness);
    }
    return lines;
}

bool dragHandleButtonContains(const QWidget *host, const QPushButton *button, const QPoint &localPoint)
{
    if (!host || !button || !button->isVisible()) {
        return false;
    }
    const QRect mapped(button->mapTo(host, QPoint(0, 0)), button->size());
    return mapped.adjusted(-2, -2, 2, 2).contains(localPoint);
}

}  // namespace

bool ScrollSessionWindow::captureFrameSuppressedForHandle() const
{
    if (!FEATURE_HANDLE_FRAME_AVOIDANCE_ENABLED) {
        return false;
    }
    return m_dragHandleHovered || m_axisDragArmed || m_axisDragging;
}

void ScrollSessionWindow::syncDragHandleHoverIfEnabled(const QPoint &localPoint)
{
    if (!FEATURE_HANDLE_FRAME_AVOIDANCE_ENABLED) {
        return;
    }

    // 1. 预览栏方向按钮和浮动握柄都是拖动选区的把手
    const bool hovered = dragHandleButtonContains(this, m_axisButton, localPoint)
        || dragHandleButtonContains(this, m_floatingAxisButton, localPoint)
        || (floatingDragHandleActive() && floatingDragHandleLocalRect().contains(localPoint));
    if (hovered == m_dragHandleHovered) {
        return;
    }

    // 2. 先保住旧标记仍在遮罩内，清掉外框后再收缩遮罩
    const QRegion dirty = handleAvoidancePaintRegion() + framePaintRegion();
    m_dragHandleHovered = hovered;
    const QRegion next = handleAvoidancePaintRegion() + framePaintRegion();
    m_transientPaintMask += dirty + next;
    m_restoreMaskAfterPaint = true;
    updateInputMask();
    repaint(dirty + next);
}

QRegion ScrollSessionWindow::handleAvoidancePaintRegion() const
{
    if (!FEATURE_HANDLE_FRAME_AVOIDANCE_ENABLED || m_panelOnlyWindow || m_gnomeShellPreview) {
        return {};
    }

    const QRect bounds(QPoint(0, 0), size());
    const QRect region = regionLocalRect();
    const QRect panel = m_previewPanelVisible ? previewPanelRect() : QRect();
    const int outsideGap = std::max(10, m_uiConfig.frameGap + kCaptureFrameWidth + 4);
    const bool suppressed = captureFrameSuppressedForHandle();
    const QRect guide = suppressed ? avoidanceGuideRect(region, bounds, panel, outsideGap) : QRect();

    QRegion painted;
    if (!guide.isEmpty()) {
        painted += QRegion(guide.adjusted(-kAntialiasPad,
                                          -kAntialiasPad,
                                          kAntialiasPad,
                                          kAntialiasPad));
    }
    // 外翻折角只在外框收起时出现，占位留在遮罩里，避免清屏被裁掉
    const qreal thickness = kCaptureFrameWidth;
    for (const QLineF &line : avoidanceMarkLines(region, panel, guide, bounds, thickness)) {
        painted += QRegion(avoidanceMarkBand(line, region, thickness).toAlignedRect());
    }
    return painted.intersected(QRegion(bounds));
}

void ScrollSessionWindow::drawHandleAvoidanceChromeIfEnabled(QPainter &painter) const
{
    if (!FEATURE_HANDLE_FRAME_AVOIDANCE_ENABLED || m_panelOnlyWindow || m_gnomeShellPreview
        || m_panelTransparentForCapture) {
        return;
    }

    const QRect bounds(QPoint(0, 0), size());
    const QRect region = regionLocalRect();
    const QRect panel = m_previewPanelVisible ? previewPanelRect() : QRect();
    const int outsideGap = std::max(10, m_uiConfig.frameGap + kCaptureFrameWidth + 4);
    const bool suppressed = captureFrameSuppressedForHandle();
    const QRect guide = suppressed ? avoidanceGuideRect(region, bounds, panel, outsideGap) : QRect();
    const qreal thickness = kCaptureFrameWidth;

    painter.save();

    if (!suppressed) {
        painter.restore();
        return;
    }

    // 1. 【滚动截图】【按钮引导】外框收起后，才在选区上方提示拖动按钮
    if (!guide.isEmpty()) {
        QPainterPath path;
        path.addRoundedRect(QRectF(guide), 8.0, 8.0);
        painter.setPen(QPen(m_uiConfig.palette.activeBorder, 1.0));
        painter.setBrush(m_uiConfig.palette.popupFill);
        painter.drawPath(path);

        painter.setPen(QPen(QColor(94, 234, 212), 1.6, Qt::SolidLine, Qt::RoundCap));
        const int gripLeft = guide.left() + 11;
        const int gripTop = guide.top() + 7;
        const int gripBottom = guide.bottom() - 7;
        painter.drawLine(gripLeft, gripTop, gripLeft, gripBottom);
        painter.drawLine(gripLeft + 5, gripTop, gripLeft + 5, gripBottom);

        painter.setPen(m_uiConfig.palette.text);
        painter.setFont(avoidanceGuideFont());
        painter.drawText(guide.adjusted(28, 0, -8, 0), Qt::AlignVCenter | Qt::AlignLeft,
                         MS_TR("Drag the button"));
    }

    // 2. 【滚动截图】【范围标记】折角向外翻，笔画停在捕获矩形外面
    painter.setPen(Qt::NoPen);
    painter.setBrush(m_paused ? QColor(250, 204, 21, 235) : QColor(45, 212, 191, 255));
    for (const QLineF &line : avoidanceMarkLines(region, panel, guide, bounds, thickness)) {
        painter.drawRect(avoidanceMarkBand(line, region, thickness));
    }

    painter.restore();
}

}  // namespace markshot::scroll
