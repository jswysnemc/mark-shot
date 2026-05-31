#include "scroll/scroll_session_window.h"

#include "annotation_launch.h"
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
#include <QPainter>
#include <QPainterPath>
#include <QPaintEvent>
#include <QProcess>
#include <QProcessEnvironment>
#include <QPushButton>
#include <QRegion>
#include <QResizeEvent>
#include <QScreen>
#include <QShowEvent>
#include <QStandardPaths>
#include <QTemporaryFile>
#include <QTimer>
#include <QWindow>

#include <algorithm>
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
constexpr int kControlBarHeight = 40;
constexpr int kStatusHeight = 22;

// A coarse grayscale grid used to tell whether the latest capture is the same
// as the previous one (the user has not scrolled yet). Mirrors wayscrollshot's
// frame_signature.
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
            const QRgb px = frame.pixel(x, y);
            const int gray =
                static_cast<int>(0.299 * qRed(px) + 0.587 * qGreen(px) + 0.114 * qBlue(px));
            signature.push_back(static_cast<std::uint8_t>(gray));
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
    // OnDemand keyboard focus lets the control buttons work while leaving the
    // page underneath scrollable through the click-through region.
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

    auto *controls = new QHBoxLayout(m_controlBar);
    controls->setContentsMargins(6, 6, 6, 6);
    controls->setSpacing(4);

    auto makeButton = [this, controls](const QString &text) {
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
            " border-color: rgba(45,212,191,90); }"));
        controls->addWidget(button);
        return button;
    };

    m_pauseButton = makeButton(MS_TR("Pause"));
    m_annotateButton = makeButton(MS_TR("Annotate"));
    m_saveButton = makeButton(MS_TR("Save"));
    m_copyButton = makeButton(MS_TR("Copy"));
    m_cancelButton = makeButton(MS_TR("Cancel"));

    connect(m_pauseButton, &QPushButton::clicked, this, [this] { togglePause(); });
    connect(m_annotateButton, &QPushButton::clicked, this, [this] { annotateResult(); });
    connect(m_saveButton, &QPushButton::clicked, this, [this] { saveResult(); });
    connect(m_copyButton, &QPushButton::clicked, this, [this] { copyResult(); });
    connect(m_cancelButton, &QPushButton::clicked, this, [this] { close(); });
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

void ScrollSessionWindow::layoutOverlay()
{
    const QRect panel = previewPanelRect();
    const int barWidth = panel.width() - kPanelPadding * 2;
    m_controlBar->setGeometry(panel.left() + kPanelPadding,
                              panel.bottom() - kPanelPadding - kControlBarHeight,
                              barWidth,
                              kControlBarHeight);
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
    mask += QRegion(previewPanelRect());
    setMask(mask);
}

void ScrollSessionWindow::showEvent(QShowEvent *event)
{
    QWidget::showEvent(event);
    layoutOverlay();
    updateInputMask();
    if (m_timer && !m_timer->isActive()) {
        m_timer->start();
    }
    if (m_blinkTimer && !m_blinkTimer->isActive()) {
        m_blinkTimer->start();
    }
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

    CaptureRequest request;
    request.preferredOutputName = m_outputName;
    request.sourceGeometry = m_geometry;
    request.allOutputs = false;

    const CaptureResult result = captureScreenFrame(request);
    if (result.image.isNull()) {
        m_statusText = MS_TR("Capture error");
        update();
        return;
    }

    const QImage frame = result.image.convertToFormat(QImage::Format_ARGB32_Premultiplied);

    const QVector<std::uint8_t> signature = frameSignature(frame, kSignatureCols, kSignatureRows);
    if (!m_lastSignature.isEmpty() && isDuplicateSignature(m_lastSignature, signature)) {
        m_statusText = MS_TR("Waiting for scroll");
        update();
        return;
    }
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
    update();
}

void ScrollSessionWindow::togglePause()
{
    m_paused = !m_paused;
    m_pauseButton->setText(m_paused ? MS_TR("Resume") : MS_TR("Pause"));
    if (!m_paused) {
        m_lastSignature.clear();
    }
    update();
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

    // Preview image area: between the status row and the control bar.
    const QRect imageArea(panel.left() + kPanelPadding,
                          statusRect.bottom() + kPanelPadding,
                          panel.width() - kPanelPadding * 2,
                          panel.height() - kStatusHeight - kControlBarHeight
                              - kPanelPadding * 4);
    if (imageArea.height() > 10) {
        painter.fillRect(imageArea, QColor(8, 13, 19, 220));

        const QImage result = currentResult();
        if (!result.isNull()) {
            const int regionH = m_geometry.height();

            // Teal frame highlighting the live viewport (the bottom region-height
            // slice of the stitched image), in whatever target rect it appears.
            auto drawRangeMarker = [&](const QRect &target, qreal scale) {
                const int markerHeight =
                    std::max(2, static_cast<int>(std::lround(regionH * scale)));
                const QRect marker(target.left(),
                                   target.bottom() - markerHeight + 1,
                                   target.width(),
                                   markerHeight);
                painter.setPen(QPen(QColor(45, 212, 191, 235), 2));
                painter.setBrush(QColor(45, 212, 191, 45));
                painter.drawRect(marker.intersected(target));
            };

            // Detail view: scale the stitched image to the column width and show
            // its bottom, discarding any overflow off the top. The scale stays
            // constant as the image grows, so recent content stays readable.
            auto drawDetailView = [&](const QRect &rect) {
                const qreal scale =
                    static_cast<qreal>(rect.width()) / std::max(1, result.width());
                const int scaledHeight =
                    std::max(1, static_cast<int>(std::lround(result.height() * scale)));
                QRect target;
                if (scaledHeight <= rect.height()) {
                    target = QRect(rect.left(), rect.bottom() - scaledHeight + 1,
                                   rect.width(), scaledHeight);
                    painter.drawImage(target, result);
                } else {
                    const int srcCropH =
                        std::max(1, static_cast<int>(std::lround(rect.height() / scale)));
                    const int srcY = std::max(0, result.height() - srcCropH);
                    const QRect srcRect(0, srcY, result.width(), result.height() - srcY);
                    target = rect;
                    painter.drawImage(target, result, srcRect);
                }
                painter.setPen(QPen(QColor(255, 255, 255, 40), 1));
                painter.setBrush(Qt::NoBrush);
                painter.drawRect(target);
                drawRangeMarker(target, scale);
            };

            // Global view: the whole stitched image fit inside the column
            // (contain), so the overall captured proportion is visible.
            auto drawGlobalView = [&](const QRect &rect) {
                const QSize fitted = result.size().scaled(rect.size(), Qt::KeepAspectRatio);
                const QRect target(rect.left() + (rect.width() - fitted.width()) / 2,
                                   rect.top() + (rect.height() - fitted.height()) / 2,
                                   fitted.width(),
                                   fitted.height());
                painter.drawImage(target, result);
                painter.setPen(QPen(QColor(255, 255, 255, 40), 1));
                painter.setBrush(Qt::NoBrush);
                painter.drawRect(target);
                const qreal scale =
                    static_cast<qreal>(target.height()) / std::max(1, result.height());
                drawRangeMarker(target, scale);
            };

            // The global thumbnail only appears once the image, scaled to the
            // full width, no longer fits the height (it has become a tall strip).
            const qreal fullScale =
                static_cast<qreal>(imageArea.width()) / std::max(1, result.width());
            const bool overflow =
                static_cast<int>(std::lround(result.height() * fullScale)) > imageArea.height();

            if (!overflow) {
                drawDetailView(imageArea);
            } else {
                const int gap = 10;
                const int leftW = (imageArea.width() - gap) * 8 / 10;
                const int rightW = imageArea.width() - gap - leftW;
                drawDetailView(QRect(imageArea.left(), imageArea.top(),
                                     leftW, imageArea.height()));
                drawGlobalView(QRect(imageArea.left() + leftW + gap, imageArea.top(),
                                     rightW, imageArea.height()));
            }
        }
    }
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
