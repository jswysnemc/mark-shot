
#include "scroll/scroll_session_window_internal.h"

namespace markshot::scroll {

ScrollSessionWindow::ScrollSessionWindow(QRect globalGeometry,
                                         QString outputName,
                                         QScreen *screen,
                                         ScrollSessionUiConfig uiConfig,
                                         QWidget *parent)
    : QWidget(parent)
    , m_geometry(globalGeometry)
    , m_outputName(std::move(outputName))
    , m_uiConfig(uiConfig)
    , m_sessionId(QDateTime::currentMSecsSinceEpoch())
    , m_stitcher(defaultConfig())
{
    setWindowTitle(MS_TR("Scroll Capture"));
    setAttribute(Qt::WA_DeleteOnClose);
    setAttribute(Qt::WA_TranslucentBackground);
    setAttribute(Qt::WA_ShowWithoutActivating);
    setWindowFlags(Qt::Window | Qt::FramelessWindowHint | Qt::WindowStaysOnTopHint);
    markshot::windows::setExcludedFromCapture(this);
    setObjectName(QStringLiteral("scrollSessionWindow"));
    setMouseTracking(true);
    setFocusPolicy(Qt::NoFocus);

    auto *cancelShortcut = new QShortcut(QKeySequence(Qt::Key_Escape), this);
    cancelShortcut->setContext(Qt::ApplicationShortcut);
    connect(cancelShortcut, &QShortcut::activated, this, [this] { close(); });

    if (!screen) {
        screen = QGuiApplication::screenAt(m_geometry.center());
    }
    if (!screen) {
        screen = QGuiApplication::primaryScreen();
    }
    m_screenOrigin = screen ? screen->geometry().topLeft() : QPoint(0, 0);

    buildControlBar();
    m_gnomeShellPreview = isGnomeWaylandSession() && hasGnomeScrollPreviewHelper();
    m_gnomePreviewSessionId = QString::number(m_sessionId);
#ifdef MARK_SHOT_WITH_DBUS
    if (m_gnomeShellPreview) {
        QDBusConnection::sessionBus().connect(QString::fromLatin1(kGnomeShellService),
                                              QString::fromLatin1(kGnomeHelperPath),
                                              QString::fromLatin1(kGnomeHelperInterface),
                                              QStringLiteral("PreviewAction"),
                                              this,
                                              SLOT(handleGnomePreviewAction(QString,QString)));
    }
#endif

    m_layerShell = configureLayerShell(screen);
    m_panelOnlyWindow = !m_layerShell && isWaylandPlatform();
    if (!m_layerShell && screen) {
        setScreen(screen);
        updatePreviewPanelVisibility();
        if (m_panelOnlyWindow) {
            updatePanelWindowGeometry();
        } else {
            setGeometry(screen->geometry());
        }
    }

    m_timer = new QTimer(this);
    m_timer->setInterval(kCaptureIntervalMs);
    connect(m_timer, &QTimer::timeout, this, [this] { captureTick(); });

    m_scrollIdleTimer = new QTimer(this);
    m_scrollIdleTimer->setSingleShot(true);
    m_scrollIdleTimer->setInterval(kScrollIdlePauseMs);
    connect(m_scrollIdleTimer, &QTimer::timeout, this, [this] { handleScrollIdleTimeout(); });

    logScrollDebug("start time=%lld geom=%d,%d %dx%d output=%s algorithm=%s",
                   m_sessionId,
                   m_geometry.x(), m_geometry.y(), m_geometry.width(), m_geometry.height(),
                   m_outputName.toUtf8().constData(), algorithmDebugName());
}

bool ScrollSessionWindow::configureLayerShell(QScreen *screen)
{
    const QSize desiredSize = m_geometry.isValid() && !m_geometry.isEmpty()
        ? m_geometry.size()
        : (screen ? screen->geometry().size() : QSize());
    if (!desiredSize.isEmpty()) {
        resize(desiredSize);
    }

    if (screen) {
        setScreen(screen);
    }

    // Do not hold exclusive keyboard focus; the page below should keep receiving
    // keyboard and wheel input while the panel can still take focus when clicked.
    return markshot::layershell::configureOverlay(
        this,
        screen,
        {QStringLiteral("mark-shot-scroll"),
         markshot::layershell::KeyboardInteractivity::OnDemand,
         false,
         true,
         false});
}

bool ScrollSessionWindow::layerShellActive() const
{
    return m_layerShell;
}

void ScrollSessionWindow::buildControlBar()
{
    m_controlBar = new QWidget(this);
    m_controlBar->setObjectName(QStringLiteral("shotToolbar"));
    m_controlBar->setStyleSheet(markshot::theme::panelStyleSheet());

    auto *row = new QHBoxLayout(m_controlBar);
    row->setContentsMargins(7, 7, 7, 7);
    row->setSpacing(7);

    auto makeButton = [this, row](const QIcon &icon, const QString &label, const QString &role = QString()) {
        auto *button = new QPushButton(m_controlBar);
        button->setProperty("role", role.isEmpty() ? QStringLiteral("secondary") : role);
        configureIconButton(button, icon, label);
        applyControlButtonChrome(button);
        row->addWidget(button, 0, Qt::AlignCenter);
        return button;
    };

    m_axisButton = makeButton(makeControlIcon(ControlIcon::AxisVertical), MS_TR("Dir: Vertical"));
    m_axisButton->installEventFilter(this);

    m_floatingAxisButton = new QPushButton(this);
    m_floatingAxisButton->setProperty("role", QStringLiteral("secondary"));
    configureIconButton(m_floatingAxisButton,
                        makeControlIcon(ControlIcon::AxisVertical),
                        MS_TR("Dir: Vertical"));
    applyControlButtonChrome(m_floatingAxisButton);
    m_floatingAxisButton->installEventFilter(this);
    m_floatingAxisButton->hide();

    row->addSpacing(4);
    m_pauseButton = makeButton(makeControlIcon(ControlIcon::Pause), MS_TR("Pause"));
    m_annotateButton = makeButton(makeControlIcon(ControlIcon::Annotate), MS_TR("Annotate"));
    m_saveButton = makeButton(makeControlIcon(ControlIcon::Save), MS_TR("Save"), QStringLiteral("primary"));
    m_copyButton = makeButton(makeControlIcon(ControlIcon::Copy), MS_TR("Copy"));
    m_cancelButton = makeButton(makeControlIcon(ControlIcon::Cancel), MS_TR("Cancel"), QStringLiteral("danger"));

    connect(m_axisButton, &QPushButton::clicked, this, [this] { toggleAxis(); });
    connect(m_floatingAxisButton, &QPushButton::clicked, this, [this] { toggleAxis(); });
    connect(m_pauseButton, &QPushButton::clicked, this, [this] { togglePause(); });
    connect(m_annotateButton, &QPushButton::clicked, this, [this] { annotateResult(); });
    connect(m_saveButton, &QPushButton::clicked, this, [this] { saveResult(); });
    connect(m_copyButton, &QPushButton::clicked, this, [this] { copyResult(); });
    connect(m_cancelButton, &QPushButton::clicked, this, [this] { close(); });

    refreshControlLabels();
}

QRect ScrollSessionWindow::regionLocalRect() const
{
    return m_geometry.translated(-m_screenOrigin);
}

QRect ScrollSessionWindow::frameOuterLocalRect() const
{
    const QRect region = regionLocalRect();
    return m_uiConfig.frameEnabled
        ? captureFrameOuterRect(region, m_uiConfig.frameGap)
        : region;
}

QRect ScrollSessionWindow::previewAnchorLocalRect() const
{
    return frameOuterLocalRect();
}

QRect ScrollSessionWindow::previewAnchorGlobalRect() const
{
    const QRect region = m_geometry.normalized();
    return m_uiConfig.frameEnabled
        ? captureFrameOuterRect(region, m_uiConfig.frameGap)
        : region;
}

QRect ScrollSessionWindow::previewPanelRect() const
{
    if (m_panelOnlyWindow) {
        return QRect(QPoint(0, 0), size());
    }

    const QRect anchor = previewAnchorLocalRect();
    const QRect bounds(QPoint(0, 0), size());
    const QSize panelSize = previewPanelSizeForAnchor(anchor, bounds);
    return choosePreviewPanelPlacement(anchor, bounds, panelSize, m_uiConfig.previewGap).rect;
}

QRegion ScrollSessionWindow::framePaintRegion() const
{
    const QRect bounds(QPoint(0, 0), size());
    if (bounds.isEmpty() || !m_uiConfig.frameEnabled || m_panelOnlyWindow) {
        return {};
    }

    const QRect region = regionLocalRect();
    QRegion frame = QRegion(captureFrameOuterRect(region, m_uiConfig.frameGap)
                                .adjusted(-kAntialiasPad,
                                          -kAntialiasPad,
                                          kAntialiasPad,
                                          kAntialiasPad));
    frame -= QRegion(captureFrameInnerRect(region, m_uiConfig.frameGap)
                         .adjusted(kAntialiasPad,
                                   kAntialiasPad,
                                   -kAntialiasPad,
                                   -kAntialiasPad));
    return frame.intersected(QRegion(bounds));
}

bool ScrollSessionWindow::previewPanelFitsAvailableSpace() const
{
    if (m_panelOnlyWindow || m_gnomeShellPreview) {
        const QRect bounds = captureBoundsGlobal();
        const QRect anchor = previewAnchorGlobalRect();
        const QSize panelSize = previewPanelSizeForAnchor(anchor, bounds);
        return choosePreviewPanelPlacement(anchor, bounds, panelSize, m_uiConfig.previewGap)
            .fitsWithoutOverlap;
    }

    const QRect anchor = previewAnchorLocalRect();
    const QRect bounds(QPoint(0, 0), size());
    const QSize panelSize = previewPanelSizeForAnchor(anchor, bounds);
    return choosePreviewPanelPlacement(anchor, bounds, panelSize, m_uiConfig.previewGap)
        .fitsWithoutOverlap;
}

bool ScrollSessionWindow::shouldAvoidPreviewOverlapForCapture() const
{
#if defined(Q_OS_WIN)
    return false;
#else
    return true;
#endif
}

bool ScrollSessionWindow::floatingDragHandleActive() const
{
    if (m_previewPanelVisible) {
        return false;
    }

    if (!m_uiConfig.hidePreviewDuringCapture && previewPanelFitsAvailableSpace()) {
        return false;
    }

    if (m_panelOnlyWindow || m_gnomeShellPreview) {
        return !floatingDragHandleGlobalRect().isEmpty();
    }
    return !floatingDragHandleLocalRect().isEmpty();
}

QRect ScrollSessionWindow::floatingDragHandleLocalRect() const
{
    if (m_panelOnlyWindow) {
        return QRect(QPoint(0, 0), controlButtonSize()).intersected(rect());
    }

    const QRect bounds(QPoint(0, 0), size());
    return chooseFloatingDragHandleRect(previewAnchorLocalRect(), bounds);
}

QRect ScrollSessionWindow::floatingDragHandleGlobalRect() const
{
    return chooseFloatingDragHandleRect(previewAnchorGlobalRect(), captureBoundsGlobal());
}

QRect ScrollSessionWindow::captureBoundsGlobal() const
{
    QScreen *targetScreen = screen();
    if (!targetScreen) {
        targetScreen = QGuiApplication::screenAt(m_geometry.center());
    }
    if (!targetScreen) {
        targetScreen = QGuiApplication::primaryScreen();
    }
    return targetScreen ? targetScreen->geometry() : m_geometry.normalized();
}

QRect ScrollSessionWindow::floatingPanelGlobalRect() const
{
    const QRect bounds = captureBoundsGlobal();
    if (bounds.isEmpty()) {
        return QRect(QPoint(0, 0), QSize(kPanelWidth, 360));
    }

    const QRect anchor = previewAnchorGlobalRect();
    const QSize panelSize = previewPanelSizeForAnchor(anchor, bounds);
    return choosePreviewPanelPlacement(anchor, bounds, panelSize, m_uiConfig.previewGap).rect;
}

void ScrollSessionWindow::updatePanelWindowGeometry()
{
    if (!m_panelOnlyWindow) {
        return;
    }
    const QRect bounds = captureBoundsGlobal();
    const QPoint hiddenPoint = bounds.isEmpty() ? QPoint(0, 0) : bounds.bottomRight();
    QRect panel = QRect(hiddenPoint, QSize(1, 1));
    if (m_gnomeShellPreview && floatingDragHandleActive()) {
        panel = floatingDragHandleGlobalRect();
    } else if (m_previewPanelVisible) {
        panel = floatingPanelGlobalRect();
    } else if (floatingDragHandleActive()) {
        panel = floatingDragHandleGlobalRect();
    }
    if (geometry() != panel) {
        setGeometry(panel);
    }
    m_screenOrigin = panel.topLeft();
}

QRegion ScrollSessionWindow::overlayPaintRegion() const
{
    const QRect bounds(QPoint(0, 0), size());
    if (bounds.isEmpty()) {
        return {};
    }
    if (m_panelOnlyWindow) {
        return (m_previewPanelVisible || floatingDragHandleActive()) ? QRegion(bounds) : QRegion();
    }

    QRegion painted;
    painted += framePaintRegion();
    if (m_previewPanelVisible) {
        painted += QRegion(previewPanelRect().adjusted(-kAntialiasPad,
                                                       -kAntialiasPad,
                                                       kAntialiasPad,
                                                       kAntialiasPad));
    }
    if (floatingDragHandleActive()) {
        painted += QRegion(floatingDragHandleLocalRect().adjusted(-kAntialiasPad,
                                                                  -kAntialiasPad,
                                                                  kAntialiasPad,
                                                                  kAntialiasPad));
    }
    return painted.intersected(QRegion(bounds));
}

void ScrollSessionWindow::setRegionGeometry(QRect geometry)
{
    geometry = geometry.normalized();
    if (!geometry.isValid() || geometry.isEmpty() || geometry == m_geometry.normalized()) {
        return;
    }

    const QRegion oldPaint = overlayPaintRegion();
    m_geometry = geometry;
    updatePreviewPanelVisibility();
    if (m_panelOnlyWindow && !m_axisDragging) {
        updatePanelWindowGeometry();
    }
    layoutOverlay();
    m_transientPaintMask += oldPaint;
    m_transientPaintMask += overlayPaintRegion();
    updateInputMask();
    syncPreviewWindowVisibility();
    if (!m_transientPaintMask.isEmpty()) {
        update(m_transientPaintMask);
    } else {
        update();
    }
    m_gnomePreviewImageDirty = true;
    updateGnomeShellPreview(true);
}

void ScrollSessionWindow::layoutFloatingDragHandle()
{
    if (!m_floatingAxisButton) {
        return;
    }

    m_floatingAxisButton->hide();
}

void ScrollSessionWindow::drawFloatingDragHandle(QPainter &painter) const
{
    if (!floatingDragHandleActive() || m_panelTransparentForCapture) {
        return;
    }

    const QRect handle = floatingDragHandleLocalRect();
    if (handle.isEmpty()) {
        return;
    }

    QPainterPath path;
    path.addRoundedRect(handle, 10, 10);
    painter.fillPath(path, QColor(15, 17, 23, 242));
    painter.setPen(QPen(QColor(94, 234, 212, 150), 1));
    painter.drawPath(path);

    const bool horizontal = m_stitcher.axis() == ScrollAxis::Horizontal;
    const QIcon icon = makeControlIcon(horizontal ? ControlIcon::AxisHorizontal
                                                  : ControlIcon::AxisVertical);
    icon.paint(&painter, handle.adjusted(9, 7, -9, -7), Qt::AlignCenter);
}

void ScrollSessionWindow::armAxisDrag(const QPoint &globalPos)
{
    m_axisDragArmed = true;
    m_axisDragging = false;
    m_axisDragStartGlobal = globalPos;
    m_axisDragStartGeometry = m_geometry.normalized();
}

bool ScrollSessionWindow::updateAxisDrag(const QPoint &globalPos)
{
    if (!m_axisDragArmed) {
        return false;
    }

    constexpr int kDragThresholdPx = 5;
    const QPoint delta = globalPos - m_axisDragStartGlobal;

    if (!m_axisDragging) {
        if (std::abs(delta.x()) < kDragThresholdPx && std::abs(delta.y()) < kDragThresholdPx) {
            return false;
        }
        // Threshold exceeded: enter drag mode. In the GNOME xdg-window
        // fallback, pause capture while the region moves so intermediate
        // drag frames do not disturb the stitcher's current anchor. On
        // layer-shell compositors, live stitching during drag is preserved.
        if (m_autoPausedForPreview) {
            resumeAutoPausedCapture();
        }
        m_axisDragging = true;
        m_lastSignature.clear();
        m_transientPaintMask = overlayPaintRegion();
        m_restoreMaskAfterPaint = false;
        m_statusText = MS_TR("Dragging region");
        refreshControlLabels();
        setCursor(Qt::ClosedHandCursor);
    }

    const bool horizontal = m_stitcher.axis() == ScrollAxis::Horizontal;
    QPoint constrainedDelta = delta;
    if (horizontal) {
        constrainedDelta.setY(0);
    } else {
        constrainedDelta.setX(0);
    }

    QRect next = m_axisDragStartGeometry.translated(constrainedDelta);
    const QRect bounds = captureBoundsGlobal();
    if (next.left() < bounds.left()) {
        next.translate(bounds.left() - next.left(), 0);
    }
    if (next.top() < bounds.top()) {
        next.translate(0, bounds.top() - next.top());
    }
    if (next.right() > bounds.right()) {
        next.translate(bounds.right() - next.right(), 0);
    }
    if (next.bottom() > bounds.bottom()) {
        next.translate(0, bounds.bottom() - next.bottom());
    }

    setRegionGeometry(next);
    m_statusText = MS_TR("Dragging region");
    return true;
}

bool ScrollSessionWindow::finishAxisDrag()
{
    if (!m_axisDragArmed) {
        return false;
    }

    const bool wasDragging = m_axisDragging;
    m_axisDragArmed = false;
    m_axisDragging = false;
    unsetCursor();

    if (!wasDragging) {
        return false;
    }

    m_lastSignature.clear();
    m_statusText = MS_TR("Region adjusted");
    refreshControlLabels();
    updatePanelWindowGeometry();
    m_transientPaintMask += overlayPaintRegion();
    m_restoreMaskAfterPaint = !m_transientPaintMask.isEmpty();
    updateInputMask();
    if (!m_transientPaintMask.isEmpty()) {
        update(m_transientPaintMask);
    } else {
        update();
    }
    return true;
}

void ScrollSessionWindow::layoutOverlay()
{
    if (m_gnomeShellPreview) {
        if (m_controlBar) {
            m_controlBar->hide();
        }
        layoutFloatingDragHandle();
        return;
    }

    if (!m_previewPanelVisible) {
        if (m_controlBar) {
            m_controlBar->hide();
        }
        layoutFloatingDragHandle();
        return;
    }

    const QRect panel = previewPanelRect();
    if (m_controlBar && !m_controlBar->isVisible() && !m_panelTransparentForCapture) {
        m_controlBar->show();
    }
    if (m_axisButton) {
        m_axisButton->setVisible(!floatingDragHandleActive());
    }
    const int barWidth = panel.width() - kPanelPadding * 2;
    const int barTop = panel.bottom() - kPanelPadding - kControlBarHeight;
    m_controlBar->setGeometry(panel.left() + kPanelPadding,
                              barTop,
                              barWidth,
                              kControlBarHeight);
    layoutFloatingDragHandle();
}

void ScrollSessionWindow::updateInputMask()
{
    if (m_gnomeShellPreview) {
        QRegion mask;
        if (floatingDragHandleActive()) {
            mask += QRegion(floatingDragHandleLocalRect());
        }
        setMask(mask);
        if (QWindow *nativeWindow = windowHandle()) {
            nativeWindow->setMask(mask);
        }
        return;
    }

    if (m_panelOnlyWindow) {
        const QRegion mask = (m_previewPanelVisible || floatingDragHandleActive())
            ? QRegion(rect())
            : QRegion();
        setMask(mask);
        if (QWindow *nativeWindow = windowHandle()) {
            nativeWindow->setMask(mask);
        }
        return;
    }

    // QWidget masks also clip painting on X11, so the visible frame must stay
    // in the mask. The capture region interior remains click-through so the
    // user can keep scrolling the page underneath.
    QRegion mask;
    if (m_previewPanelVisible) {
        mask += QRegion(previewPanelRect());
    }
    if (floatingDragHandleActive()) {
        mask += QRegion(floatingDragHandleLocalRect());
    }
    mask += framePaintRegion();
    mask += m_transientPaintMask;
    setMask(mask);
    if (QWindow *nativeWindow = windowHandle()) {
        nativeWindow->setMask(mask);
    }
}

void ScrollSessionWindow::showEvent(QShowEvent *event)
{
    QWidget::showEvent(event);
    markshot::windows::setExcludedFromCapture(this);
    updatePreviewPanelVisibility();
    if (m_panelOnlyWindow) {
        updatePanelWindowGeometry();
    } else if (!m_layerShell) {
        m_screenOrigin = geometry().topLeft();
    }
    layoutOverlay();
    updateInputMask();
    syncPreviewWindowVisibility();
    const QRect region = regionLocalRect();
    const QRect panel = previewPanelRect();
    logScrollDebug("layout window=%d,%d %dx%d screen_origin=%d,%d region=%d,%d %dx%d "
                   "panel=%d,%d %dx%d panel_gap=%d panel_overlap=%d layer_shell=%d panel_only=%d",
                   geometry().x(), geometry().y(), geometry().width(), geometry().height(),
                   m_screenOrigin.x(), m_screenOrigin.y(),
                   region.x(), region.y(), region.width(), region.height(),
                   panel.x(), panel.y(), panel.width(), panel.height(),
                   m_uiConfig.previewGap,
                   panel.intersects(region) ? 1 : 0, m_layerShell ? 1 : 0,
                   m_panelOnlyWindow ? 1 : 0);
    updateGnomeShellPreview(true);
    if (m_timer && !m_timer->isActive()) {
        // Let the compositor apply the window mask before the first X11 capture;
        // otherwise the seed frame can include the scroll overlay itself.
        QTimer::singleShot(120, this, [this] {
            if (m_timer && !m_timer->isActive() && !m_paused) {
                m_timer->start();
            }
        });
    }
    syncPreviewWindowVisibility();
}

}  // namespace markshot::scroll
