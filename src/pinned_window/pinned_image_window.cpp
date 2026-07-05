#include "pinned_window/pinned_image_window.h"

#include "app_config_store.h"
#include "clipboard_image.h"
#include "debug_log.h"
#include "notifications/app_notifications.h"
#include "pinned_window_top.h"
#include "translation_language_options.h"
#include "ui/i18n.h"

#include <QAction>
#include <QApplication>
#include <QContextMenuEvent>
#include <QCursor>
#include <QDateTime>
#include <QDir>
#include <QEvent>
#include <QFileDialog>
#include <QKeyEvent>
#include <QKeySequence>
#include <QMenu>
#include <QMouseEvent>
#include <QPainter>
#include <QPen>
#include <QScreen>
#include <QTimer>
#include <QTransform>
#include <QWheelEvent>
#include <QWindow>

#include <algorithm>
#include <cmath>
#include <utility>

namespace {

/// @brief 拦截菜单上的非左键鼠标事件。
class LeftClickMenuFilter final : public QObject {
public:
    /// @brief 创建菜单事件过滤器。
    /// @param parent 父对象。
    explicit LeftClickMenuFilter(QObject *parent = nullptr) : QObject(parent) {}

protected:
    /// @brief 过滤菜单鼠标事件。
    /// @param obj 事件目标对象。
    /// @param event Qt 事件。
    /// @return 拦截事件时返回 true。
    bool eventFilter(QObject *obj, QEvent *event) override
    {
        if (event->type() == QEvent::MouseButtonPress
            || event->type() == QEvent::MouseButtonRelease
            || event->type() == QEvent::MouseButtonDblClick) {
            auto *mouseEvent = static_cast<QMouseEvent *>(event);
            if (mouseEvent->button() != Qt::LeftButton) {
                QWidget *widget = qobject_cast<QWidget *>(obj);
                if (widget && widget->rect().contains(mouseEvent->position().toPoint())) {
                    return true;
                }
            }
        }
        return QObject::eventFilter(obj, event);
    }
};

/**
 * 将贴图窗口初始位置限制在目标屏幕可用区域内。
 * @param desiredTopLeft 期望的全局左上角。
 * @param windowSize 贴图窗口尺寸。
 * @param screen 目标屏幕。
 * @return 修正后的全局左上角。
 */
QPoint clampedInitialTopLeft(QPoint desiredTopLeft, QSize windowSize, QScreen *screen)
{
    if (!screen || windowSize.isEmpty()) {
        return desiredTopLeft;
    }

    const QRect available = screen->availableGeometry();
    const int maxX = available.left() + std::max(0, available.width() - windowSize.width());
    const int maxY = available.top() + std::max(0, available.height() - windowSize.height());
    return QPoint(std::clamp(desiredTopLeft.x(), available.left(), maxX),
                  std::clamp(desiredTopLeft.y(), available.top(), maxY));
}

}  // namespace

namespace markshot::shot {

PinnedImageWindow::PinnedImageWindow(QImage image, std::optional<QPoint> initialTopLeft)
    : m_pixmap(QPixmap::fromImage(std::move(image)))
    , m_imageSize(m_pixmap.size())
    , m_displayBaseSize(displayBaseSizeForPixmap())
    , m_config(pinnedWindowConfig())
{
    setWindowTitle(MS_TR("Pinned Mark Shot"));
    setAttribute(Qt::WA_DeleteOnClose);
    setAttribute(Qt::WA_ShowWithoutActivating);
    Qt::WindowFlags flags = Qt::Window | Qt::FramelessWindowHint;
    if (m_config.alwaysOnTop) {
        flags |= Qt::WindowStaysOnTopHint;
    }
    setWindowFlags(flags);
    setFocusPolicy(Qt::StrongFocus);
    setMouseTracking(true);
    setCursor(Qt::OpenHandCursor);
    QSize targetSize = m_displayBaseSize;
    QScreen *screen = initialTopLeft.has_value()
        ? QGuiApplication::screenAt(*initialTopLeft)
        : QGuiApplication::screenAt(QCursor::pos());
    if (!screen) {
        screen = QApplication::primaryScreen();
    }
    if (screen) {
        const QSize maxSize = screen->availableGeometry().size() * 0.9;
        if (targetSize.width() > maxSize.width() || targetSize.height() > maxSize.height()) {
            targetSize.scale(maxSize, Qt::KeepAspectRatio);
        }
        m_scale = static_cast<qreal>(targetSize.width()) / std::max(1, m_displayBaseSize.width());
        setFixedSize(targetSize);
        const QPoint centeredTopLeft = screen->availableGeometry().center() - rect().center();
        move(clampedInitialTopLeft(initialTopLeft.value_or(centeredTopLeft), targetSize, screen));
    } else {
        setFixedSize(targetSize);
        if (initialTopLeft.has_value()) {
            move(*initialTopLeft);
        }
    }
    m_logicalGeometry = QRect(pos(), size());
    setProperty("markShotPinnedGeometry", m_logicalGeometry);
    applyPinnedWindowTopState(this, m_config.alwaysOnTop);
    if (pinnedWindowHasLayerShellTop(this)) {
        setPinnedGeometry(m_logicalGeometry, false);
    }
    if (m_config.autoOcr) {
        QTimer::singleShot(0, this, [this] { startOcr(); });
    }
}

PinnedImageWindow::~PinnedImageWindow()
{
    cancelTranslation();
    cancelOcr();
}

bool PinnedImageWindow::event(QEvent *event)
{
    const bool shouldRaise = event->type() == QEvent::WindowDeactivate
        || event->type() == QEvent::ActivationChange
        || event->type() == QEvent::Show;
    const bool handled = QWidget::event(event);
    if (shouldRaise && m_config.alwaysOnTop) {
        schedulePinnedWindowRaise();
    }
    return handled;
}

void PinnedImageWindow::paintEvent(QPaintEvent *)
{
    QPainter painter(this);
    painter.setRenderHint(QPainter::SmoothPixmapTransform, true);
    const QRectF imageRect = displayedImageRect();
    painter.drawPixmap(imageRect, m_pixmap, QRectF(QPointF(0.0, 0.0), QSizeF(m_pixmap.size())));
    if (m_translationActive) {
        drawTranslationOverlay(painter);
    }

    auto drawBorder = [this, &painter, &imageRect] {
        if (!m_config.borderEnabled || !m_config.borderColor.isValid() || m_config.borderWidth <= 0.0) {
            return;
        }
        painter.save();
        painter.setRenderHint(QPainter::Antialiasing, false);
        painter.setBrush(Qt::NoBrush);
        painter.setPen(QPen(m_config.borderColor, m_config.borderWidth));
        const qreal inset = m_config.borderWidth / 2.0;
        painter.drawRect(imageRect.adjusted(inset, inset, -inset, -inset));
        painter.restore();
    };

    if (!hasTextSelection()) {
        drawBorder();
        return;
    }

    painter.setRenderHint(QPainter::Antialiasing, false);
    painter.setPen(Qt::NoPen);
    painter.setBrush(QColor(72, 132, 245, 96));
    const auto [first, last] = selectionRange();
    const QVector<OcrToken> &tokens = activeTokens();
    for (int i = first; i <= last; ++i) {
        painter.drawRect(imageToWidget(tokens.at(i).imageRect).intersected(QRectF(rect())));
    }
    drawBorder();
}

void PinnedImageWindow::mousePressEvent(QMouseEvent *event)
{
    if (event->button() == Qt::LeftButton) {
        if (startResizeDrag(event)) {
            event->accept();
            return;
        }

        const std::optional<int> token = m_config.textSelectionCopyEnabled
            ? tokenAt(widgetToImage(event->position()))
            : std::nullopt;
        if (token) {
            m_selectingText = true;
            m_selectionAnchor = *token;
            m_selectionFocus = *token;
            setCursor(Qt::IBeamCursor);
            update();
            event->accept();
            return;
        }

        clearTextSelection();
        m_dragOffset = event->globalPosition().toPoint() - pinnedTopLeft();
        setCursor(Qt::ClosedHandCursor);
        if (m_config.alwaysOnTop && pinnedWindowHasLayerShellTop(this)) {
            event->accept();
            return;
        }
        if (QWindow *window = windowHandle()) {
            if (window->startSystemMove()) {
                event->accept();
                return;
            }
        }
        event->accept();
        return;
    }
    QWidget::mousePressEvent(event);
}

void PinnedImageWindow::mouseMoveEvent(QMouseEvent *event)
{
    if (continueResizeDrag(event)) {
        event->accept();
        return;
    }

    if (m_selectingText) {
        const QPointF imagePoint = widgetToImage(event->position());
        const std::optional<int> token = tokenAt(imagePoint).has_value()
            ? tokenAt(imagePoint)
            : closestToken(imagePoint);
        if (token && m_selectionFocus != *token) {
            m_selectionFocus = *token;
            update();
        }
        event->accept();
        return;
    }

    if (event->buttons().testFlag(Qt::LeftButton)) {
        const QPoint topLeft = event->globalPosition().toPoint() - m_dragOffset;
        if (m_config.alwaysOnTop && pinnedWindowHasLayerShellTop(this)) {
            setPinnedGeometry(QRect(topLeft, logicalPinnedSize()), false);
        } else {
            move(topLeft);
            setPinnedGeometry(QRect(topLeft, size()), true);
        }
        event->accept();
        return;
    }

    updateCursorForPosition(event->position());
    QWidget::mouseMoveEvent(event);
}

void PinnedImageWindow::mouseReleaseEvent(QMouseEvent *event)
{
    if (event->button() == Qt::LeftButton) {
        if (isPinnedResizeDirection(m_resizeDrag.direction)) {
            finishResizeDrag(event->position());
            event->accept();
            return;
        }
        if (m_selectingText) {
            m_selectingText = false;
            updateCursorForPosition(event->position());
            event->accept();
            return;
        }
        updateCursorForPosition(event->position());
        event->accept();
        return;
    }
    QWidget::mouseReleaseEvent(event);
}

void PinnedImageWindow::wheelEvent(QWheelEvent *event)
{
    const QPoint delta = event->angleDelta();
    const QPoint pixelDelta = event->pixelDelta();
    if (delta.y() == 0 && pixelDelta.y() == 0) {
        QWidget::wheelEvent(event);
        return;
    }

    qreal wheelSteps = static_cast<qreal>(delta.y()) / 120.0;
    if (qFuzzyIsNull(wheelSteps) && pixelDelta.y() != 0) {
        wheelSteps = static_cast<qreal>(pixelDelta.y()) / 80.0;
    }
    const qreal factor = std::pow(1.08, wheelSteps);
    resizeByScale(m_scale * factor,
                  logicalGlobalPointForLocalAnchor(event->position(), event->globalPosition().toPoint()),
                  event->position());
    event->accept();
}

void PinnedImageWindow::mouseDoubleClickEvent(QMouseEvent *event)
{
    if (event->button() == Qt::LeftButton) {
        const std::optional<int> token = m_config.textSelectionCopyEnabled
            ? tokenAt(widgetToImage(event->position()))
            : std::nullopt;
        if (token) {
            m_selectionAnchor = *token;
            m_selectionFocus = *token;
            update();
            event->accept();
            return;
        }
        close();
        event->accept();
        return;
    }
    QWidget::mouseDoubleClickEvent(event);
}

void PinnedImageWindow::contextMenuEvent(QContextMenuEvent *event)
{
    QMenu menu(this);
    LeftClickMenuFilter filter(&menu);
    menu.installEventFilter(&filter);
    menu.addAction(MS_TR("Rotate Left"), this, [this] { rotateImage(-90); });
    menu.addAction(MS_TR("Rotate Right"), this, [this] { rotateImage(90); });
    menu.addSeparator();
    menu.addAction(MS_TR("Zoom In"), this, [this, event] {
        resizeByScale(m_scale * 1.18,
                      logicalGlobalPointForLocalAnchor(rect().center(), event->globalPos()),
                      rect().center());
    });
    menu.addAction(MS_TR("Zoom Out"), this, [this, event] {
        resizeByScale(m_scale / 1.18,
                      logicalGlobalPointForLocalAnchor(rect().center(), event->globalPos()),
                      rect().center());
    });
    menu.addAction(MS_TR("Reset Size"), this, [this] {
        const QPointF localCenter(width() / 2.0, height() / 2.0);
        resizeByScale(1.0, logicalGlobalPointForLocalAnchor(localCenter, frameGeometry().center()), localCenter);
    });
    menu.addSeparator();
    QAction *alwaysOnTopAction = menu.addAction(MS_TR("Always on Top"));
    alwaysOnTopAction->setCheckable(true);
    alwaysOnTopAction->setChecked(m_config.alwaysOnTop);
    connect(alwaysOnTopAction, &QAction::toggled, this, [this](bool checked) {
        setAlwaysOnTop(checked);
    });
    menu.addSeparator();
    menu.addAction(MS_TR("Copy"), this, [this] {
        markshot::copyImageToClipboard(m_pixmap.toImage());
    });
    QAction *copySelectedTextAction = menu.addAction(MS_TR("Copy Selected Text"), this, [this] {
        copySelectedText();
    });
    copySelectedTextAction->setEnabled(hasTextSelection());
    QAction *copyTextAction = menu.addAction(MS_TR("Copy Image Text"), this, [this] {
        copyImageText();
    });
    copyTextAction->setEnabled(m_config.ocrEnabled);
    QAction *textSelectionCopyAction = menu.addAction(MS_TR("Text Selection Copy"));
    textSelectionCopyAction->setCheckable(true);
    textSelectionCopyAction->setChecked(m_config.textSelectionCopyEnabled);
    connect(textSelectionCopyAction, &QAction::toggled, this, [this](bool checked) {
        setTextSelectionCopyEnabled(checked);
    });
    QAction *translateAction = menu.addAction(MS_TR("Translate"), this, [this] {
        requestTranslation();
    });
    translateAction->setEnabled(canRequestTranslation());
    QMenu *targetLanguageMenu = menu.addMenu(MS_TR("Target Language"));
    for (const markshot::TranslationLanguageOption &language : markshot::translationLanguageOptions()) {
        QAction *languageAction = targetLanguageMenu->addAction(language.label, this, [this, value = language.value] {
            setTranslationTargetLanguage(value);
        });
        languageAction->setCheckable(true);
        languageAction->setChecked(language.value == m_config.translationTargetLanguage);
    }
    QAction *toggleTranslationAction = menu.addAction(
        m_translationActive ? MS_TR("Show Original Text") : MS_TR("Show Translated Text"),
        this,
        [this] { setTranslationActive(!m_translationActive); });
    toggleTranslationAction->setEnabled(!m_translationOverlayTokens.isEmpty() && !m_translationTask);
    menu.addAction(MS_TR("Save As"), this, [this] { saveImageAs(); });
    menu.addSeparator();
    menu.addAction(MS_TR("Close"), this, &QWidget::close);
    menu.exec(event->globalPos());
}

void PinnedImageWindow::keyPressEvent(QKeyEvent *event)
{
    if (event->matches(QKeySequence::Copy) && hasTextSelection()) {
        copySelectedText();
        event->accept();
        return;
    }
    if (event->key() == Qt::Key_Escape) {
        close();
        return;
    }
    QWidget::keyPressEvent(event);
}

void PinnedImageWindow::rotateImage(qreal degrees)
{
    const QPoint center = frameGeometry().center();
    m_pixmap = m_pixmap.transformed(QTransform().rotate(degrees), Qt::SmoothTransformation);
    m_imageSize = m_pixmap.size();
    m_displayBaseSize = displayBaseSizeForPixmap();
    clearTextSelection();
    m_ocrTokens.clear();
    m_translatedTokens.clear();
    m_translationOverlayTokens.clear();
    m_translationActive = false;
    m_translateAfterOcr = false;
    m_copyTextAfterOcr = false;
    cancelTranslation();
    resizeByScale(m_scale, center, QPointF(width() / 2.0, height() / 2.0));
    update();
    if (m_config.autoOcr) {
        startOcr();
    }
}

void PinnedImageWindow::saveImageAs()
{
    const QString filename = QStringLiteral("mark-shot-pin-%1.png")
                                 .arg(QDateTime::currentDateTime().toString(QStringLiteral("yyyyMMdd-hhmmss")));
    const QString path = QFileDialog::getSaveFileName(this,
                                                      MS_TR("Save Pinned Image"),
                                                      QDir(markShotPicturesDir()).filePath(filename),
                                                      MS_TR("PNG Images (*.png)"));
    if (!path.isEmpty()) {
        if (m_pixmap.save(path, "PNG")) {
            markshot::notifications::notifyScreenshotSaved(path);
        }
    }
}

void PinnedImageWindow::raisePinnedWindow()
{
    if (!m_config.alwaysOnTop) {
        return;
    }
    raisePinnedWindowOnPlatform(this);
}

void PinnedImageWindow::setAlwaysOnTop(bool alwaysOnTop)
{
    if (m_config.alwaysOnTop == alwaysOnTop) {
        return;
    }

    const bool previous = m_config.alwaysOnTop;
    m_config.alwaysOnTop = alwaysOnTop;
    QString error;
    if (!markshot::writeAppConfigValue(
            {QStringLiteral("pinnedWindow"), QStringLiteral("alwaysOnTop")},
            QJsonValue(alwaysOnTop),
            &error)) {
        m_config.alwaysOnTop = previous;
        sendDesktopNotification(QStringLiteral("Mark Shot"),
                                MS_TR("Failed to save pinned window setting."));
        if (!error.isEmpty()) {
            markshot::debugLog("config",
                               "cannot save pinnedWindow.alwaysOnTop: %s",
                               error.toUtf8().constData());
        }
        return;
    }

    if (pinnedWindowHasLayerShellTop(this) || (alwaysOnTop && pinnedWindowUsesLayerShellTop())) {
        recreateWithCurrentImage();
        return;
    }

    applyPinnedWindowTopState(this, m_config.alwaysOnTop);
    if (m_config.alwaysOnTop) {
        schedulePinnedWindowRaise();
    }
}

void PinnedImageWindow::setTextSelectionCopyEnabled(bool enabled)
{
    if (m_config.textSelectionCopyEnabled == enabled) {
        return;
    }

    QString error;
    if (!markshot::writeAppConfigValue(
            {QStringLiteral("pinnedWindow"), QStringLiteral("textSelectionCopyEnabled")},
            QJsonValue(enabled),
            &error)) {
        sendDesktopNotification(QStringLiteral("Mark Shot"),
                                MS_TR("Failed to save pinned text selection setting."));
        if (!error.isEmpty()) {
            markshot::debugLog("config",
                               "【置顶图片】【拖选复制】cannot save pinnedWindow.textSelectionCopyEnabled: %s",
                               error.toUtf8().constData());
        }
        return;
    }

    m_config.textSelectionCopyEnabled = enabled;
    if (!enabled) {
        clearTextSelection();
    }
    updateCursorForPosition(mapFromGlobal(QCursor::pos()));
}

/// @brief 设置并保存翻译目标语言
/// @param targetLanguage 实际传给翻译器的目标语言名称
void PinnedImageWindow::setTranslationTargetLanguage(QString targetLanguage)
{
    targetLanguage = markshot::translationLanguageValueFromText(targetLanguage).trimmed();
    if (targetLanguage.isEmpty() || targetLanguage == m_config.translationTargetLanguage) {
        return;
    }

    const QString previousTargetLanguage = m_config.translationTargetLanguage;
    m_config.translationTargetLanguage = targetLanguage;

    QString error;
    if (!markshot::saveTranslationTargetLanguage(targetLanguage, &error)) {
        m_config.translationTargetLanguage = previousTargetLanguage;
        if (!error.isEmpty()) {
            markshot::debugLog("config",
                               "cannot save translation.targetLanguage: %s",
                               error.toUtf8().constData());
        }
        return;
    }

    // 目标语言变化后旧译文不再可信,清空缓存并等待用户再次请求翻译
    cancelTranslation();
    m_translationOverlayTokens.clear();
    m_translatedTokens.clear();
    m_translationActive = false;
    update();
}

void PinnedImageWindow::recreateWithCurrentImage()
{
    if (m_recreating) {
        return;
    }
    m_recreating = true;
    auto *replacement = new PinnedImageWindow(m_pixmap.toImage());
    replacement->restorePinnedState(m_logicalGeometry, m_scale);
    replacement->show();
    if (replacement->m_config.alwaysOnTop) {
        replacement->schedulePinnedWindowRaise();
    }
    close();
}

void PinnedImageWindow::restorePinnedState(QRect geometry, qreal scale)
{
    if (!geometry.isValid() || geometry.isEmpty()) {
        return;
    }
    m_scale = scale;
    setFixedSize(geometry.size());
    setPinnedGeometry(geometry, true);
    applyPinnedWindowTopState(this, m_config.alwaysOnTop);
    if (pinnedWindowHasLayerShellTop(this)) {
        setPinnedGeometry(m_logicalGeometry, false);
    }
}

void PinnedImageWindow::schedulePinnedWindowRaise()
{
    for (int delayMs : {0, 80, 250, 600}) {
        QTimer::singleShot(delayMs, this, [this] {
            if (m_config.alwaysOnTop) {
                raisePinnedWindow();
            }
        });
    }
}

}  // namespace markshot::shot
