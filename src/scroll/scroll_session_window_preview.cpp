#include "scroll/scroll_session_window_internal.h"

namespace markshot::scroll {

void ScrollSessionWindow::drawPreviewContent(QPainter &painter, const QRect &area) const
{
    if (area.height() <= 10) {
        return;
    }

    painter.fillRect(area, QColor(8, 13, 19, 220));

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
    painter.setPen(QPen(QColor(255, 255, 255, 40), 1));
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
    painter.setPen(QPen(QColor(255, 255, 255, 40), 1));
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
                            makeControlIcon(m_paused ? ControlIcon::Resume : ControlIcon::Pause),
                            pauseLabel);
    }
    const bool horizontal = m_stitcher.axis() == ScrollAxis::Horizontal;
    const QIcon axisIcon = makeControlIcon(horizontal ? ControlIcon::AxisHorizontal
                                                     : ControlIcon::AxisVertical);
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

    if (m_uiConfig.frameEnabled && !m_panelOnlyWindow) {
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
        QPainterPath panelPath;
        panelPath.addRoundedRect(panel, 12, 12);
        painter.fillPath(panelPath, QColor(15, 17, 23, 242));
        painter.setPen(QPen(QColor(255, 255, 255, 28), 1));
        painter.drawPath(panelPath);

        // Status text along the top of the panel.
        const QRect statusRect(panel.left() + kPanelPadding,
                               panel.top() + kPanelPadding,
                               panel.width() - kPanelPadding * 2,
                               kStatusHeight);
        painter.setPen(QColor(204, 251, 241, 240));
        QFont statusFont = painter.font();
        statusFont.setPointSize(10);
        statusFont.setBold(true);
        painter.setFont(statusFont);
        painter.drawText(statusRect, Qt::AlignVCenter | Qt::AlignLeft,
                         m_statusText.isEmpty() ? MS_TR("Scroll down to capture") : m_statusText);

        // Preview image area: between the status row and the control bar.
        const QRect imageArea = imageAreaRect();
        drawPreviewContent(painter, imageArea);
    }

    drawFloatingDragHandle(painter);

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

}  // namespace markshot::scroll
