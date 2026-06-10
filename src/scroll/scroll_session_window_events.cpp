#include "scroll/scroll_session_window_internal.h"

namespace markshot::scroll {

void ScrollSessionWindow::keyPressEvent(QKeyEvent *event)
{
    if (event->key() == Qt::Key_Escape) {
        close();
        event->accept();
        return;
    }
    if (event->key() == Qt::Key_Return || event->key() == Qt::Key_Enter) {
        annotateResult();
        event->accept();
        return;
    }
    QWidget::keyPressEvent(event);
}

void ScrollSessionWindow::mousePressEvent(QMouseEvent *event)
{
    if (event->button() == Qt::LeftButton && floatingDragHandleActive()
        && floatingDragHandleLocalRect().contains(event->position().toPoint())) {
        armAxisDrag(event->globalPosition().toPoint());
        event->accept();
        return;
    }
    if (event->button() == Qt::LeftButton && beginOverviewDrag(event->pos())) {
        event->accept();
        return;
    }
    QWidget::mousePressEvent(event);
}

void ScrollSessionWindow::mouseMoveEvent(QMouseEvent *event)
{
    if (m_axisDragArmed && (event->buttons() & Qt::LeftButton)) {
        updateAxisDrag(event->globalPosition().toPoint());
        event->accept();
        return;
    }
    if (m_overviewDragging) {
        updateOverviewDrag(event->pos());
        event->accept();
        return;
    }
    QWidget::mouseMoveEvent(event);
}

void ScrollSessionWindow::mouseReleaseEvent(QMouseEvent *event)
{
    if (event->button() == Qt::LeftButton && m_axisDragArmed) {
        const bool wasDragging = finishAxisDrag();
        if (!wasDragging) {
            toggleAxis();
        }
        event->accept();
        return;
    }
    if (event->button() == Qt::LeftButton && m_overviewDragging) {
        updateOverviewDrag(event->pos());
        m_overviewDragging = false;
        const PreviewLayout layout = computePreviewLayout();
        if (m_scrubPos >= layout.maxScrub) {
            m_following = true;
        }
        unsetCursor();
        update();
        event->accept();
        return;
    }
    QWidget::mouseReleaseEvent(event);
}

void ScrollSessionWindow::wheelEvent(QWheelEvent *event)
{
    if (!m_previewPanelVisible || !previewPanelRect().contains(event->position().toPoint())) {
        QWidget::wheelEvent(event);
        return;
    }

    const PreviewLayout layout = computePreviewLayout();
    if (!layout.valid || layout.maxScrub <= 0) {
        event->accept();
        return;
    }

    const bool horizontal = m_stitcher.axis() == ScrollAxis::Horizontal;
    const QPoint pixelDelta = event->pixelDelta();
    int delta = horizontal && pixelDelta.x() != 0 ? pixelDelta.x() : pixelDelta.y();
    if (delta == 0) {
        const QPoint angleDelta = event->angleDelta();
        const int angle = horizontal && angleDelta.x() != 0 ? angleDelta.x() : angleDelta.y();
        if (angle == 0) {
            event->accept();
            return;
        }
        const int step = std::max(24, layout.viewportLen / 8);
        delta = static_cast<int>(std::lround(static_cast<qreal>(angle) / 120.0 * step));
    }

    setScrubPosition(m_scrubPos - delta, true);
    event->accept();
}

bool ScrollSessionWindow::eventFilter(QObject *watched, QEvent *event)
{
    if (watched != m_axisButton && watched != m_floatingAxisButton) {
        return QWidget::eventFilter(watched, event);
    }

    const bool mouseEvent = event->type() == QEvent::MouseButtonPress
        || event->type() == QEvent::MouseMove
        || event->type() == QEvent::MouseButtonRelease;
    if (watched == m_axisButton && m_axisButton && !m_axisButton->isEnabled() && mouseEvent) {
        m_axisDragArmed = false;
        m_axisDragging = false;
        return true;
    }

    switch (event->type()) {
    case QEvent::MouseButtonPress: {
        auto *me = static_cast<QMouseEvent *>(event);
        if (me->button() == Qt::LeftButton) {
            armAxisDrag(me->globalPosition().toPoint());
        }
        // Let the button see the press so it can still emit clicked on release.
        return false;
    }

    case QEvent::MouseMove: {
        auto *me = static_cast<QMouseEvent *>(event);
        if (!m_axisDragArmed || !(me->buttons() & Qt::LeftButton)) {
            return false;
        }

        if (!updateAxisDrag(me->globalPosition().toPoint())) {
            return false;
        }
        return true;  // consume the move so the button doesn't highlight
    }

    case QEvent::MouseButtonRelease: {
        auto *me = static_cast<QMouseEvent *>(event);
        if (me->button() != Qt::LeftButton || !m_axisDragArmed) {
            return false;
        }

        if (!finishAxisDrag()) {
            // Treat as a plain click: let the button fire its clicked signal.
            return false;
        }

        // Consume the release so the button does NOT emit clicked (which
        // would toggle the axis).
        return true;
    }

    default:
        return QWidget::eventFilter(watched, event);
    }
}

bool ScrollSessionWindow::gnomeShellPreviewActive() const
{
    return m_gnomeShellPreview;
}

QSize ScrollSessionWindow::gnomePreviewImageSize() const
{
    const QRect bounds = captureBoundsGlobal();
    const QRect anchor = previewAnchorGlobalRect();
    const int panelHeight = previewPanelSizeForAnchor(anchor, bounds).height();

    const int width = std::max(64, kPanelWidth - kPanelPadding * 2);
    const int height = std::max(96, panelHeight - kStatusHeight - kControlBarHeight - kPanelPadding * 4);
    return QSize(width, height);
}

QImage ScrollSessionWindow::renderGnomePreviewImage(const QSize &size) const
{
    if (size.isEmpty()) {
        return {};
    }

    QImage preview(size, QImage::Format_ARGB32_Premultiplied);
    preview.fill(QColor(8, 13, 19, 220));

    QPainter painter(&preview);
    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.setRenderHint(QPainter::SmoothPixmapTransform, true);
    drawPreviewContent(painter, QRect(QPoint(0, 0), size));

    return preview;
}

void ScrollSessionWindow::updateGnomeShellPreview(bool force)
{
#ifndef MARK_SHOT_WITH_DBUS
    Q_UNUSED(force);
    return;
#else
    if (!gnomeShellPreviewActive()) {
        return;
    }
    const bool showHandle = m_gnomeShellOverlay && !m_previewPanelVisible && floatingDragHandleActive();
    if (!m_previewPanelVisible && !showHandle) {
        if (m_gnomePreviewVisible) {
            hideGnomeShellPreview();
        }
        return;
    }

    const qint64 now = QDateTime::currentMSecsSinceEpoch();
    if (!force && now - m_lastGnomePreviewUpdateMs < kGnomePreviewIntervalMs) {
        return;
    }
    m_lastGnomePreviewUpdateMs = now;

    QString previewPath;
    if (!showHandle && (force || m_gnomePreviewImageDirty)) {
        const QSize previewSize = gnomePreviewImageSize();
        const QImage preview = renderGnomePreviewImage(previewSize);
        if (!preview.isNull()) {
            const QString tempDir = QFile::exists(QStringLiteral("/dev/shm"))
                ? QStringLiteral("/dev/shm")
                : QDir::tempPath();
            previewPath = QStringLiteral("%1/mark-shot-gnome-preview-%2-%3.png")
                .arg(tempDir, m_gnomePreviewSessionId)
                .arg(++m_gnomePreviewSerial);
            if (preview.save(previewPath, "PNG")) {
                m_gnomePreviewFiles.append(previewPath);
                cleanupGnomePreviewFiles(8);
                m_gnomePreviewImageDirty = false;
            } else {
                previewPath.clear();
            }
        }
    }

    const QImage result = currentResult();
    const bool horizontal = m_stitcher.axis() == ScrollAxis::Horizontal;
    const int totalLen = result.isNull() ? 0 : (horizontal ? result.width() : result.height());
    const QString status = m_statusText.isEmpty() ? MS_TR("Scroll down to capture") : m_statusText;

    QDBusMessage message = QDBusMessage::createMethodCall(
        QString::fromLatin1(kGnomeShellService),
        QString::fromLatin1(kGnomeHelperPath),
        QString::fromLatin1(kGnomeHelperInterface),
        QStringLiteral("ShowScrollPreview")
    );
    const QRect previewAnchor = previewAnchorGlobalRect();
    message << m_gnomePreviewSessionId
            << previewAnchor.x()
            << previewAnchor.y()
            << previewAnchor.width()
            << previewAnchor.height()
            << previewPath
            << status
            << (horizontal ? QStringLiteral("horizontal") : QStringLiteral("vertical"))
            << m_capturePos
            << m_captureLen
            << totalLen
            << m_paused
            << m_stitcher.axisLocked()
            << (showHandle
                    ? -std::max(1, m_uiConfig.previewGap + 1)
                    : std::max(0, m_uiConfig.previewGap));
    QDBusConnection::sessionBus().call(message, QDBus::NoBlock);
    m_gnomePreviewVisible = true;
#endif
}

void ScrollSessionWindow::hideGnomeShellPreview()
{
#ifndef MARK_SHOT_WITH_DBUS
    return;
#else
    if (!gnomeShellPreviewActive()) {
        return;
    }

    QDBusMessage message = QDBusMessage::createMethodCall(
        QString::fromLatin1(kGnomeShellService),
        QString::fromLatin1(kGnomeHelperPath),
        QString::fromLatin1(kGnomeHelperInterface),
        QStringLiteral("HideScrollPreview")
    );
    message << m_gnomePreviewSessionId;
    QDBusConnection::sessionBus().call(message, QDBus::NoBlock);
    m_gnomePreviewVisible = false;
    cleanupGnomePreviewFiles(0);
#endif
}

void ScrollSessionWindow::cleanupGnomePreviewFiles(int keepLatest)
{
    keepLatest = std::max(0, keepLatest);
    while (m_gnomePreviewFiles.size() > keepLatest) {
        QFile::remove(m_gnomePreviewFiles.takeFirst());
    }
}

void ScrollSessionWindow::handleGnomePreviewAction(const QString &sessionId, const QString &action)
{
    if (!gnomeShellPreviewActive() || sessionId != m_gnomePreviewSessionId) {
        return;
    }

    if (action == QStringLiteral("pause")) {
        togglePause();
    } else if (action == QStringLiteral("axis")) {
        if (!m_stitcher.axisLocked()) {
            toggleAxis();
        }
    } else if (action == QStringLiteral("annotate")) {
        annotateResult();
    } else if (action == QStringLiteral("save")) {
        saveResult();
        updateGnomeShellPreview(true);
    } else if (action == QStringLiteral("copy")) {
        copyResult();
    } else if (action == QStringLiteral("cancel")) {
        close();
    }
}

void ScrollSessionWindow::closeEvent(QCloseEvent *event)
{
    if (m_timer) {
        m_timer->stop();
    }
    cancelScrollIdlePause();
    hideGnomeShellPreview();
    if (m_gnomeShellPreview) {
#ifdef MARK_SHOT_WITH_DBUS
        QDBusConnection::sessionBus().disconnect(QString::fromLatin1(kGnomeShellService),
                                                 QString::fromLatin1(kGnomeHelperPath),
                                                 QString::fromLatin1(kGnomeHelperInterface),
                                                 QStringLiteral("PreviewAction"),
                                                 this,
                                                 SLOT(handleGnomePreviewAction(QString,QString)));
#endif
    }
    stopActiveScreencastCapture();
    event->accept();
}

}  // namespace markshot::scroll
