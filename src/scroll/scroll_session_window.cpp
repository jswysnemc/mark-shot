#include "scroll/scroll_session_window.h"

#include "annotation_launch.h"
#include "screen_capture.h"
#include "scroll/scroll_config.h"
#include "ui/i18n.h"
#include "ui/theme.h"

#ifdef HAVE_LAYER_SHELL
#include <LayerShellQt/Window>
#endif

#include <QApplication>
#include <QBuffer>
#include <QByteArray>
#include <QClipboard>
#include <QCloseEvent>
#include <QDateTime>
#include <QDir>
#include <QFileDialog>
#include <QFont>
#include <QGuiApplication>
#include <QHBoxLayout>
#include <QKeyEvent>
#include <QKeySequence>
#include <QPainter>
#include <QPainterPath>
#include <QPaintEvent>
#include <QProcess>
#include <QProcessEnvironment>
#include <QPushButton>
#include <QRegion>
#include <QResizeEvent>
#include <QScreen>
#include <QShortcut>
#include <QShowEvent>
#include <QSignalBlocker>
#include <QStandardPaths>
#include <QTemporaryFile>
#include <QSlider>
#include <QTimer>
#include <QVBoxLayout>
#include <QWindow>

#include <algorithm>
#include <cmath>
#include <cstdarg>
#include <cstdio>
#include <utility>

namespace markshot::scroll {

namespace {

constexpr int kCaptureIntervalMs = 45;
constexpr int kBlinkIntervalMs = 500;
constexpr int kSignatureCols = 18;
constexpr int kSignatureRows = 24;
constexpr float kDuplicateAvgDiff = 1.1f;
constexpr int kDuplicateMaxDiff = 4;

constexpr int kBorderWidth = 3;       // thickness of the blinking capture frame
constexpr int kBorderGap = 2;         // gap between region and border (kept outside)
constexpr int kPanelWidth = 340;      // preview panel width
constexpr int kPanelGap = 14;         // gap between region and preview panel
constexpr int kPanelPadding = 12;
constexpr int kControlBarHeight = 76;   // two rows: algorithm/axis + actions
constexpr int kStatusHeight = 22;
constexpr int kScrubberHeight = 22;     // non-destructive position slider
constexpr qint64 kStoppedUiDelayMs = 650;
constexpr qint64 kOverlaySettleMs = 180;

std::uint8_t grayPixel(const QImage &frame, int x, int y)
{
    const QRgb px = frame.pixel(x, y);
    const int gray =
        static_cast<int>(0.299 * qRed(px) + 0.587 * qGreen(px) + 0.114 * qBlue(px));
    return static_cast<std::uint8_t>(gray);
}

// A coarse grayscale grid used to tell whether the latest capture is the same
// as the previous one (the user has not scrolled yet). Mirrors wayscrollshot's
// frame_signature. Only hidden clean frames use this signature; visible UI frames
// are never stitched or used as the final scroll decision.
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

const char *algorithmDebugName(StitchAlgorithm algorithm)
{
    return algorithm == StitchAlgorithm::OpenCvOrb ? "opencv-orb" : "col-sample";
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
    FILE *file = std::fopen("/tmp/mark-shot-scroll.log", "a");
    if (!file) {
        return;
    }

    std::fprintf(file, "[session] ");
    va_list args;
    va_start(args, format);
    std::vfprintf(file, format, args);
    va_end(args);
    std::fprintf(file, "\n");
    std::fclose(file);
}

}  // namespace

ScrollSessionWindow::ScrollSessionWindow(QRect globalGeometry,
                                         QString outputName,
                                         StitchAlgorithm algorithm,
                                         QScreen *screen,
                                         QWidget *parent)
    : QWidget(parent)
    , m_geometry(globalGeometry)
    , m_outputName(std::move(outputName))
    , m_stitcher(defaultConfigFor(algorithm))
{
    setWindowTitle(MS_TR("Scroll Capture"));
    setAttribute(Qt::WA_DeleteOnClose);
    setAttribute(Qt::WA_TranslucentBackground);
    setWindowFlags(Qt::Window | Qt::FramelessWindowHint | Qt::WindowStaysOnTopHint);
    setObjectName(QStringLiteral("scrollSessionWindow"));
    setMouseTracking(true);
    setFocusPolicy(Qt::StrongFocus);

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

    m_layerShell = configureLayerShell(screen);
    if (!m_layerShell && screen) {
        setScreen(screen);
        setGeometry(screen->geometry());
    }

    m_timer = new QTimer(this);
    m_timer->setInterval(kCaptureIntervalMs);
    connect(m_timer, &QTimer::timeout, this, [this] { captureTick(); });

    m_blinkTimer = new QTimer(this);
    m_blinkTimer->setInterval(kBlinkIntervalMs);
    connect(m_blinkTimer, &QTimer::timeout, this, [this] {
        m_blinkOn = !m_blinkOn;
        update();
    });

    logScrollDebug("start time=%lld geom=%d,%d %dx%d output=%s algorithm=%s opencv_available=%d",
                   QDateTime::currentMSecsSinceEpoch(),
                   m_geometry.x(), m_geometry.y(), m_geometry.width(), m_geometry.height(),
                   m_outputName.toUtf8().constData(), algorithmDebugName(algorithm),
                   openCvAvailable() ? 1 : 0);
}

bool ScrollSessionWindow::configureLayerShell(QScreen *screen)
{
#ifndef HAVE_LAYER_SHELL
    Q_UNUSED(screen);
    return false;
#else
    if (screen) {
        setScreen(screen);
    }

    setAttribute(Qt::WA_NativeWindow);
    winId();

    QWindow *nativeWindow = windowHandle();
    if (!nativeWindow) {
        return false;
    }
    if (screen) {
        nativeWindow->setScreen(screen);
    }

    LayerShellQt::Window *layerWindow = LayerShellQt::Window::get(nativeWindow);
    if (!layerWindow) {
        return false;
    }

    LayerShellQt::Window::Anchors anchors = LayerShellQt::Window::AnchorTop;
    anchors |= LayerShellQt::Window::AnchorBottom;
    anchors |= LayerShellQt::Window::AnchorLeft;
    anchors |= LayerShellQt::Window::AnchorRight;

    layerWindow->setScope(QStringLiteral("mark-shot-scroll"));
    layerWindow->setLayer(LayerShellQt::Window::LayerOverlay);
    layerWindow->setAnchors(anchors);
    layerWindow->setMargins({});
    layerWindow->setExclusiveZone(-1);
    // Keep keyboard focus so Esc can cancel the session. Pointer input remains
    // controlled by the input mask, so scrolling the page through the selected
    // region still works.
    layerWindow->setKeyboardInteractivity(LayerShellQt::Window::KeyboardInteractivityExclusive);
    layerWindow->setActivateOnShow(true);
    layerWindow->setCloseOnDismissed(false);
    if (screen) {
        layerWindow->setScreen(screen);
    } else {
        layerWindow->setWantsToBeOnActiveScreen(true);
    }
    layerWindow->setDesiredSize({});
    return true;
#endif
}

void ScrollSessionWindow::buildControlBar()
{
    m_controlBar = new QWidget(this);
    m_controlBar->setObjectName(QStringLiteral("shotToolbar"));
    m_controlBar->setStyleSheet(markshot::theme::panelStyleSheet());

    auto *rows = new QVBoxLayout(m_controlBar);
    rows->setContentsMargins(6, 6, 6, 6);
    rows->setSpacing(4);

    auto *topRow = new QHBoxLayout;
    topRow->setContentsMargins(0, 0, 0, 0);
    topRow->setSpacing(4);
    auto *bottomRow = new QHBoxLayout;
    bottomRow->setContentsMargins(0, 0, 0, 0);
    bottomRow->setSpacing(4);
    rows->addLayout(topRow);
    rows->addLayout(bottomRow);

    auto makeButton = [this](QHBoxLayout *row, const QString &text) {
        auto *button = new QPushButton(text, m_controlBar);
        button->setFocusPolicy(Qt::NoFocus);
        button->setCursor(Qt::PointingHandCursor);
        button->setStyleSheet(QStringLiteral(
            "QPushButton {"
            " color: #E5E7EB; background: rgba(255,255,255,16);"
            " border: 1px solid rgba(255,255,255,24); border-radius: 7px;"
            " padding: 5px 8px; min-width: 0; max-width: 16777215;"
            " min-height: 24px; max-height: 28px; font-size: 12px; font-weight: 600; }"
            "QPushButton:hover { background: rgba(45,212,191,30);"
            " border-color: rgba(45,212,191,90); }"
            "QPushButton:disabled { color: rgba(229,231,235,90);"
            " background: rgba(255,255,255,8); border-color: rgba(255,255,255,14); }"));
        row->addWidget(button);
        return button;
    };

    // Top row: capture-control toggles (algorithm switch, scroll axis). The axis
    // button locks after the first frame (the long image's orientation is fixed).
    m_algorithmButton = makeButton(topRow, QString());
    m_axisButton = makeButton(topRow, QString());

    // Bottom row: session actions.
    m_pauseButton = makeButton(bottomRow, MS_TR("Pause"));
    m_annotateButton = makeButton(bottomRow, MS_TR("Annotate"));
    m_saveButton = makeButton(bottomRow, MS_TR("Save"));
    m_copyButton = makeButton(bottomRow, MS_TR("Copy"));
    m_cancelButton = makeButton(bottomRow, MS_TR("Cancel"));

    connect(m_algorithmButton, &QPushButton::clicked, this, [this] { toggleAlgorithm(); });
    connect(m_axisButton, &QPushButton::clicked, this, [this] { toggleAxis(); });
    connect(m_pauseButton, &QPushButton::clicked, this, [this] { togglePause(); });
    connect(m_annotateButton, &QPushButton::clicked, this, [this] { annotateResult(); });
    connect(m_saveButton, &QPushButton::clicked, this, [this] { saveResult(); });
    connect(m_copyButton, &QPushButton::clicked, this, [this] { copyResult(); });
    connect(m_cancelButton, &QPushButton::clicked, this, [this] { close(); });

    // Scrubber: a thin slider above the control bar. It moves which window of the
    // long image the detail view shows, without ever discarding stitched content.
    m_scrubber = new QSlider(Qt::Horizontal, this);
    m_scrubber->setObjectName(QStringLiteral("scrollScrubber"));
    m_scrubber->setFocusPolicy(Qt::NoFocus);
    m_scrubber->setCursor(Qt::PointingHandCursor);
    m_scrubber->setRange(0, 0);
    m_scrubber->setStyleSheet(QStringLiteral(
        "QSlider { min-height: 18px; max-height: 18px; }"
        "QSlider::groove:horizontal { height: 4px; border-radius: 2px;"
        " background: rgba(255,255,255,28); }"
        "QSlider::handle:horizontal { width: 14px; margin: -6px 0; border-radius: 7px;"
        " background: #2DD4BF; }"
        "QSlider::handle:horizontal:hover { background: #5EEAD4; }"));
    connect(m_scrubber, &QSlider::sliderPressed, this, [this] { m_following = false; });
    connect(m_scrubber, &QSlider::valueChanged, this, [this](int value) {
        m_scrubPos = value;
        update();
    });
    connect(m_scrubber, &QSlider::sliderReleased, this, [this] {
        // Releasing at the very end resumes following the live edge.
        if (m_scrubber->value() >= m_scrubber->maximum()) {
            m_following = true;
        }
        update();
    });

    refreshControlLabels();
}

QRect ScrollSessionWindow::regionLocalRect() const
{
    return m_geometry.translated(-m_screenOrigin);
}

QRect ScrollSessionWindow::previewPanelRect() const
{
    const QRect region = regionLocalRect();
    const QRect bounds(QPoint(0, 0), size());

    // Preview height matches the region but is clamped to the overlay bounds and
    // a sensible minimum so the controls always fit.
    const int minHeight = kControlBarHeight + kStatusHeight + kPanelPadding * 3 + 120;
    int panelHeight = std::max(minHeight, region.height());
    panelHeight = std::min(panelHeight, bounds.height() - 8);

    int top = region.top();
    top = std::clamp(top, 4, std::max(4, bounds.height() - panelHeight - 4));

    // Prefer the right side of the region; fall back to the left when there is
    // not enough room.
    int left = region.right() + kPanelGap;
    if (left + kPanelWidth > bounds.right() - 4) {
        const int leftAlt = region.left() - kPanelGap - kPanelWidth;
        if (leftAlt >= 4) {
            left = leftAlt;
        } else {
            left = std::max(4, bounds.right() - kPanelWidth - 4);
        }
    }
    left = std::clamp(left, 4, std::max(4, bounds.width() - kPanelWidth - 4));

    return QRect(left, top, kPanelWidth, panelHeight);
}

QRect ScrollSessionWindow::overlayUiFrameRect(const QImage &frame) const
{
    if (frame.isNull()) {
        return {};
    }

    const QRect region = regionLocalRect();
    const QRect overlap = previewPanelRect().intersected(region);
    if (overlap.isEmpty() || region.width() <= 0 || region.height() <= 0) {
        return {};
    }

    const qreal scaleX = static_cast<qreal>(frame.width()) / region.width();
    const qreal scaleY = static_cast<qreal>(frame.height()) / region.height();
    const QRectF frameRect((overlap.left() - region.left()) * scaleX,
                           (overlap.top() - region.top()) * scaleY,
                           overlap.width() * scaleX,
                           overlap.height() * scaleY);
    return frameRect.toAlignedRect()
        .adjusted(-8, -8, 8, 8)
        .intersected(QRect(QPoint(0, 0), frame.size()));
}

bool ScrollSessionWindow::previewCanFitOutsideRegion() const
{
    const QRect region = regionLocalRect();
    const QRect bounds(QPoint(0, 0), size());
    const bool fitsRight = region.right() + kPanelGap + kPanelWidth <= bounds.right() - 4;
    const bool fitsLeft = region.left() - kPanelGap - kPanelWidth >= 4;
    return fitsRight || fitsLeft;
}

bool ScrollSessionWindow::shouldAutoHideOverlayUi() const
{
    return !previewCanFitOutsideRegion();
}

void ScrollSessionWindow::setOverlayUiVisible(bool visible)
{
    if (m_overlayUiVisible == visible) {
        return;
    }

    logScrollDebug("overlay-ui visible=%d auto_hide=%d suspended=%d probe=%d resume_at=%lld detect_at=%lld",
                   visible ? 1 : 0,
                   m_autoHideOverlayUi ? 1 : 0,
                   m_captureSuspendedForUi ? 1 : 0,
                   m_hiddenProbePending ? 1 : 0,
                   m_resumeCaptureAt,
                   m_nextVisibleDetectAt);
    m_overlayUiVisible = visible;
    if (m_controlBar) {
        m_controlBar->setVisible(visible);
    }
    if (m_scrubber) {
        m_scrubber->setVisible(visible);
    }
    updateInputMask();
    repaint();
}

void ScrollSessionWindow::layoutOverlay()
{
    const QRect panel = previewPanelRect();
    const int barWidth = panel.width() - kPanelPadding * 2;
    const int barTop = panel.bottom() - kPanelPadding - kControlBarHeight;
    m_controlBar->setGeometry(panel.left() + kPanelPadding,
                              barTop,
                              barWidth,
                              kControlBarHeight);
    if (m_scrubber) {
        // The scrubber sits just above the control bar, spanning the panel width.
        m_scrubber->setGeometry(panel.left() + kPanelPadding,
                                barTop - kScrubberHeight,
                                barWidth,
                                kScrubberHeight);
    }
    if (m_controlBar) {
        m_controlBar->setVisible(m_overlayUiVisible);
    }
    if (m_scrubber) {
        m_scrubber->setVisible(m_overlayUiVisible);
    }
}

void ScrollSessionWindow::updateInputMask()
{
    // Only the blinking border ring and the preview panel should catch input;
    // the captured region interior and everything else stays click-through so
    // the user can keep scrolling the page underneath.
    const QRect region = regionLocalRect();
    const QRect outer = region.adjusted(-(kBorderGap + kBorderWidth),
                                        -(kBorderGap + kBorderWidth),
                                        kBorderGap + kBorderWidth,
                                        kBorderGap + kBorderWidth);
    const QRect inner = region.adjusted(-kBorderGap, -kBorderGap, kBorderGap, kBorderGap);

    QRegion mask(outer);
    mask -= QRegion(inner);
    if (m_overlayUiVisible) {
        mask += QRegion(previewPanelRect());
    }
    setMask(mask);
}

void ScrollSessionWindow::showEvent(QShowEvent *event)
{
    QWidget::showEvent(event);
    m_autoHideOverlayUi = shouldAutoHideOverlayUi();
    if (m_autoHideOverlayUi) {
        const qint64 now = QDateTime::currentMSecsSinceEpoch();
        m_captureSuspendedForUi = false;
        m_hiddenProbePending = false;
        m_idleSince = 0;
        m_nextVisibleDetectAt = 0;
        m_resumeCaptureAt = now + kOverlaySettleMs;
        setOverlayUiVisible(false);
    }
    layoutOverlay();
    updateInputMask();
    if (m_timer && !m_timer->isActive()) {
        m_timer->start();
    }
    if (m_blinkTimer && !m_blinkTimer->isActive()) {
        m_blinkTimer->start();
    }
    raise();
    activateWindow();
    setFocus(Qt::ActiveWindowFocusReason);
}

void ScrollSessionWindow::resizeEvent(QResizeEvent *event)
{
    QWidget::resizeEvent(event);
    m_autoHideOverlayUi = shouldAutoHideOverlayUi();
    if (!m_autoHideOverlayUi) {
        m_captureSuspendedForUi = false;
        m_hiddenProbePending = false;
        m_idleSince = 0;
        m_resumeCaptureAt = 0;
        m_nextVisibleDetectAt = 0;
        setOverlayUiVisible(true);
    } else {
        const qint64 now = QDateTime::currentMSecsSinceEpoch();
        m_captureSuspendedForUi = false;
        m_hiddenProbePending = false;
        m_idleSince = 0;
        m_nextVisibleDetectAt = 0;
        m_resumeCaptureAt = now + kOverlaySettleMs;
        setOverlayUiVisible(false);
    }
    layoutOverlay();
    updateInputMask();
}

void ScrollSessionWindow::captureTick()
{
    if (m_paused) {
        return;
    }

    const qint64 now = QDateTime::currentMSecsSinceEpoch();
    QImage frame;
    auto captureFrame = [&](const char *debugTag) {
        CaptureRequest request;
        request.preferredOutputName = m_outputName;
        request.sourceGeometry = m_geometry;
        request.allOutputs = false;

        const CaptureResult result = captureScreenFrame(request);
        if (result.image.isNull()) {
            m_statusText = MS_TR("Capture error");
            logScrollDebug("%s-capture-error geom=%d,%d %dx%d output=%s",
                           debugTag,
                           request.sourceGeometry.x(), request.sourceGeometry.y(),
                           request.sourceGeometry.width(), request.sourceGeometry.height(),
                           request.preferredOutputName.toUtf8().constData());
            update();
            return false;
        }

        frame = result.image.convertToFormat(QImage::Format_ARGB32_Premultiplied);
        return true;
    };

    m_autoHideOverlayUi = shouldAutoHideOverlayUi();
    if (m_autoHideOverlayUi) {
        if (m_hiddenProbePending) {
            if (m_overlayUiVisible) {
                m_resumeCaptureAt = now + kOverlaySettleMs;
                setOverlayUiVisible(false);
                return;
            }
            if (now < m_resumeCaptureAt) {
                return;
            }
            if (!captureFrame("hidden-probe")) {
                return;
            }

            m_hiddenProbePending = false;
            const QVector<std::uint8_t> probeSignature =
                frameSignature(frame, kSignatureCols, kSignatureRows);
            if (!m_lastSignature.isEmpty()
                && isDuplicateSignature(m_lastSignature, probeSignature)) {
                m_captureSuspendedForUi = true;
                m_nextVisibleDetectAt = now + kStoppedUiDelayMs;
                setOverlayUiVisible(true);
                logScrollDebug("hidden-probe-idle frame=%dx%d axis=%s",
                               frame.width(), frame.height(), axisDebugName(m_stitcher.axis()));
                update();
                return;
            }

            m_captureSuspendedForUi = false;
            m_idleSince = 0;
            logScrollDebug("hidden-probe-motion frame=%dx%d axis=%s",
                           frame.width(), frame.height(), axisDebugName(m_stitcher.axis()));
        }

        if (m_captureSuspendedForUi) {
            if (!m_overlayUiVisible) {
                m_nextVisibleDetectAt = now + kStoppedUiDelayMs;
                setOverlayUiVisible(true);
                return;
            }
            if (now < m_nextVisibleDetectAt) {
                return;
            }
            m_hiddenProbePending = true;
            m_resumeCaptureAt = now + kOverlaySettleMs;
            setOverlayUiVisible(false);
            return;
        }

        if (m_overlayUiVisible) {
            m_resumeCaptureAt = now + kOverlaySettleMs;
            setOverlayUiVisible(false);
            return;
        }
        if (now < m_resumeCaptureAt) {
            return;
        }
    } else if (!m_overlayUiVisible) {
        m_captureSuspendedForUi = false;
        m_hiddenProbePending = false;
        m_idleSince = 0;
        m_resumeCaptureAt = 0;
        m_nextVisibleDetectAt = 0;
        setOverlayUiVisible(true);
    }

    if (frame.isNull() && !captureFrame("normal")) {
        return;
    }

    const QVector<std::uint8_t> signature = frameSignature(frame, kSignatureCols, kSignatureRows);
    if (!m_lastSignature.isEmpty() && isDuplicateSignature(m_lastSignature, signature)) {
        m_statusText = MS_TR("Waiting for scroll");
        if (m_autoHideOverlayUi) {
            if (m_idleSince == 0) {
                m_idleSince = now;
            }
            if (now - m_idleSince >= kStoppedUiDelayMs) {
                m_captureSuspendedForUi = true;
                m_nextVisibleDetectAt = now + kStoppedUiDelayMs;
                setOverlayUiVisible(true);
                logScrollDebug("auto-ui-suspend frame=%dx%d idle_ms=%lld excluded=%dx%d axis=%s",
                               frame.width(), frame.height(), now - m_idleSince,
                               overlayUiFrameRect(frame).width(),
                               overlayUiFrameRect(frame).height(),
                               axisDebugName(m_stitcher.axis()));
            }
        }
        logScrollDebug("skip-duplicate frame=%dx%d idle_ms=%lld axis=%s",
                       frame.width(), frame.height(), m_idleSince > 0 ? now - m_idleSince : 0,
                       axisDebugName(m_stitcher.axis()));
        update();
        return;
    }
    m_idleSince = 0;
    m_lastSignature = signature;

    const StitchResult outcome = m_stitcher.pushFrame(frame);
    const StitchStats stats = m_stitcher.stats();

    switch (outcome.status) {
    case StitchStatus::FirstFrame:
        m_statusText = MS_TR("Capturing");
        break;
    case StitchStatus::Appended:
        m_lastAppend = outcome.added;
        m_statusText = MS_TR("Height %1 px").arg(stats.totalHeight);
        break;
    case StitchStatus::NoProgress:
        m_statusText = MS_TR("Waiting for scroll");
        break;
    case StitchStatus::NoMatch:
        m_statusText = MS_TR("No overlap match");
        break;
    }
    if (outcome.frameLength > 0) {
        m_capturePos = outcome.position;
        m_captureLen = outcome.frameLength;
    }
    syncScrubber(outcome);
    refreshControlLabels();
    logScrollDebug("tick status=%s edge=%s added=%d pos=%d frame_len=%d full_len=%d frames=%d "
                   "scrub=%d following=%d axis=%s",
                   statusDebugName(outcome.status), edgeDebugName(outcome.edge), outcome.added,
                   outcome.position, outcome.frameLength, stats.totalHeight, stats.frameCount,
                   m_scrubPos, m_following ? 1 : 0, axisDebugName(m_stitcher.axis()));
    update();
}

void ScrollSessionWindow::togglePause()
{
    m_paused = !m_paused;
    if (m_paused) {
        m_captureSuspendedForUi = false;
        m_hiddenProbePending = false;
        setOverlayUiVisible(true);
    }
    if (!m_paused) {
        m_lastSignature.clear();
        if (m_autoHideOverlayUi) {
            const qint64 now = QDateTime::currentMSecsSinceEpoch();
            m_resumeCaptureAt = now + kOverlaySettleMs;
            setOverlayUiVisible(false);
        }
    }
    refreshControlLabels();
    update();
}

void ScrollSessionWindow::toggleAlgorithm()
{
    const StitchAlgorithm next = m_stitcher.algorithm() == StitchAlgorithm::OpenCvOrb
        ? StitchAlgorithm::ColSample
        : StitchAlgorithm::OpenCvOrb;
    m_stitcher.setAlgorithm(next);
    // Force the next frame through, since the anchor's match basis changed.
    m_lastSignature.clear();
    refreshControlLabels();
    update();
}

void ScrollSessionWindow::toggleAxis()
{
    // The axis locks after the first frame; this is a no-op then (the button is
    // also disabled), so only the pre-capture toggle has any effect.
    const ScrollAxis next = m_stitcher.axis() == ScrollAxis::Horizontal
        ? ScrollAxis::Vertical
        : ScrollAxis::Horizontal;
    m_stitcher.setAxis(next);
    refreshControlLabels();
    update();
}

QRect ScrollSessionWindow::imageAreaRect() const
{
    const QRect panel = previewPanelRect();
    const int left = panel.left() + kPanelPadding;
    const int top = panel.top() + kPanelPadding + kStatusHeight + kPanelPadding;
    const int width = panel.width() - kPanelPadding * 2;
    // Below the image area sit (in order) the scrubber and the control bar, each
    // separated by a padding gap from the image and from the panel edge.
    const int height = panel.height() - kStatusHeight - kControlBarHeight
                       - kScrubberHeight - kPanelPadding * 4;
    return QRect(left, top, width, std::max(0, height));
}

ScrollSessionWindow::PreviewLayout ScrollSessionWindow::computePreviewLayout() const
{
    PreviewLayout layout;
    const QImage result = currentResult();
    const QRect area = imageAreaRect();
    if (result.isNull() || area.width() <= 0 || area.height() <= 0
        || result.width() <= 0 || result.height() <= 0) {
        return layout;
    }
    layout.valid = true;

    const bool horizontal = m_stitcher.axis() == ScrollAxis::Horizontal;
    constexpr int gap = 10;

    if (!horizontal) {
        // Vertical: the detail view is scaled to the column width; once the image
        // is taller than the area, the whole-image thumbnail appears to the right
        // (80/20 split).
        const qreal fullScale = static_cast<qreal>(area.width()) / result.width();
        layout.overflow =
            static_cast<int>(std::lround(result.height() * fullScale)) > area.height();
        if (!layout.overflow) {
            layout.detailRect = area;
        } else {
            const int leftW = (area.width() - gap) * 8 / 10;
            const int rightW = area.width() - gap - leftW;
            layout.detailRect = QRect(area.left(), area.top(), leftW, area.height());
            layout.globalRect = QRect(area.left() + leftW + gap, area.top(), rightW, area.height());
        }
        layout.detailScale = static_cast<qreal>(layout.detailRect.width()) / result.width();
        layout.longLen = result.height();
    } else {
        // Horizontal: the detail view is scaled to the column height; once the
        // image is wider than the area, the thumbnail appears below (80/20 split).
        const qreal fullScale = static_cast<qreal>(area.height()) / result.height();
        layout.overflow =
            static_cast<int>(std::lround(result.width() * fullScale)) > area.width();
        if (!layout.overflow) {
            layout.detailRect = area;
        } else {
            const int topH = (area.height() - gap) * 8 / 10;
            const int botH = area.height() - gap - topH;
            layout.detailRect = QRect(area.left(), area.top(), area.width(), topH);
            layout.globalRect = QRect(area.left(), area.top() + topH + gap, area.width(), botH);
        }
        layout.detailScale = static_cast<qreal>(layout.detailRect.height()) / result.height();
        layout.longLen = result.width();
    }

    const int detailLongPx = horizontal ? layout.detailRect.width() : layout.detailRect.height();
    layout.viewportLen = std::min(layout.longLen,
        std::max(1, static_cast<int>(std::lround(detailLongPx / layout.detailScale))));
    layout.maxScrub = std::max(0, layout.longLen - layout.viewportLen);
    return layout;
}

void ScrollSessionWindow::syncScrubber(const StitchResult &outcome)
{
    if (!m_scrubber) {
        return;
    }
    const PreviewLayout layout = computePreviewLayout();
    const int maxScrub = layout.maxScrub;

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

    const QSignalBlocker blocker(m_scrubber);
    m_scrubber->setRange(0, maxScrub);
    m_scrubber->setValue(m_scrubPos);
    m_scrubber->setEnabled(maxScrub > 0);
}

void ScrollSessionWindow::refreshControlLabels()
{
    if (m_pauseButton) {
        m_pauseButton->setText(m_paused ? MS_TR("Resume") : MS_TR("Pause"));
    }
    if (m_algorithmButton) {
        const bool orb = m_stitcher.algorithm() == StitchAlgorithm::OpenCvOrb;
        // Label shows the algorithm currently in use; clicking switches it.
        m_algorithmButton->setText(orb ? MS_TR("Algo: ORB") : MS_TR("Algo: Fast"));
        m_algorithmButton->setEnabled(openCvAvailable());
    }
    if (m_axisButton) {
        const bool horizontal = m_stitcher.axis() == ScrollAxis::Horizontal;
        m_axisButton->setText(horizontal ? MS_TR("Dir: Horizontal") : MS_TR("Dir: Vertical"));
        // Direction stays switchable until the first directional stitch lands;
        // capturing the idle seed frame alone does not commit an orientation.
        m_axisButton->setEnabled(!m_stitcher.axisLocked());
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
    if (m_blinkTimer) {
        m_blinkTimer->stop();
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
    });
    connect(dialog, &QFileDialog::rejected, this, [this] { m_paused = false; });
    dialog->open();
}

void ScrollSessionWindow::copyResult()
{
    const QImage result = currentResult();
    if (result.isNull()) {
        return;
    }

    // Qt's own clipboard owner is destroyed when this window closes, which on
    // Wayland drops the clipboard contents. Hand the image to a detached
    // wl-copy (xclip on X11) so it survives independently. Mirrors
    // ShotWindow::copySelection.
    if (QClipboard *clipboard = QApplication::clipboard()) {
        clipboard->setImage(result);
    }

    QByteArray png;
    QBuffer buffer(&png);
    buffer.open(QIODevice::WriteOnly);
    result.save(&buffer, "PNG");
    buffer.close();

    const bool isWayland = QProcessEnvironment::systemEnvironment()
                               .value(QStringLiteral("XDG_SESSION_TYPE"))
                               .toLower()
        == QStringLiteral("wayland");

    QTemporaryFile tempFile(QDir::tempPath() + QStringLiteral("/mark-shot-clipboard-XXXXXX.png"));
    tempFile.setAutoRemove(false);
    if (tempFile.open()) {
        tempFile.write(png);
        const QString tempPath = tempFile.fileName();
        tempFile.close();

        if (isWayland) {
            QProcess::startDetached(
                QStringLiteral("sh"),
                {QStringLiteral("-c"),
                 QStringLiteral("wl-copy --foreground --type image/png < \"$1\"; rm -f \"$1\""),
                 QStringLiteral("mark-shot-clipboard"),
                 tempPath});
        } else {
            QProcess::startDetached(
                QStringLiteral("sh"),
                {QStringLiteral("-c"),
                 QStringLiteral("xclip -selection clipboard -t image/png < \"$1\"; rm -f \"$1\""),
                 QStringLiteral("mark-shot-clipboard"),
                 tempPath});
        }
    }

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

    // 1. Blinking capture border, drawn just outside the captured region so grim
    //    never records it.
    const QRect region = regionLocalRect();
    const QRect borderRect = region.adjusted(-(kBorderGap + kBorderWidth / 2),
                                             -(kBorderGap + kBorderWidth / 2),
                                             kBorderGap + kBorderWidth / 2,
                                             kBorderGap + kBorderWidth / 2);
    const QColor borderColor = m_paused
        ? QColor(250, 204, 21, 235)                       // amber while paused
        : (m_blinkOn ? QColor(45, 212, 191, 255) : QColor(45, 212, 191, 90));
    painter.setPen(QPen(borderColor, kBorderWidth));
    painter.setBrush(Qt::NoBrush);
    painter.drawRect(borderRect);

    if (!m_overlayUiVisible) {
        painter.setCompositionMode(QPainter::CompositionMode_Clear);
        painter.fillRect(previewPanelRect().adjusted(-6, -6, 6, 6), Qt::transparent);
        painter.setCompositionMode(QPainter::CompositionMode_SourceOver);
        return;
    }

    // 2. Preview panel.
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

    // Preview image area: between the status row and the scrubber/control bar.
    const QRect imageArea = imageAreaRect();
    if (imageArea.height() > 10) {
        painter.fillRect(imageArea, QColor(8, 13, 19, 220));

        const PreviewLayout layout = computePreviewLayout();
        if (layout.valid) {
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

            // Detail view: shows the window [m_scrubPos, m_scrubPos + viewportLen)
            // of the long image along the scroll axis, scaled to the detail rect.
            // Following tracks the current captured frame; the scrubber moves it
            // without ever discarding stitched content.
            {
                const QRect &rect = layout.detailRect;
                const int pos = std::clamp(m_scrubPos, 0, layout.maxScrub);
                QRect srcRect;
                QRect target;
                if (horizontal) {
                    const int srcW = std::min(layout.viewportLen, result.width());
                    srcRect = QRect(pos, 0, srcW, result.height());
                    const int tw = std::min(rect.width(),
                        std::max(1, static_cast<int>(std::lround(srcW * layout.detailScale))));
                    target = QRect(rect.left(), rect.top(), tw, rect.height());
                } else {
                    const int srcH = std::min(layout.viewportLen, result.height());
                    srcRect = QRect(0, pos, result.width(), srcH);
                    const int th = std::min(rect.height(),
                        std::max(1, static_cast<int>(std::lround(srcH * layout.detailScale))));
                    target = QRect(rect.left(), rect.top(), rect.width(), th);
                }
                painter.drawImage(target, result, srcRect);
                painter.setPen(QPen(QColor(255, 255, 255, 40), 1));
                painter.setBrush(Qt::NoBrush);
                painter.drawRect(target);
            }

            // Global view: the whole stitched image (contain), with a teal marker
            // boxing the window the detail view currently shows, plus an amber
            // marker for the current screen selection range.
            if (!layout.globalRect.isEmpty()) {
                const QRect &rect = layout.globalRect;
                const QSize fitted = result.size().scaled(rect.size(), Qt::KeepAspectRatio);
                const QRect target(rect.left() + (rect.width() - fitted.width()) / 2,
                                   rect.top() + (rect.height() - fitted.height()) / 2,
                                   fitted.width(),
                                   fitted.height());
                painter.drawImage(target, result);
                painter.setPen(QPen(QColor(255, 255, 255, 40), 1));
                painter.setBrush(Qt::NoBrush);
                painter.drawRect(target);

                const int pos = std::clamp(m_scrubPos, 0, layout.maxScrub);
                QRect marker;
                if (horizontal) {
                    const qreal gscale = static_cast<qreal>(target.width()) / result.width();
                    const int mx = target.left() + static_cast<int>(std::lround(pos * gscale));
                    const int mw =
                        std::max(2, static_cast<int>(std::lround(layout.viewportLen * gscale)));
                    marker = QRect(mx, target.top(), mw, target.height());
                } else {
                    const qreal gscale = static_cast<qreal>(target.height()) / result.height();
                    const int my = target.top() + static_cast<int>(std::lround(pos * gscale));
                    const int mh =
                        std::max(2, static_cast<int>(std::lround(layout.viewportLen * gscale)));
                    marker = QRect(target.left(), my, target.width(), mh);
                }
                painter.setPen(QPen(QColor(45, 212, 191, 235), 2));
                painter.setBrush(QColor(45, 212, 191, 45));
                painter.drawRect(marker.intersected(target));
                drawCaptureMarker(target,
                                  0,
                                  horizontal ? result.width() : result.height(),
                                  true);
            }
        }
    }
}

void ScrollSessionWindow::keyPressEvent(QKeyEvent *event)
{
    if (event->key() == Qt::Key_Escape) {
        close();
        event->accept();
        return;
    }
    QWidget::keyPressEvent(event);
}

void ScrollSessionWindow::closeEvent(QCloseEvent *event)
{
    if (m_timer) {
        m_timer->stop();
    }
    if (m_blinkTimer) {
        m_blinkTimer->stop();
    }
    event->accept();
}

}  // namespace markshot::scroll
