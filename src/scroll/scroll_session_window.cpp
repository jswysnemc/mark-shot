#include "scroll/scroll_session_window.h"

#include "annotation_launch.h"
#include "clipboard_image.h"
#include "debug_log.h"
#include "layer_shell_runtime.h"
#include "screen_capture.h"
#include "ui/i18n.h"
#include "ui/theme.h"

#include <QCloseEvent>
#include <QCoreApplication>
#include <QDateTime>
#include <QDBusConnection>
#include <QDBusMessage>
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
#include <utility>

namespace markshot::scroll {

namespace {

constexpr int kCaptureIntervalMs = 45;
constexpr int kSignatureCols = 18;
constexpr int kSignatureRows = 24;
constexpr float kDuplicateAvgDiff = 1.1f;
constexpr int kDuplicateMaxDiff = 4;

constexpr int kCaptureFrameWidth = 3; // thickness of the outer capture frame
constexpr int kPanelWidth = 340;      // preview panel width
constexpr int kPanelPadding = 12;
constexpr int kControlBarHeight = 54;   // single row of icon actions
constexpr int kStatusHeight = 22;
constexpr int kHidePreviewButtonSize = 22;
constexpr int kPanelMargin = 4;
constexpr int kGnomePreviewIntervalMs = 140;

constexpr auto kGnomeShellService = "org.gnome.Shell";
constexpr auto kGnomeHelperPath = "/org/gnome/Shell/Extensions/MarkShotScrollHelper";
constexpr auto kGnomeHelperInterface = "org.gnome.Shell.Extensions.MarkShotScrollHelper";

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

enum class ControlIcon {
    AxisVertical,
    AxisHorizontal,
    Pause,
    Resume,
    Annotate,
    Save,
    Copy,
    HidePreview,
    Cancel,
};

struct PreviewPanelPlacement {
    QRect rect;
    bool fitsWithoutOverlap = true;
};

QPen iconPen(const QColor &color, qreal width = 1.8)
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
    case ControlIcon::HidePreview: {
        QPainterPath eye;
        eye.moveTo(6.5, 16);
        eye.cubicTo(9.0, 10.5, 23.0, 10.5, 25.5, 16);
        eye.cubicTo(23.0, 21.5, 9.0, 21.5, 6.5, 16);
        p.setPen(iconPen(ink, 1.5));
        p.drawPath(eye);
        p.drawEllipse(QPointF(16, 16), 3.0, 3.0);
        p.setPen(iconPen(ink, 2.0));
        p.drawLine(QPointF(8.5, 23.5), QPointF(23.5, 8.5));
        break;
    }
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
    setObjectName(QStringLiteral("scrollSessionWindow"));
    setMouseTracking(true);
    setFocusPolicy(Qt::NoFocus);

    auto *cancelShortcut = new QShortcut(QKeySequence(Qt::Key_Escape), this);
    cancelShortcut->setContext(Qt::ApplicationShortcut);
    connect(cancelShortcut, &QShortcut::activated, this, [this] { close(); });
    auto *togglePreviewShortcut = new QShortcut(QKeySequence(Qt::Key_V), this);
    togglePreviewShortcut->setContext(Qt::ApplicationShortcut);
    connect(togglePreviewShortcut, &QShortcut::activated, this, [this] { togglePreviewPanel(); });

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
    if (m_gnomeShellPreview) {
        QDBusConnection::sessionBus().connect(QString::fromLatin1(kGnomeShellService),
                                              QString::fromLatin1(kGnomeHelperPath),
                                              QString::fromLatin1(kGnomeHelperInterface),
                                              QStringLiteral("PreviewAction"),
                                              this,
                                              SLOT(handleGnomePreviewAction(QString,QString)));
    }

    m_layerShell = configureLayerShell(screen);
    m_panelOnlyWindow = !m_layerShell && isWaylandPlatform();
    if (!m_layerShell && screen) {
        setScreen(screen);
        syncPreviewPanelDefaultVisibility();
        if (m_panelOnlyWindow) {
            updatePanelWindowGeometry();
        } else {
            setGeometry(screen->geometry());
        }
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
    m_axisButton->installEventFilter(this);
    row->addSpacing(4);
    m_pauseButton = makeButton(makeControlIcon(ControlIcon::Pause), MS_TR("Pause"));
    m_annotateButton = makeButton(makeControlIcon(ControlIcon::Annotate), MS_TR("Annotate"));
    m_saveButton = makeButton(makeControlIcon(ControlIcon::Save), MS_TR("Save"), QStringLiteral("primary"));
    m_copyButton = makeButton(makeControlIcon(ControlIcon::Copy), MS_TR("Copy"));
    m_cancelButton = makeButton(makeControlIcon(ControlIcon::Cancel), MS_TR("Cancel"), QStringLiteral("danger"));

    m_hidePreviewButton = new QPushButton(this);
    m_hidePreviewButton->setProperty("role", QStringLiteral("secondary"));
    configureIconButton(m_hidePreviewButton,
                        makeControlIcon(ControlIcon::HidePreview),
                        MS_TR("Hide Preview"));
    m_hidePreviewButton->setIconSize(QSize(16, 16));
    m_hidePreviewButton->setFixedSize(QSize(kHidePreviewButtonSize, kHidePreviewButtonSize));
    m_hidePreviewButton->setFocusPolicy(Qt::NoFocus);
    m_hidePreviewButton->setCursor(Qt::PointingHandCursor);
    m_hidePreviewButton->setStyleSheet(QStringLiteral(
        "QPushButton {"
        " color: #E5E7EB; background: rgba(255,255,255,16);"
        " border: 1px solid rgba(255,255,255,24); border-radius: 7px;"
        " padding: 0; min-width: 22px; max-width: 22px;"
        " min-height: 22px; max-height: 22px; }"
        "QPushButton:hover { background: rgba(45,212,191,30);"
        " border-color: rgba(45,212,191,90); }"
        "QPushButton:focus { border-color: rgba(94,234,212,180); }"));
    m_hidePreviewButton->hide();

    connect(m_axisButton, &QPushButton::clicked, this, [this] { toggleAxis(); });
    connect(m_pauseButton, &QPushButton::clicked, this, [this] { togglePause(); });
    connect(m_annotateButton, &QPushButton::clicked, this, [this] { annotateResult(); });
    connect(m_saveButton, &QPushButton::clicked, this, [this] { saveResult(); });
    connect(m_copyButton, &QPushButton::clicked, this, [this] { copyResult(); });
    connect(m_hidePreviewButton, &QPushButton::clicked, this, [this] {
        setPreviewPanelVisible(false, true);
    });
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
    const QRect panel = m_previewPanelVisible
        ? floatingPanelGlobalRect()
        : QRect(hiddenPoint, QSize(1, 1));
    setWindowOpacity(m_previewPanelVisible ? 1.0 : 0.0);
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
        return m_previewPanelVisible ? QRegion(bounds) : QRegion();
    }

    constexpr int kAntialiasPad = 2;
    QRegion painted;
    if (m_uiConfig.frameEnabled) {
        const QRect region = regionLocalRect();
        painted += QRegion(captureFrameOuterRect(region, m_uiConfig.frameGap)
                               .adjusted(-kAntialiasPad,
                                         -kAntialiasPad,
                                         kAntialiasPad,
                                         kAntialiasPad));
        painted -= QRegion(captureFrameInnerRect(region, m_uiConfig.frameGap)
                               .adjusted(kAntialiasPad,
                                         kAntialiasPad,
                                         -kAntialiasPad,
                                         -kAntialiasPad));
    }
    if (m_previewPanelVisible) {
        painted += QRegion(previewPanelRect().adjusted(-kAntialiasPad,
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
    syncPreviewPanelDefaultVisibility();
    if (m_panelOnlyWindow && !m_axisDragging) {
        updatePanelWindowGeometry();
    }
    layoutOverlay();
    m_transientPaintMask += oldPaint;
    m_transientPaintMask += overlayPaintRegion();
    updateInputMask();
    if (!m_transientPaintMask.isEmpty()) {
        update(m_transientPaintMask);
    } else {
        update();
    }
    m_gnomePreviewImageDirty = true;
    updateGnomeShellPreview(true);
}

void ScrollSessionWindow::layoutOverlay()
{
    if (m_layerShell && !m_gnomeShellPreview) {
        setWindowOpacity(m_previewPanelVisible ? 1.0 : 0.0);
    }

    if (m_gnomeShellPreview) {
        if (m_controlBar) {
            m_controlBar->hide();
        }
        if (m_hidePreviewButton) {
            m_hidePreviewButton->hide();
        }
        return;
    }

    if (!m_previewPanelVisible) {
        if (m_controlBar) {
            m_controlBar->hide();
        }
        if (m_hidePreviewButton) {
            m_hidePreviewButton->hide();
        }
        return;
    }

    const QRect panel = previewPanelRect();
    if (m_controlBar && !m_controlBar->isVisible() && !m_panelTransparentForCapture) {
        m_controlBar->show();
    }
    const int barWidth = panel.width() - kPanelPadding * 2;
    const int barTop = panel.bottom() - kPanelPadding - kControlBarHeight;
    m_controlBar->setGeometry(panel.left() + kPanelPadding,
                              barTop,
                              barWidth,
                              kControlBarHeight);
    if (m_hidePreviewButton && !m_panelTransparentForCapture) {
        m_hidePreviewButton->show();
        m_hidePreviewButton->setGeometry(panel.right() - kPanelPadding - kHidePreviewButtonSize + 1,
                                         panel.top() + kPanelPadding,
                                         kHidePreviewButtonSize,
                                         kHidePreviewButtonSize);
    }
}

void ScrollSessionWindow::updateInputMask()
{
    if (m_gnomeShellPreview) {
        const QRegion emptyMask;
        setMask(emptyMask);
        if (QWindow *nativeWindow = windowHandle()) {
            nativeWindow->setMask(emptyMask);
        }
        return;
    }

    if (m_panelOnlyWindow) {
        const QRegion mask = m_previewPanelVisible ? QRegion(rect()) : QRegion();
        setMask(mask);
        if (QWindow *nativeWindow = windowHandle()) {
            nativeWindow->setMask(mask);
        }
        return;
    }

    // Only the preview panel should catch input; the captured region
    // interior and the rest of the overlay stay click-through so the user
    // can keep scrolling the page underneath. The axis button drag-to-
    // translate is handled via an event filter on the button itself. During
    // a drag, old paint locations stay in the mask until one cleanup paint
    // has been submitted.
    QRegion mask;
    if (m_previewPanelVisible) {
        mask += QRegion(previewPanelRect());
    }
    mask += m_transientPaintMask;
    setMask(mask);
    if (QWindow *nativeWindow = windowHandle()) {
        nativeWindow->setMask(mask);
    }
}

void ScrollSessionWindow::showEvent(QShowEvent *event)
{
    QWidget::showEvent(event);
    syncPreviewPanelDefaultVisibility();
    if (m_panelOnlyWindow) {
        updatePanelWindowGeometry();
    } else if (!m_layerShell) {
        m_screenOrigin = geometry().topLeft();
    }
    layoutOverlay();
    updateInputMask();
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
}

void ScrollSessionWindow::resizeEvent(QResizeEvent *event)
{
    QWidget::resizeEvent(event);
    const QRegion oldPaint = overlayPaintRegion();
    syncPreviewPanelDefaultVisibility();
    layoutOverlay();
    updateInputMask();
    const QRegion repaintRegion = oldPaint + overlayPaintRegion();
    if (!repaintRegion.isEmpty()) {
        update(repaintRegion);
    }
}

void ScrollSessionWindow::captureTick()
{
    if (m_paused || (m_panelOnlyWindow && m_axisDragging)) {
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

        const bool makePanelTransparentForCapture =
            !m_gnomeShellPreview && m_panelOnlyWindow && m_previewPanelVisible && isVisible();
        if (makePanelTransparentForCapture) {
            m_panelTransparentForCapture = true;
            if (m_controlBar) {
                m_controlBar->hide();
            }
            if (m_hidePreviewButton) {
                m_hidePreviewButton->hide();
            }
            repaint(overlayPaintRegion());
            QCoreApplication::processEvents(QEventLoop::ExcludeUserInputEvents);
            request.minimumFrameTimeMs = QDateTime::currentMSecsSinceEpoch() + 1;
        }
        const CaptureResult result = captureScreenFrame(request);
        if (makePanelTransparentForCapture) {
            m_panelTransparentForCapture = false;
            if (m_controlBar || m_hidePreviewButton) {
                layoutOverlay();
            }
            repaint(overlayPaintRegion());
            QCoreApplication::processEvents(QEventLoop::ExcludeUserInputEvents);
        }
        if (result.image.isNull()) {
            m_paused = true;
            stopActiveScreencastCapture();
            m_statusText = MS_TR("Capture error");
            refreshControlLabels();
            updateGnomeShellPreview(true);
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
        updateGnomeShellPreview();
        update();
        return;
    }
    m_lastSignature = signature;
    dumpDebugFrame(frame, "candidate");

    const StitchResult outcome = m_stitcher.pushFrame(frame);
    const StitchStats stats = m_stitcher.stats();
    const int oldCapturePos = m_capturePos;
    const int oldCaptureLen = m_captureLen;

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
    if (outcome.status == StitchStatus::FirstFrame
        || outcome.status == StitchStatus::Appended
        || oldCapturePos != m_capturePos
        || oldCaptureLen != m_captureLen) {
        m_gnomePreviewImageDirty = true;
    }
    syncPreviewScroll(outcome);
    refreshControlLabels();
    updateGnomeShellPreview();
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
    m_previewPanelPausedCapture = false;
    if (!m_paused) {
        m_lastSignature.clear();
    }
    refreshControlLabels();
    updateGnomeShellPreview(true);
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
    m_gnomePreviewImageDirty = true;
    updateInputMask();
    refreshControlLabels();
    updateGnomeShellPreview(true);
    update();
}

void ScrollSessionWindow::togglePreviewPanel()
{
    setPreviewPanelVisible(!m_previewPanelVisible, true);
}

void ScrollSessionWindow::setPreviewPanelVisible(bool visible, bool userRequested)
{
    if (userRequested) {
        m_previewPanelUserSet = true;
    }

    bool captureStateChanged = false;
    if (visible && !previewPanelFitsAvailableSpace() && !m_paused) {
        m_paused = true;
        m_previewPanelPausedCapture = true;
        captureStateChanged = true;
        m_lastSignature.clear();
        refreshControlLabels();
    } else if (!visible && m_previewPanelPausedCapture) {
        m_previewPanelPausedCapture = false;
        if (m_paused) {
            m_paused = false;
            captureStateChanged = true;
            m_lastSignature.clear();
            refreshControlLabels();
            if (m_timer && !m_timer->isActive()) {
                m_timer->start();
            }
        }
    }

    if (m_previewPanelVisible == visible) {
        if (captureStateChanged) {
            updateGnomeShellPreview(true);
            update();
        }
        return;
    }

    const QRegion oldPaint = overlayPaintRegion();
    m_previewPanelVisible = visible;
    if (m_panelOnlyWindow) {
        updatePanelWindowGeometry();
    }
    layoutOverlay();
    updateInputMask();
    updateGnomeShellPreview(true);
    const QRegion repaintRegion = oldPaint + overlayPaintRegion();
    if (!repaintRegion.isEmpty()) {
        update(repaintRegion);
    } else {
        update();
    }
}

void ScrollSessionWindow::syncPreviewPanelDefaultVisibility()
{
    if (m_previewPanelUserSet) {
        return;
    }
    m_previewPanelVisible = previewPanelFitsAvailableSpace();
}

QRect ScrollSessionWindow::imageAreaRect() const
{
    const QRect panel = previewPanelRect();
    const int left = panel.left() + kPanelPadding;
    const int top = panel.top() + kPanelPadding + kStatusHeight + kPanelPadding;
    const int width = panel.width() - kPanelPadding * 2;
    // Below the image area sits the control bar, separated by a padding gap from
    // the image and from the panel edge.
    const int height = panel.height() - kStatusHeight - kControlBarHeight - kPanelPadding * 4;
    return QRect(left, top, width, std::max(0, height));
}

ScrollSessionWindow::PreviewLayout ScrollSessionWindow::computePreviewLayout() const
{
    return computePreviewLayout(imageAreaRect());
}

ScrollSessionWindow::PreviewLayout ScrollSessionWindow::computePreviewLayout(const QRect &area) const
{
    PreviewLayout layout;
    const QImage result = currentResult();
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
        return;
    }

    if (m_panelTransparentForCapture) {
        return;
    }

    if (m_uiConfig.frameEnabled && !m_panelOnlyWindow) {
        const QRect region = regionLocalRect();
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
    }

    if (m_previewPanelVisible) {
        const QRect panel = previewPanelRect();
        QPainterPath panelPath;
        panelPath.addRoundedRect(panel, 12, 12);
        painter.fillPath(panelPath, QColor(15, 17, 23, 242));
        painter.setPen(QPen(QColor(255, 255, 255, 28), 1));
        painter.drawPath(panelPath);

        // Status text along the top of the panel.
        const int statusRightInset = m_hidePreviewButton && m_hidePreviewButton->isVisible()
            ? kPanelPadding + kHidePreviewButtonSize + 6
            : kPanelPadding;
        const QRect statusRect(panel.left() + kPanelPadding,
                               panel.top() + kPanelPadding,
                               panel.width() - kPanelPadding - statusRightInset,
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
    if (event->key() == Qt::Key_V && event->modifiers() == Qt::NoModifier) {
        togglePreviewPanel();
        event->accept();
        return;
    }
    QWidget::keyPressEvent(event);
}

void ScrollSessionWindow::mousePressEvent(QMouseEvent *event)
{
    if (event->button() == Qt::LeftButton && beginOverviewDrag(event->pos())) {
        event->accept();
        return;
    }
    QWidget::mousePressEvent(event);
}

void ScrollSessionWindow::mouseMoveEvent(QMouseEvent *event)
{
    if (m_overviewDragging) {
        updateOverviewDrag(event->pos());
        event->accept();
        return;
    }
    QWidget::mouseMoveEvent(event);
}

void ScrollSessionWindow::mouseReleaseEvent(QMouseEvent *event)
{
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
    if (watched != m_axisButton) {
        return QWidget::eventFilter(watched, event);
    }

    switch (event->type()) {
    case QEvent::MouseButtonPress: {
        auto *me = static_cast<QMouseEvent *>(event);
        if (me->button() == Qt::LeftButton) {
            m_axisDragArmed = true;
            m_axisDragging = false;
            m_axisDragStartGlobal = me->globalPosition().toPoint();
            m_axisDragStartGeometry = m_geometry.normalized();
        }
        // Let the button see the press so it can still emit clicked on release.
        return false;
    }

    case QEvent::MouseMove: {
        auto *me = static_cast<QMouseEvent *>(event);
        if (!m_axisDragArmed || !(me->buttons() & Qt::LeftButton)) {
            return false;
        }

        constexpr int kDragThresholdPx = 5;
        const QPoint globalPos = me->globalPosition().toPoint();
        const QPoint delta = globalPos - m_axisDragStartGlobal;

        if (!m_axisDragging) {
            if (std::abs(delta.x()) < kDragThresholdPx &&
                std::abs(delta.y()) < kDragThresholdPx) {
                return false;  // below threshold, not yet dragging
            }
            // Threshold exceeded: enter drag mode. In the GNOME xdg-window
            // fallback, pause capture while the region moves so intermediate
            // drag frames do not disturb the stitcher's current anchor. On
            // layer-shell compositors, live stitching during drag is preserved.
            m_axisDragging = true;
            m_lastSignature.clear();
            m_transientPaintMask = overlayPaintRegion();
            m_restoreMaskAfterPaint = false;
            m_statusText = MS_TR("Dragging region");
            refreshControlLabels();
            setCursor(Qt::ClosedHandCursor);
        }

        // Constrain movement to the current scroll axis only.
        const bool horizontal = m_stitcher.axis() == ScrollAxis::Horizontal;
        QPoint constrainedDelta = delta;
        if (horizontal) {
            constrainedDelta.setY(0);
        } else {
            constrainedDelta.setX(0);
        }

        QRect next = m_axisDragStartGeometry.translated(constrainedDelta);
        // Clamp to screen bounds.
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
        return true;  // consume the move so the button doesn't highlight
    }

    case QEvent::MouseButtonRelease: {
        auto *me = static_cast<QMouseEvent *>(event);
        if (me->button() != Qt::LeftButton || !m_axisDragArmed) {
            return false;
        }

        const bool wasDragging = m_axisDragging;
        m_axisDragArmed = false;
        m_axisDragging = false;
        unsetCursor();

        if (!wasDragging) {
            // Treat as a plain click: let the button fire its clicked signal.
            return false;
        }

        // Drag just finished: clear the frame signature so the next capture
        // tick is accepted as a fresh frame at the adjusted region.
        m_lastSignature.clear();
        m_statusText = MS_TR("Region adjusted");
        refreshControlLabels();
        updatePanelWindowGeometry();
        m_transientPaintMask += overlayPaintRegion();
        m_restoreMaskAfterPaint = !m_transientPaintMask.isEmpty();
        // Keep the temporary paint mask active for one cleanup repaint. If it
        // is dropped immediately, coalesced mouse-move damage can leave old
        // frame or panel pixels on the compositor surface.
        updateInputMask();
        if (!m_transientPaintMask.isEmpty()) {
            update(m_transientPaintMask);
        } else {
            update();
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
    if (!gnomeShellPreviewActive()) {
        return;
    }
    if (!m_previewPanelVisible) {
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
    if (force || m_gnomePreviewImageDirty) {
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
            << std::max(0, m_uiConfig.previewGap);
    QDBusConnection::sessionBus().call(message, QDBus::NoBlock);
    m_gnomePreviewVisible = true;
}

void ScrollSessionWindow::hideGnomeShellPreview()
{
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
    } else if (action == QStringLiteral("hide")) {
        setPreviewPanelVisible(false, true);
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
    hideGnomeShellPreview();
    if (m_gnomeShellPreview) {
        QDBusConnection::sessionBus().disconnect(QString::fromLatin1(kGnomeShellService),
                                                 QString::fromLatin1(kGnomeHelperPath),
                                                 QString::fromLatin1(kGnomeHelperInterface),
                                                 QStringLiteral("PreviewAction"),
                                                 this,
                                                 SLOT(handleGnomePreviewAction(QString,QString)));
    }
    stopActiveScreencastCapture();
    event->accept();
}

}  // namespace markshot::scroll
