#include "scroll/scroll_session_window.h"

#include "annotation_launch.h"
#include "debug_log.h"
#include "screen_capture.h"
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
#include <QIcon>
#include <QKeyEvent>
#include <QKeySequence>
#include <QPainter>
#include <QPainterPath>
#include <QPaintEvent>
#include <QPixmap>
#include <QProcess>
#include <QProcessEnvironment>
#include <QPushButton>
#include <QRegion>
#include <QResizeEvent>
#include <QScreen>
#include <QShortcut>
#include <QSize>
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
constexpr int kSignatureCols = 18;
constexpr int kSignatureRows = 24;
constexpr float kDuplicateAvgDiff = 1.1f;
constexpr int kDuplicateMaxDiff = 4;

constexpr int kBorderWidth = 3;       // thickness of the capture frame
constexpr int kBorderGap = 10;        // gap between region and border (kept outside)
constexpr int kPanelWidth = 340;      // preview panel width
constexpr int kPanelGap = 14;         // gap between region and preview panel
constexpr int kNoLayerPanelGap = 96;  // GNOME xdg windows can bleed into PipeWire captures
constexpr int kPanelPadding = 12;
constexpr int kControlBarHeight = 54;   // single row of icon actions
constexpr int kStatusHeight = 22;
constexpr int kScrubberHeight = 22;     // non-destructive position slider

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

const char *algorithmDebugName()
{
    return "col-sample";
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

enum class ControlIcon {
    AxisVertical,
    AxisHorizontal,
    Pause,
    Resume,
    Annotate,
    Save,
    Copy,
    Cancel,
};

QPen iconPen(const QColor &color, qreal width = 1.8)
{
    return QPen(color, width, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin);
}

QIcon makeControlIcon(ControlIcon icon)
{
    constexpr int size = 32;
    const QColor ink(229, 231, 235);
    const QColor soft(229, 231, 235, 130);
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
        p.setBrush(QColor(229, 231, 235, 70));
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

}  // namespace

ScrollSessionWindow::ScrollSessionWindow(QRect globalGeometry,
                                         QString outputName,
                                         QScreen *screen,
                                         QWidget *parent)
    : QWidget(parent)
    , m_geometry(globalGeometry)
    , m_outputName(std::move(outputName))
    , m_sessionId(QDateTime::currentMSecsSinceEpoch())
    , m_stitcher(defaultConfig())
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

    logScrollDebug("start time=%lld geom=%d,%d %dx%d output=%s algorithm=%s",
                   m_sessionId,
                   m_geometry.x(), m_geometry.y(), m_geometry.width(), m_geometry.height(),
                   m_outputName.toUtf8().constData(), algorithmDebugName());
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
    // Do not hold exclusive keyboard focus; the page below should keep receiving
    // keyboard and wheel input while the panel can still take focus when clicked.
    layerWindow->setKeyboardInteractivity(LayerShellQt::Window::KeyboardInteractivityOnDemand);
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

    auto *row = new QHBoxLayout(m_controlBar);
    row->setContentsMargins(7, 7, 7, 7);
    row->setSpacing(7);

    auto makeButton = [this, row](const QIcon &icon, const QString &label, const QString &role = QString()) {
        auto *button = new QPushButton(m_controlBar);
        button->setProperty("role", role.isEmpty() ? QStringLiteral("secondary") : role);
        configureIconButton(button, icon, label);
        button->setFixedSize(QSize(40, 36));
        button->setFocusPolicy(Qt::NoFocus);
        button->setCursor(Qt::PointingHandCursor);
        button->setStyleSheet(QStringLiteral(
            "QPushButton {"
            " color: #E5E7EB; background: rgba(255,255,255,16);"
            " border: 1px solid rgba(255,255,255,24); border-radius: 10px;"
            " padding: 0; min-width: 40px; max-width: 40px;"
            " min-height: 36px; max-height: 36px; }"
            "QPushButton:hover { background: rgba(45,212,191,30);"
            " border-color: rgba(45,212,191,90); }"
            "QPushButton[role=\"primary\"] { background: rgba(45,212,191,92);"
            " border-color: rgba(94,234,212,150); }"
            "QPushButton[role=\"primary\"]:hover { background: rgba(45,212,191,126);"
            " border-color: rgba(153,246,228,190); }"
            "QPushButton[role=\"danger\"]:hover { background: rgba(248,113,113,42);"
            " border-color: rgba(248,113,113,105); }"
            "QPushButton:focus { border-color: rgba(94,234,212,180); }"
            "QPushButton:disabled { color: rgba(229,231,235,90);"
            " background: rgba(255,255,255,8); border-color: rgba(255,255,255,14); }"));
        row->addWidget(button, 0, Qt::AlignCenter);
        return button;
    };

    m_axisButton = makeButton(makeControlIcon(ControlIcon::AxisVertical), MS_TR("Dir: Vertical"));
    row->addSpacing(4);
    m_pauseButton = makeButton(makeControlIcon(ControlIcon::Pause), MS_TR("Pause"));
    m_annotateButton = makeButton(makeControlIcon(ControlIcon::Annotate), MS_TR("Annotate"));
    m_saveButton = makeButton(makeControlIcon(ControlIcon::Save), MS_TR("Save"), QStringLiteral("primary"));
    m_copyButton = makeButton(makeControlIcon(ControlIcon::Copy), MS_TR("Copy"));
    m_cancelButton = makeButton(makeControlIcon(ControlIcon::Cancel), MS_TR("Cancel"), QStringLiteral("danger"));

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
    const int panelGap = m_layerShell ? kPanelGap : kNoLayerPanelGap;
    int left = region.right() + panelGap;
    if (left + kPanelWidth > bounds.right() - 4) {
        const int leftAlt = region.left() - panelGap - kPanelWidth;
        if (leftAlt >= 4) {
            left = leftAlt;
        } else {
            left = std::max(4, bounds.right() - kPanelWidth - 4);
        }
    }
    left = std::clamp(left, 4, std::max(4, bounds.width() - kPanelWidth - 4));

    return QRect(left, top, kPanelWidth, panelHeight);
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
}

void ScrollSessionWindow::updateInputMask()
{
    // Only the border ring and the preview panel should catch input;
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
    mask += QRegion(previewPanelRect());
    setMask(mask);
    if (QWindow *nativeWindow = windowHandle()) {
        nativeWindow->setMask(mask);
    }
}

void ScrollSessionWindow::showEvent(QShowEvent *event)
{
    QWidget::showEvent(event);
    if (!m_layerShell) {
        m_screenOrigin = geometry().topLeft();
    }
    layoutOverlay();
    updateInputMask();
    const QRect region = regionLocalRect();
    const QRect panel = previewPanelRect();
    logScrollDebug("layout window=%d,%d %dx%d screen_origin=%d,%d region=%d,%d %dx%d "
                   "panel=%d,%d %dx%d panel_gap=%d panel_overlap=%d layer_shell=%d",
                   geometry().x(), geometry().y(), geometry().width(), geometry().height(),
                   m_screenOrigin.x(), m_screenOrigin.y(),
                   region.x(), region.y(), region.width(), region.height(),
                   panel.x(), panel.y(), panel.width(), panel.height(),
                   m_layerShell ? kPanelGap : kNoLayerPanelGap,
                   panel.intersects(region) ? 1 : 0, m_layerShell ? 1 : 0);
    if (m_timer && !m_timer->isActive()) {
        // Let the compositor apply the window mask before the first X11 capture;
        // otherwise the seed frame can include the scroll overlay itself.
        QTimer::singleShot(120, this, [this] {
            if (m_timer && !m_timer->isActive() && !m_paused) {
                m_timer->start();
            }
        });
    }
    raise();
    activateWindow();
    setFocus(Qt::ActiveWindowFocusReason);
}

void ScrollSessionWindow::resizeEvent(QResizeEvent *event)
{
    QWidget::resizeEvent(event);
    layoutOverlay();
    updateInputMask();
}

void ScrollSessionWindow::captureTick()
{
    if (m_paused) {
        return;
    }

    QImage frame;
    auto captureFrame = [&](const char *debugTag) {
        CaptureRequest request;
        request.preferredOutputName = m_outputName;
        request.sourceGeometry = m_geometry;
        request.allOutputs = false;
        request.preferScreencast = true;
        request.allowInteractivePortal = false;
        request.allowPortalScreenshotFallback = false;

        const CaptureResult result = captureScreenFrame(request);
        if (result.image.isNull()) {
            m_paused = true;
            stopActiveScreencastCapture();
            m_statusText = MS_TR("Capture error");
            refreshControlLabels();
            logScrollDebug("%s-capture-error geom=%d,%d %dx%d output=%s error=%s",
                           debugTag,
                           request.sourceGeometry.x(), request.sourceGeometry.y(),
                           request.sourceGeometry.width(), request.sourceGeometry.height(),
                           request.preferredOutputName.toUtf8().constData(),
                           result.error.toUtf8().constData());
            update();
            return false;
        }

        Q_UNUSED(debugTag);
        frame = result.image.convertToFormat(QImage::Format_ARGB32_Premultiplied);
        return true;
    };

    if (frame.isNull() && !captureFrame("normal")) {
        return;
    }

    const QVector<std::uint8_t> signature = frameSignature(frame, kSignatureCols, kSignatureRows);
    if (!m_lastSignature.isEmpty() && isDuplicateSignature(m_lastSignature, signature)) {
        m_statusText = MS_TR("Waiting for scroll");
        logScrollDebug("skip-duplicate frame=%dx%d axis=%s",
                       frame.width(), frame.height(),
                       axisDebugName(m_stitcher.axis()));
        update();
        return;
    }
    m_lastSignature = signature;
    dumpDebugFrame(frame, "candidate");

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

void ScrollSessionWindow::dumpDebugFrame(const QImage &frame, const char *tag)
{
    if (!markshot::debugEnabled() || frame.isNull() || m_debugFrameDumpCount >= 12) {
        return;
    }

    const QString safeTag = QString::fromLatin1(tag ? tag : "frame").remove(QLatin1Char('/'));
    const QString path = QDir::temp().filePath(
        QStringLiteral("mark-shot-scroll-%1-%2-%3.png")
            .arg(m_sessionId)
            .arg(m_debugFrameDumpCount, 2, 10, QLatin1Char('0'))
            .arg(safeTag));
    const bool saved = frame.save(path, "PNG");
    logScrollDebug("dump-frame index=%d saved=%d path=%s frame=%dx%d",
                   m_debugFrameDumpCount, saved ? 1 : 0,
                   path.toUtf8().constData(), frame.width(), frame.height());
    ++m_debugFrameDumpCount;
}

void ScrollSessionWindow::togglePause()
{
    m_paused = !m_paused;
    if (!m_paused) {
        m_lastSignature.clear();
    }
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
        configureIconButton(m_pauseButton,
                            makeControlIcon(m_paused ? ControlIcon::Resume : ControlIcon::Pause),
                            m_paused ? MS_TR("Resume") : MS_TR("Pause"));
    }
    if (m_axisButton) {
        const bool horizontal = m_stitcher.axis() == ScrollAxis::Horizontal;
        configureIconButton(m_axisButton,
                            makeControlIcon(horizontal ? ControlIcon::AxisHorizontal : ControlIcon::AxisVertical),
                            horizontal ? MS_TR("Dir: Horizontal") : MS_TR("Dir: Vertical"));
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

    // 1. Capture border, drawn just outside the captured region so grim
    //    never records it.
    const QRect region = regionLocalRect();
    const QRect borderRect = region.adjusted(-(kBorderGap + kBorderWidth / 2),
                                             -(kBorderGap + kBorderWidth / 2),
                                             kBorderGap + kBorderWidth / 2,
                                             kBorderGap + kBorderWidth / 2);
    const QColor borderColor = m_paused
        ? QColor(250, 204, 21, 235)                       // amber while paused
        : QColor(45, 212, 191, 255);
    painter.setPen(QPen(borderColor, kBorderWidth));
    painter.setBrush(Qt::NoBrush);
    painter.drawRect(borderRect);

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
    stopActiveScreencastCapture();
    event->accept();
}

}  // namespace markshot::scroll
