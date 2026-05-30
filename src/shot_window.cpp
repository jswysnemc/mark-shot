#include "shot_window.h"

#include "screen_capture.h"
#include "ui/color_picker.h"
#include "ui/i18n.h"
#include "ui/icons.h"
#include "ui/theme.h"

#ifdef HAVE_LAYER_SHELL
#include <LayerShellQt/Window>
#endif

#include <QAbstractItemView>
#include <QAction>
#include <QApplication>
#include <QBuffer>
#include <QBoxLayout>
#include <QBrush>
#include <QClipboard>
#include <QContextMenuEvent>
#include <QCoreApplication>
#include <QCursor>
#include <QDateTime>
#include <QDir>
#include <QDirIterator>
#include <QEvent>
#include <QFile>
#include <QFileDialog>
#include <QFileInfo>
#include <QFontDatabase>
#include <QFrame>
#include <QHBoxLayout>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>
#include <QJsonValue>
#include <QKeyEvent>
#include <QLabel>
#include <QLayout>
#include <QLineF>
#include <QListWidget>
#include <QLineEdit>
#include <QMenu>
#include <QMouseEvent>
#include <QPainter>
#include <QPainterPath>
#include <QPixmap>
#include <QProcess>
#include <QProcessEnvironment>
#include <QPushButton>
#include <QScreen>
#include <QShortcut>
#include <QSignalBlocker>
#include <QSlider>
#include <QStandardPaths>
#include <QStyle>
#include <QTemporaryFile>
#include <QTextEdit>
#include <QTextBlock>
#include <QTextDocument>
#include <QTextLayout>
#include <QTextOption>
#include <QTimer>
#include <QTransform>
#include <QUrl>
#include <QVBoxLayout>
#include <QWindow>
#include <QWheelEvent>

#include <algorithm>
#include <cmath>
#include <limits>
#include <utility>

namespace {

constexpr qreal kMinSelectionSize = 8.0;
constexpr qreal kToolbarMargin = 12.0;
constexpr qreal kMinStrokeWidth = 1.0;
constexpr qreal kMaxStrokeWidth = 24.0;
constexpr qreal kMinNumberWidth = 1.0;
constexpr qreal kMaxNumberWidth = 72.0;
constexpr qreal kMinMosaicBlockSize = 4.0;
constexpr qreal kMaxMosaicBlockSize = 48.0;
constexpr qreal kMinLaserWidth = 4.0;
constexpr qreal kMaxLaserWidth = 48.0;
constexpr qint64 kLaserLifetimeMs = 1800;
constexpr qreal kTextBackgroundPaddingX = 6.0;
constexpr qreal kTextBackgroundPaddingY = 4.0;
constexpr qreal kMinImageZoom = 0.25;
constexpr qreal kMaxImageZoom = 8.0;
constexpr qint64 kCtrlDoubleTapMs = 360;
constexpr int kPinnedOcrTimeoutMs = 30000;
constexpr int kPinnedTranslationTimeoutMs = 60000;

QRectF normalizedRect(QPointF a, QPointF b)
{
    return QRectF(a, b).normalized();
}

qreal imageNavigationWheelFactor(const QWheelEvent *event)
{
    const QPoint angleDelta = event->angleDelta();
    if (angleDelta.y() != 0) {
        return std::pow(1.12, static_cast<qreal>(angleDelta.y()) / 120.0);
    }

    const QPoint pixelDelta = event->pixelDelta();
    if (pixelDelta.y() != 0) {
        return std::pow(1.12, static_cast<qreal>(pixelDelta.y()) / 80.0);
    }

    return 1.0;
}

// Stylesheets, palette presets, action names, and toolbar icons now live in
// src/ui/theme.{h,cpp} and src/ui/icons.{h,cpp}.

QString desktopEntryValue(const QStringList &lines, const QString &key)
{
    bool inDesktopEntry = false;
    const QString prefix = key + QLatin1Char('=');
    for (const QString &line : lines) {
        const QString trimmed = line.trimmed();
        if (trimmed.isEmpty() || trimmed.startsWith(QLatin1Char('#'))) {
            continue;
        }
        if (trimmed.startsWith(QLatin1Char('[')) && trimmed.endsWith(QLatin1Char(']'))) {
            inDesktopEntry = trimmed == QStringLiteral("[Desktop Entry]");
            continue;
        }
        if (inDesktopEntry && trimmed.startsWith(prefix)) {
            return trimmed.mid(prefix.size()).trimmed();
        }
    }
    return {};
}

bool desktopEntryBool(const QStringList &lines, const QString &key)
{
    const QString value = desktopEntryValue(lines, key).toLower();
    return value == QStringLiteral("true") || value == QStringLiteral("1");
}

bool desktopEntrySupportsImage(const QStringList &lines)
{
    const QStringList mimeTypes = desktopEntryValue(lines, QStringLiteral("MimeType"))
        .split(QLatin1Char(';'), Qt::SkipEmptyParts);
    for (const QString &mimeType : mimeTypes) {
        const QString normalized = mimeType.trimmed().toLower();
        if (normalized == QStringLiteral("image/png") || normalized == QStringLiteral("image/*")
            || normalized.startsWith(QStringLiteral("image/"))) {
            return true;
        }
    }
    return false;
}

QStringList desktopSearchDirs()
{
    QStringList dataDirs;
    dataDirs << QDir::home().filePath(QStringLiteral(".local/share"));

    const QString envDataDirs = QProcessEnvironment::systemEnvironment().value(
        QStringLiteral("XDG_DATA_DIRS"),
        QStringLiteral("/usr/local/share:/usr/share"));
    dataDirs << envDataDirs.split(QLatin1Char(':'), Qt::SkipEmptyParts);
    dataDirs.removeDuplicates();

    QStringList appDirs;
    for (const QString &dataDir : dataDirs) {
        appDirs << QDir(dataDir).filePath(QStringLiteral("applications"));
    }
    appDirs.removeDuplicates();
    return appDirs;
}

QString markShotPicturesDir()
{
    QString pictures = QStandardPaths::writableLocation(QStandardPaths::PicturesLocation);
    if (pictures.isEmpty()) {
        pictures = QDir::homePath();
    }

    const QString markShotDir = QDir(pictures).filePath(QStringLiteral("mark-shot"));
    QDir dir(markShotDir);
    if (dir.exists() || dir.mkpath(QStringLiteral("."))) {
        return markShotDir;
    }
    return pictures;
}

QString markShotConfigDir()
{
    QString configDir = QStandardPaths::writableLocation(QStandardPaths::AppConfigLocation);
    if (!configDir.isEmpty()) {
        return configDir;
    }

    const QString genericConfigDir = QStandardPaths::writableLocation(QStandardPaths::GenericConfigLocation);
    if (!genericConfigDir.isEmpty()) {
        return QDir(genericConfigDir).filePath(QStringLiteral("mark-shot"));
    }
    return QDir::home().filePath(QStringLiteral(".config/mark-shot"));
}

QString extensionCommandsConfigPath()
{
    return QDir(markShotConfigDir()).filePath(QStringLiteral("extensions.json"));
}

QString appConfigPath()
{
    return QDir(markShotConfigDir()).filePath(QStringLiteral("config.json"));
}

QString markShotDataDir()
{
    QString dataDir = QStandardPaths::writableLocation(QStandardPaths::AppDataLocation);
    if (!dataDir.isEmpty()) {
        return dataDir;
    }

    const QString genericDataDir = QStandardPaths::writableLocation(QStandardPaths::GenericDataLocation);
    if (!genericDataDir.isEmpty()) {
        return QDir(genericDataDir).filePath(QStringLiteral("mark-shot"));
    }
    return QDir::home().filePath(QStringLiteral(".local/share/mark-shot"));
}

QString slurpGeometry(QRect geometry)
{
    geometry = geometry.normalized();
    return QStringLiteral("%1,%2 %3x%4")
        .arg(geometry.x())
        .arg(geometry.y())
        .arg(geometry.width())
        .arg(geometry.height());
}

QString expandUserPath(const QString &path)
{
    if (path == QStringLiteral("~")) {
        return QDir::homePath();
    }
    if (path.startsWith(QStringLiteral("~/"))) {
        return QDir::home().filePath(path.mid(2));
    }
    return path;
}

QString shellQuote(QString value)
{
    if (value.isEmpty()) {
        return QStringLiteral("''");
    }
    value.replace(QStringLiteral("'"), QStringLiteral("'\"'\"'"));
    return QStringLiteral("'") + value + QStringLiteral("'");
}

bool extensionCommandUsesImagePlaceholder(const QString &command)
{
    return command.contains(QStringLiteral("{image}"))
        || command.contains(QStringLiteral("{imagePath}"))
        || command.contains(QStringLiteral("{imageUrl}"));
}

bool replaceExtensionImagePlaceholders(QString *command, const QString &imagePath)
{
    if (!command) {
        return false;
    }

    bool replaced = false;
    const QString quotedPath = shellQuote(imagePath);
    if (command->contains(QStringLiteral("{image}"))) {
        command->replace(QStringLiteral("{image}"), quotedPath);
        replaced = true;
    }
    if (command->contains(QStringLiteral("{imagePath}"))) {
        command->replace(QStringLiteral("{imagePath}"), quotedPath);
        replaced = true;
    }
    if (command->contains(QStringLiteral("{imageUrl}"))) {
        command->replace(QStringLiteral("{imageUrl}"), shellQuote(QUrl::fromLocalFile(imagePath).toString()));
        replaced = true;
    }
    return replaced;
}

bool replaceExtensionSlurpPlaceholder(QString *command, const QString &geometry)
{
    if (!command || !command->contains(QStringLiteral("{slurp}"))) {
        return false;
    }

    command->replace(QStringLiteral("{slurp}"), shellQuote(geometry));
    return true;
}

void replaceShellPlaceholder(QString *command, const QString &placeholder, const QString &value, bool *replaced = nullptr)
{
    if (!command || !command->contains(placeholder)) {
        return;
    }
    command->replace(placeholder, shellQuote(value));
    if (replaced) {
        *replaced = true;
    }
}

QStringList expandDesktopExec(const ShotWindow::DesktopApp &app, const QString &imagePath)
{
    QStringList command = QProcess::splitCommand(app.exec);
    if (command.isEmpty()) {
        return {};
    }

    const QString fileUrl = QUrl::fromLocalFile(imagePath).toString();
    bool usedFileField = false;
    QStringList expanded;
    for (QString argument : command) {
        if (argument == QStringLiteral("%i")) {
            continue;
        }

        bool appendArgument = true;
        if (argument.contains(QLatin1Char('%'))) {
            if (argument.contains(QStringLiteral("%f")) || argument.contains(QStringLiteral("%F"))) {
                argument.replace(QStringLiteral("%f"), imagePath);
                argument.replace(QStringLiteral("%F"), imagePath);
                usedFileField = true;
            }
            if (argument.contains(QStringLiteral("%u")) || argument.contains(QStringLiteral("%U"))) {
                argument.replace(QStringLiteral("%u"), fileUrl);
                argument.replace(QStringLiteral("%U"), fileUrl);
                usedFileField = true;
            }
            argument.replace(QStringLiteral("%c"), app.name);
            argument.replace(QStringLiteral("%k"), app.desktopPath);
            argument.replace(QStringLiteral("%%"), QStringLiteral("%"));

            static const QStringList unsupportedFields = {
                QStringLiteral("%d"), QStringLiteral("%D"), QStringLiteral("%n"), QStringLiteral("%N"),
                QStringLiteral("%v"), QStringLiteral("%m"),
            };
            for (const QString &field : unsupportedFields) {
                argument.remove(field);
            }
            appendArgument = !argument.trimmed().isEmpty();
        }

        if (appendArgument) {
            expanded.append(argument);
        }
    }

    if (!usedFileField) {
        expanded.append(imagePath);
    }
    return expanded;
}

QString helperProgramPath(const QString &programName)
{
    const QString appDir = QCoreApplication::applicationDirPath();
    const QString configDir = markShotConfigDir();
    const QString dataDir = markShotDataDir();
    const QStringList candidates = {
        QDir(appDir).filePath(programName),
        QDir(appDir).filePath(QStringLiteral("../scripts/%1").arg(programName)),
        QDir(appDir).filePath(QStringLiteral("../libexec/mark-shot/%1").arg(programName)),
        QDir(appDir).filePath(QStringLiteral("../lib/mark-shot/%1").arg(programName)),
        QDir::current().filePath(QStringLiteral("scripts/%1").arg(programName)),
        QDir(configDir).filePath(programName),
        QDir(configDir).filePath(QStringLiteral("scripts/%1").arg(programName)),
        QDir(dataDir).filePath(programName),
        QDir(dataDir).filePath(QStringLiteral("scripts/%1").arg(programName)),
        QDir::home().filePath(QStringLiteral(".local/bin/%1").arg(programName)),
        QStringLiteral("/usr/local/bin/%1").arg(programName),
        QStringLiteral("/usr/bin/%1").arg(programName),
    };

    for (const QString &candidate : candidates) {
        QFileInfo fileInfo(expandUserPath(QDir::cleanPath(candidate)));
        if (fileInfo.isExecutable()) {
            return fileInfo.absoluteFilePath();
        }
    }

    return programName;
}

struct PinnedWindowConfig {
    bool ocrEnabled = true;
    QString ocrBackend = QStringLiteral("auto");
    QString ocrCommand;
    int ocrTimeoutMs = kPinnedOcrTimeoutMs;
    QString translationCommand;
    QString translationTargetLanguage = QStringLiteral("Simplified Chinese");
    int translationTimeoutMs = kPinnedTranslationTimeoutMs;
};

QJsonObject objectValue(const QJsonObject &object, const QString &key)
{
    const QJsonValue value = object.value(key);
    return value.isObject() ? value.toObject() : QJsonObject();
}

PinnedWindowConfig pinnedWindowConfig()
{
    PinnedWindowConfig config;

    QFile file(appConfigPath());
    if (file.exists() && file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        QJsonParseError parseError;
        const QJsonDocument document = QJsonDocument::fromJson(file.readAll(), &parseError);
        if (parseError.error == QJsonParseError::NoError && document.isObject()) {
            const QJsonObject root = document.object();
            const QJsonObject ocr = objectValue(root, QStringLiteral("ocr"));
            if (ocr.value(QStringLiteral("enabled")).isBool()) {
                config.ocrEnabled = ocr.value(QStringLiteral("enabled")).toBool();
            }
            config.ocrBackend = ocr.value(QStringLiteral("backend")).toString(config.ocrBackend).trimmed();
            config.ocrCommand = ocr.value(QStringLiteral("command")).toString().trimmed();
            if (ocr.value(QStringLiteral("timeoutMs")).isDouble()) {
                config.ocrTimeoutMs = std::max(1000, ocr.value(QStringLiteral("timeoutMs")).toInt(config.ocrTimeoutMs));
            }

            const QJsonObject translation = objectValue(root, QStringLiteral("translation"));
            config.translationCommand = translation.value(QStringLiteral("command")).toString().trimmed();
            config.translationTargetLanguage = translation.value(QStringLiteral("targetLanguage"))
                                                   .toString(config.translationTargetLanguage)
                                                   .trimmed();
            if (translation.value(QStringLiteral("timeoutMs")).isDouble()) {
                config.translationTimeoutMs = std::max(3000, translation.value(QStringLiteral("timeoutMs")).toInt(config.translationTimeoutMs));
            }
        }
    }

    const QProcessEnvironment env = QProcessEnvironment::systemEnvironment();
    const QString disabled = env.value(QStringLiteral("MARK_SHOT_OCR_DISABLED")).trimmed().toLower();
    if (disabled == QStringLiteral("1") || disabled == QStringLiteral("true") || disabled == QStringLiteral("yes")) {
        config.ocrEnabled = false;
    }
    const QString envOcrBackend = env.value(QStringLiteral("MARK_SHOT_OCR_BACKEND")).trimmed();
    if (!envOcrBackend.isEmpty()) {
        config.ocrBackend = envOcrBackend;
    }
    const QString envOcrCommand = env.value(QStringLiteral("MARK_SHOT_OCR_COMMAND")).trimmed();
    if (!envOcrCommand.isEmpty()) {
        config.ocrCommand = envOcrCommand;
    }
    const QString envTargetLanguage = env.value(QStringLiteral("MARK_SHOT_TRANSLATION_TARGET_LANGUAGE")).trimmed();
    if (!envTargetLanguage.isEmpty()) {
        config.translationTargetLanguage = envTargetLanguage;
    }
    const QString envTranslationCommand = env.value(QStringLiteral("MARK_SHOT_TRANSLATION_COMMAND")).trimmed();
    if (!envTranslationCommand.isEmpty()) {
        config.translationCommand = envTranslationCommand;
    }

    if (config.ocrBackend.isEmpty()) {
        config.ocrBackend = QStringLiteral("auto");
    }
    if (config.translationTargetLanguage.isEmpty()) {
        config.translationTargetLanguage = QStringLiteral("Simplified Chinese");
    }
    return config;
}

class PinnedImageWindow final : public QWidget {
public:
    struct OcrToken {
        QString text;
        QRectF imageRect;
        int line = 0;
        int index = 0;
        qreal confidence = 0.0;
    };

    explicit PinnedImageWindow(QImage image)
        : m_pixmap(QPixmap::fromImage(std::move(image)))
        , m_imageSize(m_pixmap.size())
        , m_config(pinnedWindowConfig())
    {
        setWindowTitle(MS_TR("Pinned Mark Shot"));
        setAttribute(Qt::WA_DeleteOnClose);
        setWindowFlags(Qt::Window | Qt::FramelessWindowHint | Qt::WindowStaysOnTopHint);
        setFocusPolicy(Qt::StrongFocus);
        setMouseTracking(true);
        setCursor(Qt::OpenHandCursor);

        QSize targetSize = m_imageSize;
        if (QScreen *screen = QApplication::primaryScreen()) {
            const QSize maxSize = screen->availableGeometry().size() * 0.9;
            if (targetSize.width() > maxSize.width() || targetSize.height() > maxSize.height()) {
                targetSize.scale(maxSize, Qt::KeepAspectRatio);
            }
            m_scale = static_cast<qreal>(targetSize.width()) / std::max(1, m_imageSize.width());
            setFixedSize(targetSize);
            move(screen->availableGeometry().center() - rect().center());
        } else {
            setFixedSize(targetSize);
        }

        QTimer::singleShot(0, this, [this] { startOcr(); });
    }

    ~PinnedImageWindow() override
    {
        cancelTranslation();
        cancelOcr();
    }

protected:
    void paintEvent(QPaintEvent *) override
    {
        QPainter painter(this);
        painter.setRenderHint(QPainter::SmoothPixmapTransform, true);
        painter.drawPixmap(rect(), m_pixmap);

        if (m_translationActive) {
            drawTranslationOverlay(painter);
        }

        if (!hasTextSelection()) {
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
    }

    void mousePressEvent(QMouseEvent *event) override
    {
        if (event->button() == Qt::LeftButton) {
            if (const std::optional<int> token = tokenAt(widgetToImage(event->position()))) {
                m_selectingText = true;
                m_selectionAnchor = *token;
                m_selectionFocus = *token;
                setCursor(Qt::IBeamCursor);
                update();
                event->accept();
                return;
            }

            clearTextSelection();
            m_dragOffset = event->globalPosition().toPoint() - frameGeometry().topLeft();
            setCursor(Qt::ClosedHandCursor);
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

    void mouseMoveEvent(QMouseEvent *event) override
    {
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
            move(event->globalPosition().toPoint() - m_dragOffset);
            event->accept();
            return;
        }

        updateCursorForPosition(event->position());
        QWidget::mouseMoveEvent(event);
    }

    void mouseReleaseEvent(QMouseEvent *event) override
    {
        if (event->button() == Qt::LeftButton) {
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

    void wheelEvent(QWheelEvent *event) override
    {
        const QPoint delta = event->angleDelta();
        if (delta.y() == 0) {
            QWidget::wheelEvent(event);
            return;
        }

        const qreal factor = std::pow(1.12, static_cast<qreal>(delta.y()) / 120.0);
        resizeByScale(m_scale * factor, event->globalPosition().toPoint(), event->position());
        event->accept();
    }

    void mouseDoubleClickEvent(QMouseEvent *event) override
    {
        if (event->button() == Qt::LeftButton) {
            if (const std::optional<int> token = tokenAt(widgetToImage(event->position()))) {
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

    void contextMenuEvent(QContextMenuEvent *event) override
    {
        class LeftClickMenuFilter final : public QObject {
        public:
            explicit LeftClickMenuFilter(QObject *parent = nullptr) : QObject(parent) {}
        protected:
            bool eventFilter(QObject *obj, QEvent *event) override
            {
                if (event->type() == QEvent::MouseButtonPress || event->type() == QEvent::MouseButtonRelease || event->type() == QEvent::MouseButtonDblClick) {
                    QMouseEvent *me = static_cast<QMouseEvent *>(event);
                    if (me->button() != Qt::LeftButton) {
                        QWidget *widget = qobject_cast<QWidget *>(obj);
                        if (widget && widget->rect().contains(me->position().toPoint())) {
                            return true;
                        }
                    }
                }
                return QObject::eventFilter(obj, event);
            }
        };

        QMenu menu(this);
        LeftClickMenuFilter filter(&menu);
        menu.installEventFilter(&filter);

        menu.addAction(MS_TR("Rotate Left"), this, [this] { rotateImage(-90); });
        menu.addAction(MS_TR("Rotate Right"), this, [this] { rotateImage(90); });
        menu.addSeparator();
        menu.addAction(MS_TR("Zoom In"), this, [this, event] {
            resizeByScale(m_scale * 1.18, event->globalPos(), rect().center());
        });
        menu.addAction(MS_TR("Zoom Out"), this, [this, event] {
            resizeByScale(m_scale / 1.18, event->globalPos(), rect().center());
        });
        menu.addAction(MS_TR("Reset Size"), this, [this] {
            resizeByScale(1.0, frameGeometry().center(), QPointF(width() / 2.0, height() / 2.0));
        });
        menu.addSeparator();
        menu.addAction(MS_TR("Copy"), this, [this] {
            QApplication::clipboard()->setPixmap(m_pixmap);
        });
        QAction *copySelectedTextAction = menu.addAction(MS_TR("Copy Selected Text"), this, [this] {
            QApplication::clipboard()->setText(selectedText());
        });
        copySelectedTextAction->setEnabled(hasTextSelection());
        QAction *copyTextAction = menu.addAction(MS_TR("Copy Image Text"), this, [this] {
            QApplication::clipboard()->setText(allText());
        });
        copyTextAction->setEnabled(!activeTokens().isEmpty());
        QAction *translateAction = menu.addAction(MS_TR("Translate"), this, [this] {
            requestTranslation();
        });
        translateAction->setEnabled(canRequestTranslation());
        QAction *toggleTranslationAction = menu.addAction(
            m_translationActive ? MS_TR("Show Original Text") : MS_TR("Show Translated Text"),
            this,
            [this] { setTranslationActive(!m_translationActive); });
        toggleTranslationAction->setEnabled(!m_translationOverlayTokens.isEmpty() && !m_translationProcess);
        menu.addAction(MS_TR("Save As"), this, [this] { saveImageAs(); });
        menu.addSeparator();
        menu.addAction(MS_TR("Close"), this, &QWidget::close);
        menu.exec(event->globalPos());
    }

    void keyPressEvent(QKeyEvent *event) override
    {
        if (event->matches(QKeySequence::Copy) && hasTextSelection()) {
            QApplication::clipboard()->setText(selectedText());
            event->accept();
            return;
        }

        if (event->key() == Qt::Key_Escape) {
            close();
            return;
        }
        QWidget::keyPressEvent(event);
    }

private:
    void rotateImage(qreal degrees)
    {
        const QPoint center = frameGeometry().center();
        m_pixmap = m_pixmap.transformed(QTransform().rotate(degrees), Qt::SmoothTransformation);
        m_imageSize = m_pixmap.size();
        clearTextSelection();
        m_ocrTokens.clear();
        m_translatedTokens.clear();
        m_translationOverlayTokens.clear();
        m_translationActive = false;
        m_translateAfterOcr = false;
        cancelTranslation();
        resizeByScale(m_scale, center, QPointF(width() / 2.0, height() / 2.0));
        update();
        startOcr();
    }

    void saveImageAs()
    {
        const QString filename = QStringLiteral("mark-shot-pin-%1.png").arg(QDateTime::currentDateTime().toString(QStringLiteral("yyyyMMdd-hhmmss")));
        const QString path = QFileDialog::getSaveFileName(this,
                                                          MS_TR("Save Pinned Image"),
                                                          QDir(markShotPicturesDir()).filePath(filename),
                                                          MS_TR("PNG Images (*.png)"));
        if (!path.isEmpty()) {
            m_pixmap.save(path, "PNG");
        }
    }

    void resizeByScale(qreal scale, QPoint globalAnchor, QPointF localAnchor)
    {
        scale = std::clamp(scale, 0.1, 6.0);
        QSize targetSize(qMax(24, qRound(m_imageSize.width() * scale)),
                         qMax(24, qRound(m_imageSize.height() * scale)));
        targetSize.scale(targetSize, Qt::KeepAspectRatio);

        const qreal xRatio = width() > 0 ? localAnchor.x() / width() : 0.5;
        const qreal yRatio = height() > 0 ? localAnchor.y() / height() : 0.5;
        const QPoint topLeft(globalAnchor.x() - qRound(targetSize.width() * xRatio),
                             globalAnchor.y() - qRound(targetSize.height() * yRatio));

        m_scale = static_cast<qreal>(targetSize.width()) / std::max(1, m_imageSize.width());
        setFixedSize(targetSize);
        move(topLeft);
    }

    void startOcr()
    {
        cancelOcr();

        if (!m_config.ocrEnabled) {
            return;
        }

        QTemporaryFile tempFile(QDir(QStandardPaths::writableLocation(QStandardPaths::TempLocation).isEmpty()
                                    ? QDir::tempPath()
                                    : QStandardPaths::writableLocation(QStandardPaths::TempLocation))
                                    .filePath(QStringLiteral("mark-shot-pin-ocr-XXXXXX.png")));
        tempFile.setAutoRemove(false);
        if (!tempFile.open()) {
            return;
        }

        m_ocrTempPath = tempFile.fileName();
        if (!m_pixmap.save(&tempFile, "PNG")) {
            tempFile.close();
            QFile::remove(m_ocrTempPath);
            m_ocrTempPath.clear();
            return;
        }
        tempFile.close();

        auto *process = new QProcess(this);
        m_ocrProcess = process;

        if (!m_config.ocrCommand.isEmpty()) {
            QString commandLine = m_config.ocrCommand;
            const bool replaced = replaceExtensionImagePlaceholders(&commandLine, m_ocrTempPath);
            if (!replaced) {
                commandLine += QLatin1Char(' ');
                commandLine += shellQuote(m_ocrTempPath);
            }

            QString shell = QProcessEnvironment::systemEnvironment().value(QStringLiteral("SHELL"), QStringLiteral("/bin/sh"));
            if (shell.isEmpty()) {
                shell = QStringLiteral("/bin/sh");
            }
            process->setProgram(shell);
            process->setArguments({QStringLiteral("-c"), commandLine});
        } else {
            process->setProgram(defaultOcrHelperProgram());
            process->setArguments({QStringLiteral("--format"),
                                   QStringLiteral("json"),
                                   QStringLiteral("--backend"),
                                   m_config.ocrBackend,
                                   m_ocrTempPath});
        }

        connect(process, &QProcess::errorOccurred, this, [this, process] {
            if (process == m_ocrProcess && process->state() == QProcess::NotRunning) {
                finishOcr(process, QByteArray());
            }
        });
        connect(process, QOverload<int, QProcess::ExitStatus>::of(&QProcess::finished), this,
                [this, process](int exitCode, QProcess::ExitStatus exitStatus) {
            const QByteArray output = exitStatus == QProcess::NormalExit && exitCode == 0
                ? process->readAllStandardOutput()
                : QByteArray();
            finishOcr(process, output);
        });
        QTimer::singleShot(m_config.ocrTimeoutMs, process, [process] {
            if (process->state() != QProcess::NotRunning) {
                process->kill();
            }
        });

        process->start();
    }

    QString defaultOcrHelperProgram() const
    {
        return helperProgramPath(QStringLiteral("mark-shot-ocr"));
    }

    void cancelOcr()
    {
        if (m_ocrProcess) {
            disconnect(m_ocrProcess, nullptr, this, nullptr);
            if (m_ocrProcess->state() != QProcess::NotRunning) {
                m_ocrProcess->kill();
            }
            m_ocrProcess->deleteLater();
            m_ocrProcess = nullptr;
        }

        if (!m_ocrTempPath.isEmpty()) {
            QFile::remove(m_ocrTempPath);
            m_ocrTempPath.clear();
        }
    }

    void finishOcr(QProcess *process, const QByteArray &output)
    {
        if (process != m_ocrProcess) {
            return;
        }

        if (!output.isEmpty()) {
            applyOcrOutput(output);
        } else {
            m_translateAfterOcr = false;
        }

        m_ocrProcess = nullptr;
        if (!m_ocrTempPath.isEmpty()) {
            QFile::remove(m_ocrTempPath);
            m_ocrTempPath.clear();
        }
        process->deleteLater();
    }

    void applyOcrOutput(const QByteArray &output)
    {
        const QVector<OcrToken> tokens = tokensFromJsonOutput(output);
        if (tokens.isEmpty()) {
            m_translateAfterOcr = false;
            return;
        }

        m_ocrTokens = tokens;
        m_translatedTokens.clear();
        m_translationOverlayTokens.clear();
        m_translationActive = false;
        const bool translateAfterOcr = m_translateAfterOcr;
        m_translateAfterOcr = false;
        updateCursorForPosition(mapFromGlobal(QCursor::pos()));
        if (translateAfterOcr) {
            startTranslation();
        } else {
            update();
        }
    }

    QVector<OcrToken> tokensFromJsonOutput(const QByteArray &output) const
    {
        QJsonParseError parseError;
        const QJsonDocument document = QJsonDocument::fromJson(output, &parseError);
        if (parseError.error != QJsonParseError::NoError) {
            return {};
        }

        QJsonArray tokenArray;
        if (document.isArray()) {
            tokenArray = document.array();
        } else if (document.isObject()) {
            tokenArray = document.object().value(QStringLiteral("tokens")).toArray();
        }

        QVector<OcrToken> tokens;
        tokens.reserve(tokenArray.size());
        int fallbackIndex = 0;
        for (const QJsonValue &value : tokenArray) {
            if (!value.isObject()) {
                continue;
            }

            const QJsonObject object = value.toObject();
            OcrToken token;
            token.text = object.value(QStringLiteral("text")).toString().trimmed();
            if (token.text.isEmpty()) {
                continue;
            }

            const std::optional<QRectF> rect = ocrRect(object);
            if (!rect) {
                continue;
            }

            token.imageRect = rect->normalized().intersected(QRectF(QPointF(0.0, 0.0), QSizeF(m_imageSize)));
            if (token.imageRect.isEmpty()) {
                continue;
            }

            token.line = object.value(QStringLiteral("line")).toInt(0);
            token.index = object.value(QStringLiteral("index")).toInt(fallbackIndex++);
            token.confidence = object.value(QStringLiteral("confidence")).toDouble(0.0);
            tokens.append(token);
        }

        std::stable_sort(tokens.begin(), tokens.end(), [](const OcrToken &left, const OcrToken &right) {
            if (left.line != right.line) {
                return left.line < right.line;
            }
            if (left.index != right.index) {
                return left.index < right.index;
            }
            if (!qFuzzyCompare(left.imageRect.top(), right.imageRect.top())) {
                return left.imageRect.top() < right.imageRect.top();
            }
            return left.imageRect.left() < right.imageRect.left();
        });

        return tokens;
    }

    void startTranslation()
    {
        if (m_ocrTokens.isEmpty()) {
            return;
        }

        cancelTranslation();
        clearTextSelection();

        QTemporaryFile inputFile(QDir(QStandardPaths::writableLocation(QStandardPaths::TempLocation).isEmpty()
                                     ? QDir::tempPath()
                                     : QStandardPaths::writableLocation(QStandardPaths::TempLocation))
                                     .filePath(QStringLiteral("mark-shot-translate-XXXXXX.json")));
        inputFile.setAutoRemove(false);
        if (!inputFile.open()) {
            return;
        }

        m_translationInputPath = inputFile.fileName();
        inputFile.write(translationInputJson());
        inputFile.close();

        auto *process = new QProcess(this);
        m_translationProcess = process;

        if (!m_config.translationCommand.isEmpty()) {
            QString commandLine = m_config.translationCommand;
            bool replaced = false;
            replaceShellPlaceholder(&commandLine, QStringLiteral("{input}"), m_translationInputPath, &replaced);
            replaceShellPlaceholder(&commandLine, QStringLiteral("{inputPath}"), m_translationInputPath, &replaced);
            replaceShellPlaceholder(&commandLine, QStringLiteral("{targetLanguage}"), m_config.translationTargetLanguage, &replaced);
            replaceShellPlaceholder(&commandLine, QStringLiteral("{config}"), appConfigPath(), &replaced);
            if (!replaced) {
                commandLine += QLatin1Char(' ');
                commandLine += shellQuote(m_translationInputPath);
            }

            QString shell = QProcessEnvironment::systemEnvironment().value(QStringLiteral("SHELL"), QStringLiteral("/bin/sh"));
            if (shell.isEmpty()) {
                shell = QStringLiteral("/bin/sh");
            }
            process->setProgram(shell);
            process->setArguments({QStringLiteral("-c"), commandLine});
        } else {
            process->setProgram(defaultTranslationHelperProgram());
            process->setArguments({QStringLiteral("--input"),
                                   m_translationInputPath,
                                   QStringLiteral("--target-language"),
                                   m_config.translationTargetLanguage,
                                   QStringLiteral("--config"),
                                   appConfigPath()});
        }

        connect(process, &QProcess::errorOccurred, this, [this, process] {
            if (process == m_translationProcess && process->state() == QProcess::NotRunning) {
                finishTranslation(process, QByteArray());
            }
        });
        connect(process, QOverload<int, QProcess::ExitStatus>::of(&QProcess::finished), this,
                [this, process](int exitCode, QProcess::ExitStatus exitStatus) {
            const QByteArray output = exitStatus == QProcess::NormalExit && exitCode == 0
                ? process->readAllStandardOutput()
                : QByteArray();
            finishTranslation(process, output);
        });
        QTimer::singleShot(m_config.translationTimeoutMs, process, [process] {
            if (process->state() != QProcess::NotRunning) {
                process->kill();
            }
        });

        setTranslationBusyCursor(true);
        process->start();
        update();
    }

    QByteArray translationInputJson() const
    {
        QJsonArray tokens;
        for (const OcrToken &token : m_ocrTokens) {
            QJsonObject object;
            object.insert(QStringLiteral("text"), token.text);
            object.insert(QStringLiteral("box"), rectToJson(token.imageRect));
            object.insert(QStringLiteral("line"), token.line);
            object.insert(QStringLiteral("index"), token.index);
            object.insert(QStringLiteral("confidence"), token.confidence);
            tokens.append(object);
        }

        QJsonObject root;
        root.insert(QStringLiteral("targetLanguage"), m_config.translationTargetLanguage);
        root.insert(QStringLiteral("tokens"), tokens);
        return QJsonDocument(root).toJson(QJsonDocument::Compact);
    }

    QJsonArray rectToJson(QRectF rect) const
    {
        QJsonArray array;
        array.append(rect.x());
        array.append(rect.y());
        array.append(rect.width());
        array.append(rect.height());
        return array;
    }

    QString defaultTranslationHelperProgram() const
    {
        return helperProgramPath(QStringLiteral("mark-shot-translate"));
    }

    void cancelTranslation()
    {
        if (m_translationProcess) {
            disconnect(m_translationProcess, nullptr, this, nullptr);
            if (m_translationProcess->state() != QProcess::NotRunning) {
                m_translationProcess->kill();
            }
            m_translationProcess->deleteLater();
            m_translationProcess = nullptr;
        }

        if (!m_translationInputPath.isEmpty()) {
            QFile::remove(m_translationInputPath);
            m_translationInputPath.clear();
        }
        setTranslationBusyCursor(false);
    }

    void finishTranslation(QProcess *process, const QByteArray &output)
    {
        if (process != m_translationProcess) {
            return;
        }

        if (!output.isEmpty()) {
            const QVector<OcrToken> tokens = tokensFromJsonOutput(output);
            if (!tokens.isEmpty()) {
                m_translationOverlayTokens = tokens;
                m_translatedTokens = selectableTranslationTokens(tokens);
                m_translationActive = true;
                clearTextSelection();
                updateCursorForPosition(mapFromGlobal(QCursor::pos()));
                update();
            }
        }

        m_translationProcess = nullptr;
        if (!m_translationInputPath.isEmpty()) {
            QFile::remove(m_translationInputPath);
            m_translationInputPath.clear();
        }
        setTranslationBusyCursor(false);
        process->deleteLater();
    }

    std::optional<QRectF> ocrRect(const QJsonObject &object) const
    {
        if (object.contains(QStringLiteral("box"))) {
            return rectFromJsonValue(object.value(QStringLiteral("box")));
        }
        if (object.contains(QStringLiteral("bbox"))) {
            return rectFromJsonValue(object.value(QStringLiteral("bbox")));
        }
        if (object.contains(QStringLiteral("points"))) {
            return rectFromJsonValue(object.value(QStringLiteral("points")));
        }

        if (object.contains(QStringLiteral("x")) && object.contains(QStringLiteral("y"))) {
            return QRectF(object.value(QStringLiteral("x")).toDouble(),
                          object.value(QStringLiteral("y")).toDouble(),
                          object.value(QStringLiteral("width")).toDouble(),
                          object.value(QStringLiteral("height")).toDouble());
        }

        if (object.contains(QStringLiteral("left")) && object.contains(QStringLiteral("top"))) {
            return QRectF(object.value(QStringLiteral("left")).toDouble(),
                          object.value(QStringLiteral("top")).toDouble(),
                          object.value(QStringLiteral("width")).toDouble(),
                          object.value(QStringLiteral("height")).toDouble());
        }

        return std::nullopt;
    }

    std::optional<QRectF> rectFromJsonValue(const QJsonValue &value) const
    {
        if (!value.isArray()) {
            return std::nullopt;
        }

        const QJsonArray array = value.toArray();
        if (array.size() == 4 && array.at(0).isDouble()) {
            return QRectF(array.at(0).toDouble(),
                          array.at(1).toDouble(),
                          array.at(2).toDouble(),
                          array.at(3).toDouble());
        }

        if (array.size() < 2 || !array.at(0).isArray()) {
            return std::nullopt;
        }

        QRectF bounds;
        bool initialized = false;
        for (const QJsonValue &pointValue : array) {
            if (!pointValue.isArray()) {
                continue;
            }
            const QJsonArray point = pointValue.toArray();
            if (point.size() < 2) {
                continue;
            }
            const QPointF p(point.at(0).toDouble(), point.at(1).toDouble());
            bounds = initialized ? bounds.united(QRectF(p, QSizeF(0.0, 0.0))) : QRectF(p, QSizeF(0.0, 0.0));
            initialized = true;
        }

        if (!initialized) {
            return std::nullopt;
        }
        return bounds;
    }

    QPointF widgetToImage(QPointF point) const
    {
        if (width() <= 0 || height() <= 0 || m_imageSize.isEmpty()) {
            return {};
        }
        return QPointF(point.x() * static_cast<qreal>(m_imageSize.width()) / static_cast<qreal>(width()),
                       point.y() * static_cast<qreal>(m_imageSize.height()) / static_cast<qreal>(height()));
    }

    QRectF imageToWidget(QRectF imageRect) const
    {
        if (m_imageSize.isEmpty()) {
            return {};
        }
        const qreal sx = static_cast<qreal>(width()) / static_cast<qreal>(m_imageSize.width());
        const qreal sy = static_cast<qreal>(height()) / static_cast<qreal>(m_imageSize.height());
        return QRectF(imageRect.left() * sx,
                      imageRect.top() * sy,
                      imageRect.width() * sx,
                      imageRect.height() * sy);
    }

    std::optional<int> tokenAt(QPointF imagePoint) const
    {
        const QVector<OcrToken> &tokens = activeTokens();
        for (int i = 0; i < tokens.size(); ++i) {
            const QRectF hitRect = tokens.at(i).imageRect.adjusted(-2.0, -2.0, 2.0, 2.0);
            if (hitRect.contains(imagePoint)) {
                return i;
            }
        }
        return std::nullopt;
    }

    std::optional<int> closestToken(QPointF imagePoint) const
    {
        const QVector<OcrToken> &tokens = activeTokens();
        if (tokens.isEmpty()) {
            return std::nullopt;
        }

        int bestIndex = 0;
        qreal bestDistance = std::numeric_limits<qreal>::max();
        for (int i = 0; i < tokens.size(); ++i) {
            const QRectF rect = tokens.at(i).imageRect;
            const qreal dx = imagePoint.x() < rect.left()
                ? rect.left() - imagePoint.x()
                : imagePoint.x() > rect.right() ? imagePoint.x() - rect.right() : 0.0;
            const qreal dy = imagePoint.y() < rect.top()
                ? rect.top() - imagePoint.y()
                : imagePoint.y() > rect.bottom() ? imagePoint.y() - rect.bottom() : 0.0;
            const qreal distance = dx * dx + dy * dy;
            if (distance < bestDistance) {
                bestDistance = distance;
                bestIndex = i;
            }
        }
        return bestIndex;
    }

    void updateCursorForPosition(QPointF widgetPoint)
    {
        if (tokenAt(widgetToImage(widgetPoint))) {
            setCursor(Qt::IBeamCursor);
        } else {
            setCursor(Qt::OpenHandCursor);
        }
    }

    bool hasTextSelection() const
    {
        const QVector<OcrToken> &tokens = activeTokens();
        return m_selectionAnchor >= 0
            && m_selectionFocus >= 0
            && m_selectionAnchor < tokens.size()
            && m_selectionFocus < tokens.size();
    }

    std::pair<int, int> selectionRange() const
    {
        const int first = std::min(m_selectionAnchor, m_selectionFocus);
        const int last = std::max(m_selectionAnchor, m_selectionFocus);
        return {first, last};
    }

    void clearTextSelection()
    {
        if (m_selectionAnchor < 0 && m_selectionFocus < 0) {
            return;
        }
        m_selectionAnchor = -1;
        m_selectionFocus = -1;
        update();
    }

    QString selectedText() const
    {
        if (!hasTextSelection()) {
            return {};
        }

        const auto [first, last] = selectionRange();
        return tokenRangeText(first, last);
    }

    QString allText() const
    {
        const QVector<OcrToken> &tokens = activeTokens();
        if (tokens.isEmpty()) {
            return {};
        }
        return tokenRangeText(0, tokens.size() - 1);
    }

    QString tokenRangeText(int first, int last) const
    {
        const QVector<OcrToken> &tokens = activeTokens();
        QString text;
        int currentLine = -1;
        QRectF previousRect;
        QString previousText;
        for (int i = first; i <= last; ++i) {
            const OcrToken &token = tokens.at(i);
            if (currentLine != token.line) {
                if (!text.isEmpty()) {
                    text += QLatin1Char('\n');
                }
                currentLine = token.line;
            } else if (shouldInsertSpace(previousText, token.text, previousRect, token.imageRect)) {
                text += QLatin1Char(' ');
            }
            text += token.text;
            previousText = token.text;
            previousRect = token.imageRect;
        }
        return text;
    }

    const QVector<OcrToken> &activeTokens() const
    {
        return m_translationActive ? m_translatedTokens : m_ocrTokens;
    }

    bool canRequestTranslation() const
    {
        return m_config.ocrEnabled && !m_translationProcess;
    }

    void requestTranslation()
    {
        if (!canRequestTranslation()) {
            return;
        }

        if (m_ocrTokens.isEmpty()) {
            m_translateAfterOcr = true;
            if (!m_ocrProcess) {
                startOcr();
            }
            return;
        }

        m_translateAfterOcr = false;
        startTranslation();
    }

    void setTranslationActive(bool active)
    {
        if (active && m_translationOverlayTokens.isEmpty()) {
            return;
        }

        m_translationActive = active;
        clearTextSelection();
        updateCursorForPosition(mapFromGlobal(QCursor::pos()));
        update();
    }

    void setTranslationBusyCursor(bool active)
    {
        if (m_translationBusyCursor == active) {
            return;
        }

        m_translationBusyCursor = active;
        if (active) {
            QApplication::setOverrideCursor(Qt::WaitCursor);
        } else {
            QApplication::restoreOverrideCursor();
            updateCursorForPosition(mapFromGlobal(QCursor::pos()));
        }
    }

    QVector<OcrToken> selectableTranslationTokens(const QVector<OcrToken> &tokens) const
    {
        QVector<OcrToken> selectableTokens;
        int selectableIndex = 0;
        for (const OcrToken &token : tokens) {
            const QVector<OcrToken> splitTokens = splitTokenForSelection(token);
            for (OcrToken splitToken : splitTokens) {
                splitToken.index = selectableIndex++;
                selectableTokens.append(splitToken);
            }
        }
        return selectableTokens;
    }

    QVector<OcrToken> splitTokenForSelection(const OcrToken &token) const
    {
        QVector<OcrToken> splitTokens;
        if (token.text.size() <= 1) {
            splitTokens.append(token);
            return splitTokens;
        }

        qreal totalWeight = 0.0;
        QVector<qreal> weights;
        weights.reserve(token.text.size());
        for (const QChar ch : token.text) {
            const qreal weight = selectionCharacterWeight(ch);
            weights.append(weight);
            totalWeight += weight;
        }

        if (totalWeight <= 0.0 || token.imageRect.width() <= 0.0) {
            splitTokens.append(token);
            return splitTokens;
        }

        qreal offset = 0.0;
        for (int i = 0; i < token.text.size(); ++i) {
            const qreal nextOffset = offset + token.imageRect.width() * weights.at(i) / totalWeight;
            OcrToken splitToken = token;
            splitToken.text = token.text.mid(i, 1);
            splitToken.imageRect = QRectF(token.imageRect.left() + offset,
                                          token.imageRect.top(),
                                          std::max<qreal>(1.0, nextOffset - offset),
                                          token.imageRect.height());
            splitTokens.append(splitToken);
            offset = nextOffset;
        }

        return splitTokens;
    }

    qreal selectionCharacterWeight(QChar ch) const
    {
        if (ch.isSpace()) {
            return 0.45;
        }
        if (isNoLeadingSpacePunctuation(ch)) {
            return 0.65;
        }
        if (ch.unicode() < 0x80) {
            return 0.75;
        }
        return 1.0;
    }

    void drawTranslationOverlay(QPainter &painter) const
    {
        painter.save();
        painter.setRenderHint(QPainter::Antialiasing, true);
        painter.setRenderHint(QPainter::TextAntialiasing, true);

        for (const OcrToken &token : m_translationOverlayTokens) {
            const QRectF textRect = imageToWidget(token.imageRect).adjusted(-3.0, -2.0, 3.0, 2.0);
            if (textRect.isEmpty()) {
                continue;
            }

            painter.setPen(Qt::NoPen);
            painter.setBrush(QColor(255, 255, 255, 232));
            painter.drawRoundedRect(textRect, 2.0, 2.0);

            QFont font = painter.font();
            const int pixelSize = std::clamp(qRound(textRect.height() * 0.62), 8, 28);
            font.setPixelSize(pixelSize);

            QTextDocument document;
            document.setDefaultFont(font);
            document.setDocumentMargin(1.0);
            document.setTextWidth(textRect.width());
            QTextOption option;
            option.setWrapMode(QTextOption::WrapAtWordBoundaryOrAnywhere);
            document.setDefaultTextOption(option);
            document.setPlainText(token.text);

            painter.save();
            painter.setClipRect(textRect);
            painter.translate(textRect.topLeft());
            document.drawContents(&painter, QRectF(QPointF(0.0, 0.0), textRect.size()));
            painter.restore();
        }

        painter.restore();
    }

    bool shouldInsertSpace(const QString &previousText, const QString &currentText, QRectF previousRect, QRectF currentRect) const
    {
        if (previousText.isEmpty() || currentText.isEmpty()) {
            return false;
        }
        const QChar currentFirst = currentText.front();
        if (isNoLeadingSpacePunctuation(currentFirst)) {
            return false;
        }
        const qreal gap = currentRect.left() - previousRect.right();
        const qreal threshold = std::max<qreal>(3.0, std::min(previousRect.height(), currentRect.height()) * 0.28);
        return gap > threshold;
    }

    bool isNoLeadingSpacePunctuation(QChar ch) const
    {
        switch (ch.unicode()) {
        case '.':
        case ',':
        case ';':
        case ':':
        case '!':
        case '?':
        case ')':
        case ']':
        case '}':
        case 0x3001:
        case 0x3002:
        case 0x300B:
        case 0x3011:
        case 0xFF01:
        case 0xFF09:
        case 0xFF0C:
        case 0xFF1A:
        case 0xFF1B:
        case 0xFF1F:
            return true;
        default:
            return false;
        }
    }

    QPixmap m_pixmap;
    QSize m_imageSize;
    qreal m_scale = 1.0;
    QPoint m_dragOffset;
    PinnedWindowConfig m_config;
    QVector<OcrToken> m_ocrTokens;
    QVector<OcrToken> m_translatedTokens;
    QVector<OcrToken> m_translationOverlayTokens;
    QProcess *m_ocrProcess = nullptr;
    QProcess *m_translationProcess = nullptr;
    QString m_ocrTempPath;
    QString m_translationInputPath;
    int m_selectionAnchor = -1;
    int m_selectionFocus = -1;
    bool m_selectingText = false;
    bool m_translationActive = false;
    bool m_translateAfterOcr = false;
    bool m_translationBusyCursor = false;
};

} // namespace

ShotWindow::ShotWindow(QImage frozenFrame, QString outputName, QRect sourceGeometry, QWidget *parent)
    : QWidget(parent)
    , m_frozenFrame(std::move(frozenFrame))
    , m_outputName(std::move(outputName))
    , m_sourceGeometry(sourceGeometry)
{
    setWindowTitle(MS_TR("Mark Shot"));
    setCursor(Qt::CrossCursor);
    setMouseTracking(true);
    setFocusPolicy(Qt::StrongFocus);
    setAttribute(Qt::WA_DeleteOnClose);
    setWindowFlags(Qt::Window | Qt::FramelessWindowHint | Qt::WindowStaysOnTopHint);

    m_toolbar = new QWidget(this);
    m_toolbar->setObjectName(QStringLiteral("shotToolbar"));
    m_toolbar->setStyleSheet(markshot::theme::panelStyleSheet());
    m_toolbar->installEventFilter(this);

    m_toolbarLayout = new QHBoxLayout(m_toolbar);
    m_toolbarLayout->setContentsMargins(10, 10, 10, 10);
    m_toolbarLayout->setSpacing(7);

    m_toolbarLayout->addWidget(addToolbarButton(Action::ToolMove, QStringLiteral("V")));
    m_toolbarLayout->addWidget(addToolbarButton(Action::ToolSelect, QStringLiteral("S")));
    m_toolbarLayout->addWidget(addToolbarButton(Action::ToolPen, QStringLiteral("P")));
    m_toolbarLayout->addWidget(addToolbarButton(Action::ToolLine, QStringLiteral("L")));
    m_toolbarLayout->addWidget(addToolbarButton(Action::ToolHighlighter, QStringLiteral("H")));
    m_toolbarLayout->addWidget(addToolbarButton(Action::ToolRectangle, QStringLiteral("R")));
    m_toolbarLayout->addWidget(addToolbarButton(Action::ToolEllipse, QStringLiteral("E")));
    m_toolbarLayout->addWidget(addToolbarButton(Action::ToolArrow, QStringLiteral("A")));
    m_toolbarLayout->addWidget(addToolbarButton(Action::ToolText, QStringLiteral("T")));
    m_toolbarLayout->addWidget(addToolbarButton(Action::ToolNumber, QStringLiteral("N")));
    m_toolbarLayout->addWidget(addToolbarButton(Action::ToolMosaic, QStringLiteral("M")));
    m_toolbarLayout->addWidget(addToolbarButton(Action::ToolLaser, QStringLiteral("G")));
    m_toolbarLayout->addWidget(addToolbarButton(Action::Clear, QStringLiteral("Clear")));
    m_toolbarLayout->addWidget(addToolbarButton(Action::Undo, QStringLiteral("Ctrl+Z")));
    m_toolbarLayout->addWidget(addToolbarButton(Action::Redo, QStringLiteral("Ctrl+Shift+Z")));
    for (Action action : {Action::ToggleCaptureScope,
                          Action::ToggleToolbarLayout,
                          Action::OpenWith,
                          Action::Extensions,
                          Action::Pin,
                          Action::OcrCopy,
                          Action::Copy,
                          Action::Save,
                          Action::Cancel}) {
        const QString shortcut = action == Action::OpenWith ? QStringLiteral("Open")
            : action == Action::Extensions           ? QStringLiteral("Ext")
            : action == Action::Pin                  ? QStringLiteral("Pin")
            : action == Action::OcrCopy              ? QStringLiteral("OCR")
            : action == Action::Copy                 ? QStringLiteral("Ctrl+C")
            : action == Action::Save                 ? QStringLiteral("Ctrl+S")
            : action == Action::ToggleToolbarLayout  ? QStringLiteral("Layout")
            : action == Action::ToggleCaptureScope   ? QStringLiteral("F")
                                                     : QStringLiteral("Esc");
        QPushButton *button = addToolbarButton(action, shortcut);
        button->hide();
        m_fullscreenActionButtons.append(button);
        m_toolbarLayout->addWidget(button);
    }
    m_toolbar->hide();

    m_actionToolbar = new QWidget(this);
    m_actionToolbar->setObjectName(QStringLiteral("actionToolbar"));
    m_actionToolbar->setStyleSheet(m_toolbar->styleSheet());
    auto *actionLayout = new QVBoxLayout(m_actionToolbar);
    actionLayout->setContentsMargins(10, 10, 10, 10);
    actionLayout->setSpacing(7);
    actionLayout->addWidget(addToolbarButton(Action::ToggleCaptureScope, QStringLiteral("F"), m_actionToolbar));
    actionLayout->addWidget(addToolbarButton(Action::OpenWith, QStringLiteral("Open"), m_actionToolbar));
    actionLayout->addWidget(addToolbarButton(Action::Extensions, QStringLiteral("Ext"), m_actionToolbar));
    actionLayout->addWidget(addToolbarButton(Action::Pin, QStringLiteral("Pin"), m_actionToolbar));
    actionLayout->addWidget(addToolbarButton(Action::OcrCopy, QStringLiteral("OCR"), m_actionToolbar));
    actionLayout->addWidget(addToolbarButton(Action::Copy, QStringLiteral("Ctrl+C"), m_actionToolbar));
    actionLayout->addWidget(addToolbarButton(Action::Save, QStringLiteral("Ctrl+S"), m_actionToolbar));
    actionLayout->addWidget(addToolbarButton(Action::Cancel, QStringLiteral("Esc"), m_actionToolbar));
    m_actionToolbar->hide();

    auto shortcutBlockedByTextInput = [this] {
        if (m_textEditor && m_textEditor->isVisible()) {
            return true;
        }
        QWidget *focusWidget = QApplication::focusWidget();
        return qobject_cast<QLineEdit *>(focusWidget) != nullptr
            || qobject_cast<QTextEdit *>(focusWidget) != nullptr;
    };
    auto addPlainShortcut = [this, shortcutBlockedByTextInput](const QKeySequence &sequence, auto callback) {
        auto *shortcut = new QShortcut(sequence, this);
        shortcut->setContext(Qt::WindowShortcut);
        shortcut->setAutoRepeat(false);
        connect(shortcut, &QShortcut::activated, this, [this, shortcutBlockedByTextInput, callback] {
            if (shortcutBlockedByTextInput()) {
                return;
            }
            callback();
        });
    };
    addPlainShortcut(QKeySequence(Qt::Key_V), [this] { setTool(Tool::Move); });
    addPlainShortcut(QKeySequence(Qt::Key_S), [this] { setTool(Tool::Select); });
    addPlainShortcut(QKeySequence(Qt::Key_P), [this] { setTool(Tool::Pen); });
    addPlainShortcut(QKeySequence(Qt::Key_L), [this] { setTool(Tool::Line); });
    addPlainShortcut(QKeySequence(Qt::Key_H), [this] { setTool(Tool::Highlighter); });
    addPlainShortcut(QKeySequence(Qt::Key_R), [this] { setTool(Tool::Rectangle); });
    addPlainShortcut(QKeySequence(Qt::Key_E), [this] { setTool(Tool::Ellipse); });
    addPlainShortcut(QKeySequence(Qt::Key_A), [this] { setTool(Tool::Arrow); });
    addPlainShortcut(QKeySequence(Qt::Key_T), [this] { setTool(Tool::Text); });
    addPlainShortcut(QKeySequence(Qt::Key_N), [this] { setTool(Tool::Number); });
    addPlainShortcut(QKeySequence(Qt::Key_M), [this] { setTool(Tool::Mosaic); });
    addPlainShortcut(QKeySequence(Qt::Key_G), [this] { setTool(Tool::Laser); });
    addPlainShortcut(QKeySequence(Qt::Key_F), [this] { toggleCaptureScope(); });

    m_annotationPropertyPanel = new QWidget(this);
    m_annotationPropertyPanel->setObjectName(QStringLiteral("annotationPropertyPanel"));
    m_annotationPropertyPanel->setStyleSheet(m_toolbar->styleSheet());
    auto *propertyLayout = new QHBoxLayout(m_annotationPropertyPanel);
    propertyLayout->setContentsMargins(10, 10, 10, 10);
    propertyLayout->setSpacing(7);
    m_annotationPropertyTitle = new QLabel(QStringLiteral("Object"), m_annotationPropertyPanel);
    propertyLayout->addWidget(m_annotationPropertyTitle);
    m_propertyWidthLabel = new QLabel(MS_TR("Width %1").arg(2), m_annotationPropertyPanel);
    propertyLayout->addWidget(m_propertyWidthLabel);
    m_propertyWidthSlider = new QSlider(Qt::Horizontal, m_annotationPropertyPanel);
    m_propertyWidthSlider->setFocusPolicy(Qt::NoFocus);
    m_propertyWidthSlider->setFixedWidth(120);
    m_propertyWidthSlider->setToolTip(MS_TR("Selected object width or size"));
    connect(m_propertyWidthSlider, &QSlider::valueChanged, this, [this](int value) { setSelectedAnnotationWidth(value); });
    propertyLayout->addWidget(m_propertyWidthSlider);
    m_propertyOpacityLabel = new QLabel(MS_TR("Opacity %1%").arg(100), m_annotationPropertyPanel);
    propertyLayout->addWidget(m_propertyOpacityLabel);
    m_propertyOpacitySlider = new QSlider(Qt::Horizontal, m_annotationPropertyPanel);
    m_propertyOpacitySlider->setFocusPolicy(Qt::NoFocus);
    m_propertyOpacitySlider->setRange(0, 100);
    m_propertyOpacitySlider->setFixedWidth(110);
    m_propertyOpacitySlider->setToolTip(MS_TR("Selected object opacity"));
    connect(m_propertyOpacitySlider, &QSlider::valueChanged, this, [this](int value) { setSelectedAnnotationOpacity(value); });
    propertyLayout->addWidget(m_propertyOpacitySlider);
    m_propertyColorButton = new QPushButton(m_annotationPropertyPanel);
    m_propertyColorButton->setFocusPolicy(Qt::NoFocus);
    m_propertyColorButton->setToolTip(MS_TR("Change selected object color"));
    connect(m_propertyColorButton, &QPushButton::clicked, this, [this] { openSelectedAnnotationColorPalette(); });
    propertyLayout->addWidget(m_propertyColorButton);
    m_propertyTextBackgroundButton = new QPushButton(MS_TR("Bg"), m_annotationPropertyPanel);
    m_propertyTextBackgroundButton->setFocusPolicy(Qt::NoFocus);
    m_propertyTextBackgroundButton->setToolTip(MS_TR("Text background color"));
    connect(m_propertyTextBackgroundButton, &QPushButton::clicked, this, [this] { openSelectedTextBackgroundColorPalette(); });
    propertyLayout->addWidget(m_propertyTextBackgroundButton);
    m_propertyFillButton = new QPushButton(m_annotationPropertyPanel);
    m_propertyFillButton->setCheckable(true);
    m_propertyFillButton->setFocusPolicy(Qt::NoFocus);
    m_propertyFillButton->setIcon(markshot::ui::makeFillIcon(false));
    m_propertyFillButton->setIconSize(QSize(26, 26));
    m_propertyFillButton->setToolTip(MS_TR("Toggle shape fill"));
    connect(m_propertyFillButton, &QPushButton::toggled, this, [this](bool checked) {
        m_propertyFillButton->setIcon(markshot::ui::makeFillIcon(checked));
        setSelectedAnnotationFilled(checked);
    });
    propertyLayout->addWidget(m_propertyFillButton);
    m_propertyRadiusLabel = new QLabel(MS_TR("Radius %1").arg(0), m_annotationPropertyPanel);
    propertyLayout->addWidget(m_propertyRadiusLabel);
    m_propertyRadiusSlider = new QSlider(Qt::Horizontal, m_annotationPropertyPanel);
    m_propertyRadiusSlider->setFocusPolicy(Qt::NoFocus);
    m_propertyRadiusSlider->setRange(0, 48);
    m_propertyRadiusSlider->setFixedWidth(100);
    m_propertyRadiusSlider->setToolTip(MS_TR("Rectangle corner radius"));
    connect(m_propertyRadiusSlider, &QSlider::valueChanged, this, [this](int value) { setSelectedAnnotationCornerRadius(value); });
    propertyLayout->addWidget(m_propertyRadiusSlider);
    m_propertyFontButton = new QPushButton(MS_TR("Font"), m_annotationPropertyPanel);
    m_propertyFontButton->setFocusPolicy(Qt::NoFocus);
    m_propertyFontButton->setFixedWidth(160);
    m_propertyFontButton->setToolTip(MS_TR("Text font"));
    connect(m_propertyFontButton, &QPushButton::clicked, this, [this] { toggleSelectedTextFontPanel(); });
    propertyLayout->addWidget(m_propertyFontButton);
    m_propertyEditTextButton = new QPushButton(MS_TR("Edit"), m_annotationPropertyPanel);
    m_propertyEditTextButton->setFocusPolicy(Qt::NoFocus);
    m_propertyEditTextButton->setToolTip(MS_TR("Edit selected text"));
    connect(m_propertyEditTextButton, &QPushButton::clicked, this, [this] { beginEditingSelectedTextAnnotation(); });
    propertyLayout->addWidget(m_propertyEditTextButton);
    m_annotationPropertyPanel->hide();

    m_propertyColorDialogPanel = new QWidget(this);
    m_propertyColorDialogPanel->setObjectName(QStringLiteral("propertyColorDialogPanel"));
    m_propertyColorDialogPanel->setStyleSheet(markshot::theme::propertyColorDialogPanelStyleSheet());
    auto *propertyColorLayout = new QVBoxLayout(m_propertyColorDialogPanel);
    propertyColorLayout->setContentsMargins(12, 12, 12, 12);
    propertyColorLayout->setSpacing(0);
    m_propertyColorPicker = new markshot::ui::ColorPicker(m_propertyColorDialogPanel);
    m_propertyColorPicker->setColor(m_currentColor);
    connect(m_propertyColorPicker, &markshot::ui::ColorPicker::colorChanged, this,
            [this](const QColor &color) { applyPropertyColor(color); });
    propertyColorLayout->addWidget(m_propertyColorPicker);
    m_propertyColorDialogPanel->hide();

    m_propertyFontPanel = new QWidget(this);
    m_propertyFontPanel->setObjectName(QStringLiteral("propertyFontPanel"));
    m_propertyFontPanel->setStyleSheet(markshot::theme::openWithPanelStyleSheet());
    auto *fontPanelLayout = new QVBoxLayout(m_propertyFontPanel);
    fontPanelLayout->setContentsMargins(8, 8, 8, 8);
    fontPanelLayout->setSpacing(0);
    m_propertyFontList = new QListWidget(m_propertyFontPanel);
    m_propertyFontList->setFocusPolicy(Qt::NoFocus);
    m_propertyFontList->setUniformItemSizes(true);
    m_propertyFontList->setMinimumHeight(96);
    m_propertyFontList->setMaximumHeight(260);
    m_propertyFontList->setVerticalScrollBarPolicy(Qt::ScrollBarAsNeeded);
    m_propertyFontList->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    for (const QString &family : QFontDatabase::families()) {
        auto *item = new QListWidgetItem(family, m_propertyFontList);
        item->setData(Qt::UserRole, family);
        item->setFont(QFont(family, 12));
    }
    connect(m_propertyFontList, &QListWidget::itemClicked, this, [this](QListWidgetItem *item) {
        if (!item) {
            return;
        }
        setSelectedTextFontFamily(item->data(Qt::UserRole).toString());
        if (m_propertyFontPanel) {
            m_propertyFontPanel->hide();
        }
    });
    fontPanelLayout->addWidget(m_propertyFontList);
    m_propertyFontPanel->hide();

    m_openWithPanel = new QWidget(this);
    m_openWithPanel->setObjectName(QStringLiteral("openWithPanel"));
    m_openWithPanel->setStyleSheet(markshot::theme::openWithPanelStyleSheet());
    auto *openLayout = new QVBoxLayout(m_openWithPanel);
    openLayout->setContentsMargins(12, 12, 12, 12);
    openLayout->setSpacing(7);
    m_openWithPanel->hide();

    m_extensionPanel = new QWidget(this);
    m_extensionPanel->setObjectName(QStringLiteral("extensionPanel"));
    m_extensionPanel->setStyleSheet(markshot::theme::openWithPanelStyleSheet());
    auto *extensionLayout = new QVBoxLayout(m_extensionPanel);
    extensionLayout->setContentsMargins(12, 12, 12, 12);
    extensionLayout->setSpacing(7);
    m_extensionPanel->hide();

    m_colorPalette = new QWidget(this);
    m_colorPalette->setObjectName(QStringLiteral("colorPalette"));
    m_colorPalette->setStyleSheet(markshot::theme::colorPaletteStyleSheet());
    for (const QColor &color : markshot::theme::paletteColors()) {
        auto *button = new QPushButton(m_colorPalette);
        button->setFocusPolicy(Qt::NoFocus);
        button->setStyleSheet(QStringLiteral("background: %1;").arg(color.name()));
        connect(button, &QPushButton::clicked, this, [this, color] { setCurrentColor(color); });
    }
    m_colorPalettePreview = new QWidget(m_colorPalette);
    m_colorPalettePreview->setObjectName(QStringLiteral("colorPalettePreview"));
    m_colorPalette->installEventFilter(this);
    m_colorPalette->hide();
    updateColorPalettePreview();

    m_textEditor = new QTextEdit(this);
    m_textEditor->setObjectName(QStringLiteral("textEditor"));
    m_textEditor->setPlaceholderText(MS_TR("Type text"));
    m_textEditor->setStyleSheet(markshot::theme::textEditorStyleSheet(QColor(94, 234, 212), QColor(0, 0, 0, 0), 24));
    m_textEditor->setAcceptRichText(false);
    m_textEditor->setTabChangesFocus(false);
    m_textEditor->setFrameShape(QFrame::NoFrame);
    m_textEditor->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    m_textEditor->setVerticalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    m_textEditor->viewport()->setAutoFillBackground(false);
    m_textEditor->setToolTip(MS_TR("Enter inserts newline, click outside commits, Esc cancels"));
    m_textEditor->hide();
    m_textEditor->installEventFilter(this);

    m_laserClock.start();
    m_laserTimer = new QTimer(this);
    m_laserTimer->setInterval(33);
    connect(m_laserTimer, &QTimer::timeout, this, [this] { cleanupLaserStrokes(); });

    const QVector<QRect> screenRects = enumerateX11WindowGeometries();
    if (!screenRects.isEmpty() && m_sourceGeometry.isValid() && !m_sourceGeometry.isEmpty()) {
        for (const QRect &r : screenRects) {
            QRect imageRect(r.x() - m_sourceGeometry.x(),
                            r.y() - m_sourceGeometry.y(),
                            r.width(), r.height());
            const QRect clipped = imageRect.intersected(QRect(QPoint(0, 0), m_frozenFrame.size()));
            if (clipped.width() > 1 && clipped.height() > 1) {
                m_windowRects.append(clipped);
            }
        }
    } else if (!screenRects.isEmpty()) {
        for (const QRect &r : screenRects) {
            const QRect clipped = r.intersected(QRect(QPoint(0, 0), m_frozenFrame.size()));
            if (clipped.width() > 1 && clipped.height() > 1) {
                m_windowRects.append(clipped);
            }
        }
    }
}

bool ShotWindow::configureLayerShell(QScreen *screen)
{
#ifndef HAVE_LAYER_SHELL
    Q_UNUSED(screen);
    return false;
#else
    if (screen) {
        setScreen(screen);
    } else {
        resize(m_frozenFrame.size());
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

    layerWindow->setScope(QStringLiteral("mark-shot"));
    layerWindow->setLayer(LayerShellQt::Window::LayerOverlay);
    layerWindow->setAnchors(anchors);
    layerWindow->setMargins({});
    layerWindow->setExclusiveZone(-1);
    layerWindow->setKeyboardInteractivity(LayerShellQt::Window::KeyboardInteractivityExclusive);
    layerWindow->setActivateOnShow(true);
    layerWindow->setCloseOnDismissed(true);
    if (screen) {
        layerWindow->setScreen(screen);
        layerWindow->setDesiredSize({});
    } else {
        layerWindow->setWantsToBeOnActiveScreen(true);
        layerWindow->setDesiredSize({});
    }

    return true;
#endif
}

void ShotWindow::startFullscreenAnnotation()
{
    enterFullscreenAnnotation(true);
}

void ShotWindow::setImageNavigationEnabled(bool enabled)
{
    m_imageNavigationEnabled = enabled;
    if (!enabled) {
        m_imageZoom = 1.0;
        m_imageCenterInitialized = false;
        m_imageSelected = false;
        m_imagePanning = false;
    }
    updateFrozenImageRect();
    refreshViewGeometry();
    update();
}

void ShotWindow::enterFullscreenAnnotation(bool resetAnnotations)
{
    commitTextEditor();
    if (m_colorPalette) {
        m_colorPalette->hide();
    }

    if (!m_fullscreenAnnotation && hasUsableSelection()) {
        m_selectionBeforeFullscreenAnnotation = normalizedSelection();
    }
    m_mode = Mode::Editing;
    m_dragging = false;
    m_fullscreenAnnotation = true;
    applyToolbarLayout();
    m_toolbarDragging = false;
    m_toolbarUserPlaced = false;
    m_selection = QRectF(QPointF(0, 0), QSizeF(m_frozenFrame.size()));
    if (resetAnnotations) {
        m_annotations.clear();
        m_undoStack.clear();
        m_redoStack.clear();
        m_laserStrokes.clear();
        m_laserDraft.reset();
    }
    m_draft.reset();
    setSelectedAnnotations({});
    if (resetAnnotations) {
        m_nextNumber = 1;
        m_nextAnnotationId = 1;
    }
    if (m_openWithPanel) {
        m_openWithPanel->hide();
    }
    if (m_extensionPanel) {
        m_extensionPanel->hide();
    }
    if (m_annotationPropertyPanel) {
        m_annotationPropertyPanel->hide();
    }
    if (m_propertyColorDialogPanel) {
        m_propertyColorDialogPanel->hide();
    }
    if (m_propertyFontPanel) {
        m_propertyFontPanel->hide();
    }
    setTool(Tool::Pen);
    if (m_toolbar) {
        setFullscreenActionButtonsVisible(true);
        m_toolbar->show();
    }
    if (m_actionToolbar) {
        m_actionToolbar->hide();
    }
    updateToolbarGeometry();
    updateToolbarState();
    update();
}

void ShotWindow::leaveFullscreenAnnotation()
{
    commitTextEditor();
    m_dragging = false;
    m_toolbarDragging = false;
    m_toolbarUserPlaced = false;
    m_fullscreenAnnotation = false;
    m_toolbarVerticalLayout = false;
    applyToolbarLayout();
    m_selectionDrag = SelectionDrag::None;
    m_annotationDrag = SelectionDrag::None;
    m_annotationSelectionBoxActive = false;
    m_draft.reset();
    m_laserDraft.reset();

    if (m_selectionBeforeFullscreenAnnotation.has_value()) {
        m_selection = *m_selectionBeforeFullscreenAnnotation;
    } else {
        resetImageZoom();
        m_mode = Mode::Selecting;
        m_selection = {};
        if (m_toolbar) {
            m_toolbar->hide();
        }
        if (m_actionToolbar) {
            m_actionToolbar->hide();
        }
        setFullscreenActionButtonsVisible(false);
        updateToolbarState();
        update();
        return;
    }

    m_mode = Mode::Editing;
    setFullscreenActionButtonsVisible(false);
    if (m_toolbar) {
        m_toolbar->show();
    }
    if (m_actionToolbar) {
        m_actionToolbar->show();
    }
    updateToolbarGeometry();
    updateActionToolbarGeometry();
    updateOpenWithPanelGeometry();
    updateExtensionPanelGeometry();
    updateAnnotationPropertyPanelGeometry();
    updateToolbarState();
    update();
}

void ShotWindow::toggleCaptureScope()
{
    resetImageZoom();
    if (m_fullscreenAnnotation) {
        leaveFullscreenAnnotation();
    } else {
        enterFullscreenAnnotation(false);
    }
}

void ShotWindow::toggleToolbarLayout()
{
    m_toolbarVerticalLayout = !m_toolbarVerticalLayout;
    m_toolbarUserPlaced = false;
    applyToolbarLayout();
    updateToolbarGeometry();
    updateOpenWithPanelGeometry();
    updateExtensionPanelGeometry();
    updateAnnotationPropertyPanelGeometry();
    updateToolbarState();
}

void ShotWindow::applyToolbarLayout()
{
    if (!m_toolbarLayout) {
        return;
    }

    m_toolbarLayout->setDirection(m_toolbarVerticalLayout ? QBoxLayout::TopToBottom : QBoxLayout::LeftToRight);
    m_toolbar->setFixedSize(QWIDGETSIZE_MAX, QWIDGETSIZE_MAX);
    m_toolbar->adjustSize();
}

QPushButton *ShotWindow::addToolbarButton(Action action, const QString &shortcutText, QWidget *parentToolbar)
{
    QWidget *toolbar = parentToolbar ? parentToolbar : m_toolbar;
    auto *button = new QPushButton(toolbar);
    button->setIcon(markshot::ui::makeToolIcon(action));
    button->setIconSize(QSize(26, 26));
    button->setFocusPolicy(Qt::NoFocus);
    button->setToolTip(QStringLiteral("%1 (%2)").arg(markshot::i18n::translate(markshot::ui::actionName(action)), shortcutText));
    button->setProperty("action", markshot::ui::actionName(action));
    if (!parentToolbar && action == Action::ToolMove) {
        button->installEventFilter(this);
    }
    if (action == Action::Save) {
        button->setProperty("role", QStringLiteral("primary"));
    } else if (action == Action::Cancel) {
        button->setProperty("role", QStringLiteral("danger"));
    } else if (action == Action::OpenWith || action == Action::Extensions || action == Action::Pin || action == Action::OcrCopy || action == Action::Copy) {
        button->setProperty("role", QStringLiteral("secondary"));
    }

    if (action == Action::ToolMove) {
        connect(button, &QPushButton::clicked, this, [this] { setTool(Tool::Move); });
    } else if (action == Action::ToolSelect) {
        connect(button, &QPushButton::clicked, this, [this] { setTool(Tool::Select); });
    } else if (action == Action::ToolPen) {
        connect(button, &QPushButton::clicked, this, [this] { setTool(Tool::Pen); });
    } else if (action == Action::ToolLine) {
        connect(button, &QPushButton::clicked, this, [this] { setTool(Tool::Line); });
    } else if (action == Action::ToolHighlighter) {
        connect(button, &QPushButton::clicked, this, [this] { setTool(Tool::Highlighter); });
    } else if (action == Action::ToolRectangle) {
        connect(button, &QPushButton::clicked, this, [this] { setTool(Tool::Rectangle); });
    } else if (action == Action::ToolEllipse) {
        connect(button, &QPushButton::clicked, this, [this] { setTool(Tool::Ellipse); });
    } else if (action == Action::ToolArrow) {
        connect(button, &QPushButton::clicked, this, [this] { setTool(Tool::Arrow); });
    } else if (action == Action::ToolText) {
        connect(button, &QPushButton::clicked, this, [this] { setTool(Tool::Text); });
    } else if (action == Action::ToolNumber) {
        connect(button, &QPushButton::clicked, this, [this] { setTool(Tool::Number); });
    } else if (action == Action::ToolMosaic) {
        connect(button, &QPushButton::clicked, this, [this] { setTool(Tool::Mosaic); });
    } else if (action == Action::ToolLaser) {
        connect(button, &QPushButton::clicked, this, [this] { setTool(Tool::Laser); });
    } else if (action == Action::ToggleCaptureScope) {
        connect(button, &QPushButton::clicked, this, [this] { toggleCaptureScope(); });
    } else if (action == Action::ToggleToolbarLayout) {
        connect(button, &QPushButton::clicked, this, [this] { toggleToolbarLayout(); });
    } else if (action == Action::Clear) {
        connect(button, &QPushButton::clicked, this, [this] { clearAnnotations(); });
    } else if (action == Action::Undo) {
        connect(button, &QPushButton::clicked, this, [this] { undoAnnotationEdit(); });
    } else if (action == Action::Redo) {
        connect(button, &QPushButton::clicked, this, [this] { redoAnnotation(); });
    } else if (action == Action::OpenWith) {
        connect(button, &QPushButton::clicked, this, [this] { toggleOpenWithPanel(); });
    } else if (action == Action::Extensions) {
        connect(button, &QPushButton::clicked, this, [this] { toggleExtensionPanel(); });
    } else if (action == Action::Pin) {
        connect(button, &QPushButton::clicked, this, [this] { pinSelection(); });
    } else if (action == Action::OcrCopy) {
        connect(button, &QPushButton::clicked, this, [this] { ocrCopySelection(); });
    } else if (action == Action::Copy) {
        connect(button, &QPushButton::clicked, this, [this] { copySelection(); });
    } else if (action == Action::Save) {
        connect(button, &QPushButton::clicked, this, [this] { saveSelection(); });
    } else if (action == Action::Cancel) {
        connect(button, &QPushButton::clicked, this, [this] { close(); });
    }

    return button;
}

QVector<ShotWindow::DesktopApp> ShotWindow::imageDesktopApps() const
{
    QVector<DesktopApp> apps;
    QStringList seenPaths;

    for (const QString &appDir : desktopSearchDirs()) {
        if (!QDir(appDir).exists()) {
            continue;
        }

        QDirIterator iterator(appDir, {QStringLiteral("*.desktop")}, QDir::Files, QDirIterator::Subdirectories);
        while (iterator.hasNext()) {
            const QString desktopPath = iterator.next();
            if (seenPaths.contains(desktopPath)) {
                continue;
            }
            seenPaths.append(desktopPath);

            QFile file(desktopPath);
            if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) {
                continue;
            }

            const QStringList lines = QString::fromUtf8(file.readAll()).split(QLatin1Char('\n'));
            if (desktopEntryValue(lines, QStringLiteral("Type")) != QStringLiteral("Application")) {
                continue;
            }
            if (desktopEntryBool(lines, QStringLiteral("Hidden"))
                || desktopEntryBool(lines, QStringLiteral("NoDisplay"))
                || !desktopEntrySupportsImage(lines)) {
                continue;
            }

            const QString exec = desktopEntryValue(lines, QStringLiteral("Exec"));
            const QString name = desktopEntryValue(lines, QStringLiteral("Name"));
            const QString icon = desktopEntryValue(lines, QStringLiteral("Icon"));
            if (exec.isEmpty() || name.isEmpty()) {
                continue;
            }

            apps.append({name, desktopPath, exec, icon});
        }
    }

    std::sort(apps.begin(), apps.end(), [](const DesktopApp &left, const DesktopApp &right) {
        return QString::localeAwareCompare(left.name, right.name) < 0;
    });
    return apps;
}

QVector<ShotWindow::ExtensionCommand> ShotWindow::extensionCommands(QString *errorMessage) const
{
    if (errorMessage) {
        errorMessage->clear();
    }

    const QString configPath = extensionCommandsConfigPath();
    QFile file(configPath);
    if (!file.exists()) {
        return {};
    }
    if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        if (errorMessage) {
            *errorMessage = QStringLiteral("Cannot read %1").arg(configPath);
        }
        return {};
    }

    QJsonParseError parseError;
    const QJsonDocument document = QJsonDocument::fromJson(file.readAll(), &parseError);
    if (parseError.error != QJsonParseError::NoError) {
        if (errorMessage) {
            *errorMessage = QStringLiteral("Invalid JSON at offset %1: %2").arg(parseError.offset).arg(parseError.errorString());
        }
        return {};
    }

    QJsonArray commandArray;
    if (document.isArray()) {
        commandArray = document.array();
    } else if (document.isObject()) {
        const QJsonObject root = document.object();
        if (root.value(QStringLiteral("commands")).isArray()) {
            commandArray = root.value(QStringLiteral("commands")).toArray();
        } else if (root.value(QStringLiteral("command")).isString()) {
            commandArray.append(root);
        }
    } else {
        if (errorMessage) {
            *errorMessage = QStringLiteral("Expected a JSON array, a command object, or an object with a commands array");
        }
        return {};
    }

    if (commandArray.isEmpty()) {
        if (errorMessage) {
            *errorMessage = QStringLiteral("No extension commands found");
        }
        return {};
    }

    QVector<ExtensionCommand> commands;
    for (const QJsonValue &value : commandArray) {
        if (!value.isObject()) {
            continue;
        }

        const QJsonObject object = value.toObject();
        ExtensionCommand command;
        command.name = object.value(QStringLiteral("name")).toString().trimmed();
        command.command = object.value(QStringLiteral("command")).toString().trimmed();
        command.workingDirectory = object.value(QStringLiteral("workingDirectory"))
                                       .toString(object.value(QStringLiteral("cwd")).toString())
                                       .trimmed();
        command.description = object.value(QStringLiteral("description")).toString().trimmed();
        command.saveImage = extensionCommandUsesImagePlaceholder(command.command)
            || object.value(QStringLiteral("saveImage")).toBool(false)
            || object.value(QStringLiteral("needsImage")).toBool(false);
        if (object.value(QStringLiteral("closeOnStart")).isBool()) {
            command.closeOnStart = object.value(QStringLiteral("closeOnStart")).toBool();
        }

        if (command.name.isEmpty() || command.command.isEmpty()) {
            continue;
        }
        commands.append(command);
    }

    return commands;
}

bool ShotWindow::eventFilter(QObject *watched, QEvent *event)
{
    if (event->type() == QEvent::MouseButtonPress || event->type() == QEvent::KeyPress) {
        clearWheelPreview();
    }

    const bool isFullscreenMoveButton = m_fullscreenAnnotation
        && watched->property("action").toString() == markshot::ui::actionName(Action::ToolMove);
    if (isFullscreenMoveButton) {
        if (event->type() == QEvent::MouseButtonPress) {
            auto *mouseEvent = static_cast<QMouseEvent *>(event);
            if (mouseEvent->button() == Qt::LeftButton) {
                auto *eventWidget = qobject_cast<QWidget *>(watched);
                if (!eventWidget) {
                    return false;
                }
                m_dragging = true;
                m_toolbarDragging = true;
                m_toolbarDragStart = eventWidget->mapTo(this, mouseEvent->pos());
                m_toolbarBeforeDrag = m_toolbar->geometry();
                setCursor(Qt::SizeAllCursor);
                return true;
            }
        } else if (event->type() == QEvent::MouseMove && m_toolbarDragging) {
            auto *mouseEvent = static_cast<QMouseEvent *>(event);
            auto *eventWidget = qobject_cast<QWidget *>(watched);
            if (!eventWidget) {
                return false;
            }
            const QPoint delta = eventWidget->mapTo(this, mouseEvent->pos()) - m_toolbarDragStart;
            m_toolbarUserPlaced = true;
            m_toolbar->setGeometry(clampedToolbarGeometry(m_toolbarBeforeDrag.translated(delta)));
            updateOpenWithPanelGeometry();
            updateExtensionPanelGeometry();
            updateAnnotationPropertyPanelGeometry();
            return true;
        } else if (event->type() == QEvent::MouseButtonRelease && m_toolbarDragging) {
            auto *mouseEvent = static_cast<QMouseEvent *>(event);
            if (mouseEvent->button() == Qt::LeftButton) {
                m_dragging = false;
                m_toolbarDragging = false;
                updateCursor();
                updateOpenWithPanelGeometry();
                updateExtensionPanelGeometry();
                updateAnnotationPropertyPanelGeometry();
                return true;
            }
        }
    }

    if (watched == m_textEditor && event->type() == QEvent::KeyPress) {
        auto *keyEvent = static_cast<QKeyEvent *>(event);
        if (imageNavigationAvailable() && keyEvent->key() == Qt::Key_Control && !keyEvent->isAutoRepeat()) {
            if (m_ctrlTapTimer.isValid() && m_ctrlTapTimer.elapsed() <= kCtrlDoubleTapMs) {
                resetImageZoom();
                m_ctrlTapTimer.invalidate();
            } else {
                m_ctrlTapTimer.restart();
            }
            return true;
        }
        if (keyEvent->key() == Qt::Key_Escape) {
            m_draft.reset();
            m_editingTextAnnotationId.reset();
            m_textEditor->hide();
            m_textEditor->clear();
            setFocus(Qt::OtherFocusReason);
            update();
            return true;
        }
    }

    if (watched == m_colorPalette && event->type() == QEvent::MouseButtonPress) {
        m_colorPalette->hide();
        update();
        return true;
    }

    return QWidget::eventFilter(watched, event);
}

void ShotWindow::setFullscreenActionButtonsVisible(bool visible)
{
    for (QPushButton *button : std::as_const(m_fullscreenActionButtons)) {
        if (button) {
            button->setVisible(visible);
        }
    }
}

void ShotWindow::paintEvent(QPaintEvent *)
{
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.fillRect(rect(), QColor(0, 0, 0));
    painter.drawImage(m_frozenImageRect, m_frozenFrame);

    const QRectF selection = normalizedSelection();
    QPainterPath dimPath;
    dimPath.addRect(rect());
    if (hasUsableSelection()) {
        dimPath.addRect(imageRectToWidget(selection));
        painter.fillPath(dimPath, QColor(2, 6, 12, 128));
    } else {
        painter.fillRect(rect(), QColor(2, 6, 12, 88));
    }

    if (hasUsableSelection()) {
        const QRectF widgetSelection = imageRectToWidget(selection);
        painter.save();
        for (const Annotation &annotation : m_annotations) {
            if (m_editingTextAnnotationId.has_value() && annotation.id == *m_editingTextAnnotationId) {
                continue;
            }
            drawAnnotation(painter, annotation, true);
        }
        if (m_draft.has_value()) {
            drawAnnotation(painter, *m_draft, true);
        }
        const qint64 now = m_laserClock.isValid() ? m_laserClock.elapsed() : 0;
        for (const LaserStroke &stroke : m_laserStrokes) {
            const qreal opacity = std::clamp(static_cast<qreal>(stroke.expiresAt - now) / kLaserLifetimeMs, 0.0, 1.0);
            if (opacity > 0.0) {
                drawLaserStroke(painter, stroke, true, opacity);
            }
        }
        if (m_laserDraft.has_value()) {
            drawLaserStroke(painter, *m_laserDraft, true, 1.0);
        }
        drawSelectedAnnotationFrame(painter);
        if (m_annotationSelectionBoxActive) {
            const QRectF box = imageRectToWidget(m_annotationSelectionBox.normalized());
            painter.setPen(QPen(QColor(45, 212, 191), 1.5, Qt::DashLine));
            painter.setBrush(QColor(45, 212, 191, 34));
            painter.drawRoundedRect(box, 4.0, 4.0);
        }
        painter.restore();

        painter.setPen(QPen(QColor(94, 234, 212), 2.0));
        painter.setBrush(Qt::NoBrush);
        painter.drawRoundedRect(widgetSelection, 3.0, 3.0);

        if (m_tool == Tool::Move && !m_fullscreenAnnotation) {
            painter.setPen(Qt::NoPen);
            painter.setBrush(QColor(94, 234, 212));
            const QVector<QPointF> handles = {
                widgetSelection.topLeft(), QPointF(widgetSelection.center().x(), widgetSelection.top()), widgetSelection.topRight(),
                QPointF(widgetSelection.left(), widgetSelection.center().y()), QPointF(widgetSelection.right(), widgetSelection.center().y()),
                widgetSelection.bottomLeft(), QPointF(widgetSelection.center().x(), widgetSelection.bottom()), widgetSelection.bottomRight(),
            };
            for (const QPointF &handle : handles) {
                painter.drawRoundedRect(QRectF(handle.x() - 4.0, handle.y() - 4.0, 8.0, 8.0), 2.0, 2.0);
            }
        }

        const bool selectionInfoVisible = m_selectionDrag != SelectionDrag::None
            || (m_showSelectionInfo && m_selectionInfoTimer.isValid() && m_selectionInfoTimer.elapsed() <= 1000);
        if (selectionInfoVisible) {
            const QString sizeText = QStringLiteral("%1 x %2").arg(qRound(selection.width())).arg(qRound(selection.height()));
            painter.setFont(QFont(QStringLiteral("Sans Serif"), 11, QFont::DemiBold));
            const QFontMetrics metrics(painter.font());
            const QRectF labelRect(widgetSelection.left() + 10.0,
                                   widgetSelection.top() + 10.0,
                                   metrics.horizontalAdvance(sizeText) + 22.0,
                                   metrics.height() + 12.0);
            painter.setPen(Qt::NoPen);
            painter.setBrush(QColor(8, 13, 19, 220));
            painter.drawRoundedRect(labelRect, 10.0, 10.0);
            painter.setPen(QColor(204, 251, 241, 238));
            painter.drawText(labelRect, Qt::AlignCenter, sizeText);
        } else if (m_showSelectionInfo) {
            m_showSelectionInfo = false;
        }
    }

    if (m_hoveredWindowRect.has_value() && m_mode == Mode::Selecting) {
        const QRectF hoverWidget = imageRectToWidget(QRectF(*m_hoveredWindowRect));
        painter.setPen(QPen(QColor(94, 234, 212), 2.0));
        painter.setBrush(QColor(94, 234, 212, 32));
        painter.drawRect(hoverWidget);
    }

    if (!hasUsableSelection()) {
        const QString hint = QStringLiteral("Drag to select   Esc cancels");
        painter.setFont(QFont(QStringLiteral("Sans Serif"), 15, QFont::DemiBold));
        const QFontMetrics metrics(painter.font());
        const QRectF hintRect((width() - metrics.horizontalAdvance(hint) - 44.0) / 2.0,
                              (height() - metrics.height() - 24.0) / 2.0,
                              metrics.horizontalAdvance(hint) + 44.0,
                              metrics.height() + 24.0);
        painter.setPen(Qt::NoPen);
        painter.setBrush(QColor(8, 13, 19, 222));
        painter.drawRoundedRect(hintRect, 18.0, 18.0);
        painter.setPen(QColor(204, 251, 241, 240));
        painter.drawText(hintRect, Qt::AlignCenter, hint);
    }

    drawWheelPreview(painter);
}

void ShotWindow::resizeEvent(QResizeEvent *)
{
    updateFrozenImageRect();
    if (m_colorPalette && m_colorPalette->isVisible()) {
        updateColorPaletteGeometry(m_colorPaletteAnchor);
    }
    updateTextEditorGeometry();
    updateToolbarGeometry();
    updateActionToolbarGeometry();
    updateAnnotationPropertyPanelGeometry();
    updateOpenWithPanelGeometry();
    updateExtensionPanelGeometry();
}

void ShotWindow::mousePressEvent(QMouseEvent *event)
{
    clearWheelPreview();

    if (event->button() != Qt::LeftButton) {
        if (event->button() == Qt::MiddleButton && imageNavigationAvailable() && m_frozenImageRect.contains(event->position())) {
            commitTextEditor();
            m_dragging = false;
            m_annotationSelectionBoxActive = false;
            m_imagePanning = true;
            m_imagePanStartWidget = event->position();
            m_imagePanStartCenter = m_imageCenterInitialized
                ? m_imageCenter
                : QPointF(m_frozenFrame.width() / 2.0, m_frozenFrame.height() / 2.0);
            updateCursor();
            event->accept();
            return;
        }
        if (event->button() == Qt::RightButton && m_mode == Mode::Editing) {
            setTool(Tool::Select);
            event->accept();
            return;
        }
        return;
    }

    const QPointF imagePoint = widgetToImage(event->position());
    if (m_openWithPanel && m_openWithPanel->isVisible()
        && !m_openWithPanel->geometry().contains(event->pos())
        && (!m_actionToolbar || !m_actionToolbar->geometry().contains(event->pos()))
        && (!m_toolbar || !m_toolbar->geometry().contains(event->pos()))) {
        m_openWithPanel->hide();
    }
    if (m_extensionPanel && m_extensionPanel->isVisible()
        && !m_extensionPanel->geometry().contains(event->pos())
        && (!m_actionToolbar || !m_actionToolbar->geometry().contains(event->pos()))
        && (!m_toolbar || !m_toolbar->geometry().contains(event->pos()))) {
        m_extensionPanel->hide();
    }
    if (m_colorPalette && m_colorPalette->isVisible()
        && !m_colorPalette->geometry().contains(event->pos())) {
        m_colorPalette->hide();
        update();
    }
    if (m_propertyColorDialogPanel && m_propertyColorDialogPanel->isVisible()
        && !m_propertyColorDialogPanel->geometry().contains(event->pos())
        && (!m_annotationPropertyPanel || !m_annotationPropertyPanel->geometry().contains(event->pos()))
        && (!m_toolbar || !m_toolbar->geometry().contains(event->pos()))) {
        m_propertyColorDialogPanel->hide();
    }
    if (m_propertyFontPanel && m_propertyFontPanel->isVisible()
        && !m_propertyFontPanel->geometry().contains(event->pos())
        && (!m_annotationPropertyPanel || !m_annotationPropertyPanel->geometry().contains(event->pos()))
        && (!m_toolbar || !m_toolbar->geometry().contains(event->pos()))) {
        m_propertyFontPanel->hide();
    }
    if (m_textEditor && m_textEditor->isVisible() && !m_textEditor->geometry().contains(event->pos())) {
        commitTextEditor();
    }

    if (m_mode == Mode::Selecting) {
        if (m_colorPalette) {
            m_colorPalette->hide();
        }
        m_selectionClickStart = event->position();
        beginSelection(imagePoint);
        return;
    }

    if (!m_frozenImageRect.contains(event->position())) {
        return;
    }

    if (m_tool == Tool::Move && !m_fullscreenAnnotation) {
        m_selectionDrag = selectionDragAt(imagePoint);
        if (m_selectionDrag == SelectionDrag::None) {
            updateCursor();
            return;
        }
        m_dragging = true;
        m_dragStart = imagePoint;
        m_selectionBeforeDrag = normalizedSelection();
        revealSelectionInfo();
        updateCursor();
        update();
        return;
    }

    if (m_tool == Tool::Select) {
        if (selectedAnnotationDeleteButtonRect().contains(event->position())) {
            deleteSelectedAnnotation();
            return;
        }

        const QVector<int> selectedIds = selectedAnnotationIds();
        if (selectedIds.size() > 1) {
            const SelectionDrag drag = annotationBoundsDragAt(imagePoint, selectedAnnotationsBounds());
            if (drag != SelectionDrag::None) {
                beginAnnotationDrag(selectedIds.first(), drag, imagePoint);
                return;
            }
        } else if (m_selectedAnnotationId.has_value()) {
            const SelectionDrag drag = annotationDragAt(imagePoint, *m_selectedAnnotationId);
            if (drag != SelectionDrag::None) {
                beginAnnotationDrag(*m_selectedAnnotationId, drag, imagePoint);
                return;
            }
        }

        const std::optional<int> hitAnnotationId = annotationAt(imagePoint);
        if (hitAnnotationId.has_value()) {
            const SelectionDrag drag = annotationDragAt(imagePoint, *hitAnnotationId);
            setSelectedAnnotations({*hitAnnotationId});
            beginAnnotationDrag(*hitAnnotationId, drag == SelectionDrag::None ? SelectionDrag::Move : drag, imagePoint);
            updateAnnotationPropertyPanel();
        } else if (m_imageNavigationEnabled) {
            beginAnnotationSelectionBox(imagePoint);
            m_imageSelected = true;
        } else {
            beginAnnotationSelectionBox(imagePoint);
        }
        return;
    }

    if (m_tool == Tool::Text) {
        commitTextEditor();
        beginTextAnnotation(imagePoint);
        return;
    }

    if (m_tool == Tool::Number) {
        pushHistorySnapshot();
        Annotation annotation;
        annotation.id = m_nextAnnotationId++;
        annotation.tool = Tool::Number;
        annotation.points.append(imagePoint);
        annotation.number = m_nextNumber++;
        annotation.color = m_currentColor;
        annotation.width = m_numberWidth;
        m_annotations.append(annotation);
        update();
        return;
    }

    if (m_tool == Tool::Laser) {
        beginLaserStroke(imagePoint);
        return;
    }

    m_dragging = true;
    m_dragStart = imagePoint;
    Annotation annotation;
    annotation.tool = m_tool;
    annotation.color = m_currentColor;
    annotation.width = currentToolWidth();
    annotation.filled = m_shapeFilled;
    annotation.cornerRadius = m_tool == Tool::Rectangle ? m_rectangleCornerRadius : 0.0;
    annotation.fontFamily = m_textFontFamily;
    if (m_tool == Tool::Pen || m_tool == Tool::Highlighter) {
        annotation.points.append(imagePoint);
    } else if (m_tool == Tool::Mosaic) {
        annotation.width = m_mosaicBlockSize;
        annotation.rect = QRectF(imagePoint, imagePoint);
        annotation.points.append(imagePoint);
        annotation.points.append(imagePoint);
    } else {
        annotation.rect = QRectF(imagePoint, imagePoint);
        annotation.points.append(imagePoint);
        annotation.points.append(imagePoint);
    }
    m_draft = annotation;
    update();
}

void ShotWindow::mouseMoveEvent(QMouseEvent *event)
{
    if (m_imagePanning) {
        panImageTo(event->position());
        event->accept();
        return;
    }

    if (m_showWheelPreview && m_wheelPreviewTimer.isValid() && m_wheelPreviewTimer.elapsed() <= 900) {
        m_wheelPreviewPosition = event->position();
        update();
    } else if (m_showWheelPreview) {
        m_showWheelPreview = false;
        updateCursor();
        update();
    }

    const QPointF imagePoint = widgetToImage(event->position());
    if (m_mode == Mode::Selecting && !m_dragging) {
        std::optional<QRect> best;
        qint64 bestArea = std::numeric_limits<qint64>::max();
        const QPoint imgPt = imagePoint.toPoint();
        for (const QRect &r : std::as_const(m_windowRects)) {
            if (r.contains(imgPt)) {
                qint64 area = static_cast<qint64>(r.width()) * r.height();
                if (area < bestArea) {
                    bestArea = area;
                    best = r;
                }
            }
        }
        if (best != m_hoveredWindowRect) {
            m_hoveredWindowRect = best;
            update();
        }
    }
    if (m_mode == Mode::Selecting && m_dragging) {
        m_selection = normalizedRect(m_selectionStart, imagePoint);
        revealSelectionInfo();
        update();
        return;
    }

    if (m_mode == Mode::Editing && m_tool == Tool::Select && m_dragging && m_annotationDrag != SelectionDrag::None) {
        updateAnnotationDrag(imagePoint, event->modifiers().testFlag(Qt::ControlModifier));
        updateAnnotationPropertyPanelGeometry();
        return;
    }

    if (m_mode == Mode::Editing && m_tool == Tool::Select && m_dragging && m_annotationSelectionBoxActive) {
        updateAnnotationSelectionBox(imagePoint);
        return;
    }

    if (m_mode == Mode::Editing && m_tool == Tool::Select && !m_dragging) {
        if (selectedAnnotationIds().size() > 1) {
            m_annotationDrag = annotationBoundsDragAt(imagePoint, selectedAnnotationsBounds());
            if (m_annotationDrag != SelectionDrag::None) {
                updateCursor();
                return;
            }
        } else if (m_selectedAnnotationId.has_value()) {
            m_annotationDrag = annotationDragAt(imagePoint, *m_selectedAnnotationId);
            if (m_annotationDrag != SelectionDrag::None) {
                updateCursor();
                return;
            }
        }
        m_annotationDrag = annotationAt(imagePoint).has_value() ? SelectionDrag::Move : SelectionDrag::None;
        updateCursor();
        return;
    }

    if (m_fullscreenAnnotation && m_toolbarDragging) {
        const QPoint delta = event->pos() - m_toolbarDragStart;
        QRect toolbarGeometry = m_toolbarBeforeDrag.translated(delta);
        if (m_toolbar) {
            m_toolbarUserPlaced = true;
            m_toolbar->setGeometry(clampedToolbarGeometry(toolbarGeometry));
        }
        updateOpenWithPanelGeometry();
        updateExtensionPanelGeometry();
        updateAnnotationPropertyPanelGeometry();
        return;
    }

    if (m_mode == Mode::Editing && m_tool == Tool::Move && !m_fullscreenAnnotation && !m_dragging) {
        const SelectionDrag hoverDrag = selectionDragAt(imagePoint);
        switch (hoverDrag) {
        case SelectionDrag::Left:
        case SelectionDrag::Right:
            setCursor(Qt::SizeHorCursor);
            break;
        case SelectionDrag::Top:
        case SelectionDrag::Bottom:
            setCursor(Qt::SizeVerCursor);
            break;
        case SelectionDrag::TopLeft:
        case SelectionDrag::BottomRight:
            setCursor(Qt::SizeFDiagCursor);
            break;
        case SelectionDrag::TopRight:
        case SelectionDrag::BottomLeft:
            setCursor(Qt::SizeBDiagCursor);
            break;
        case SelectionDrag::Move:
            setCursor(Qt::SizeAllCursor);
            break;
        case SelectionDrag::None:
            setCursor(Qt::CrossCursor);
            break;
        }
        return;
    }

    if (m_mode == Mode::Editing && m_tool == Tool::Move && !m_fullscreenAnnotation && m_dragging && m_selectionDrag != SelectionDrag::None) {
        const QPointF clamped = clampImagePoint(imagePoint);
        const QRectF start = m_selectionBeforeDrag;
        const qreal maxWidth = m_frozenFrame.width();
        const qreal maxHeight = m_frozenFrame.height();
        qreal left = start.left();
        qreal top = start.top();
        qreal right = start.right();
        qreal bottom = start.bottom();

        if (m_selectionDrag == SelectionDrag::Move) {
            const QPointF delta = clamped - m_dragStart;
            left = std::clamp(start.left() + delta.x(), 0.0, std::max<qreal>(0.0, maxWidth - start.width()));
            top = std::clamp(start.top() + delta.y(), 0.0, std::max<qreal>(0.0, maxHeight - start.height()));
            right = left + start.width();
            bottom = top + start.height();
        } else {
            if (m_selectionDrag == SelectionDrag::Left || m_selectionDrag == SelectionDrag::TopLeft
                || m_selectionDrag == SelectionDrag::BottomLeft) {
                left = std::clamp(clamped.x(), 0.0, right - kMinSelectionSize);
            }
            if (m_selectionDrag == SelectionDrag::Right || m_selectionDrag == SelectionDrag::TopRight
                || m_selectionDrag == SelectionDrag::BottomRight) {
                right = std::clamp(clamped.x(), left + kMinSelectionSize, maxWidth);
            }
            if (m_selectionDrag == SelectionDrag::Top || m_selectionDrag == SelectionDrag::TopLeft
                || m_selectionDrag == SelectionDrag::TopRight) {
                top = std::clamp(clamped.y(), 0.0, bottom - kMinSelectionSize);
            }
            if (m_selectionDrag == SelectionDrag::Bottom || m_selectionDrag == SelectionDrag::BottomLeft
                || m_selectionDrag == SelectionDrag::BottomRight) {
                bottom = std::clamp(clamped.y(), top + kMinSelectionSize, maxHeight);
            }
        }

        m_selection = QRectF(QPointF(left, top), QPointF(right, bottom)).normalized();
        revealSelectionInfo();
        updateToolbarGeometry();
        updateActionToolbarGeometry();
        updateOpenWithPanelGeometry();
        updateExtensionPanelGeometry();
        updateTextEditorGeometry();
        update();
        return;
    }

    if (m_mode == Mode::Editing && m_tool == Tool::Laser && m_dragging && m_laserDraft.has_value()) {
        updateLaserStroke(imagePoint);
        return;
    }

    if (m_mode != Mode::Editing || !m_dragging || !m_draft.has_value()) {
        return;
    }

    const QPointF clamped = clampImagePoint(imagePoint);
    if (m_draft->tool == Tool::Pen || m_draft->tool == Tool::Highlighter) {
        m_draft->points.append(clamped);
    } else {
        if ((m_draft->tool == Tool::Rectangle || m_draft->tool == Tool::Ellipse)
            && event->modifiers().testFlag(Qt::ControlModifier)) {
            m_draft->rect = constrainedRect(m_dragStart, clamped);
        } else {
            m_draft->rect = normalizedRect(m_dragStart, clamped);
        }
        if (m_draft->points.size() >= 2) {
            m_draft->points[1] = (m_draft->tool == Tool::Rectangle || m_draft->tool == Tool::Ellipse)
                ? m_draft->rect.bottomRight()
                : clamped;
        }
    }
    update();
}

void ShotWindow::mouseDoubleClickEvent(QMouseEvent *event)
{
    if (event->button() == Qt::RightButton && m_mode == Mode::Editing) {
        toggleColorPalette(event->pos());
        event->accept();
        return;
    }

    if (event->button() != Qt::LeftButton || m_mode != Mode::Editing) {
        QWidget::mouseDoubleClickEvent(event);
        return;
    }

    const QPointF imagePoint = widgetToImage(event->position());
    if (!m_frozenImageRect.contains(event->position())) {
        QWidget::mouseDoubleClickEvent(event);
        return;
    }

    const auto annotationId = annotationAt(imagePoint);
    if (!annotationId) {
        QWidget::mouseDoubleClickEvent(event);
        return;
    }

    const Annotation *annotation = annotationById(*annotationId);
    if (!annotation || annotation->tool != Tool::Text) {
        QWidget::mouseDoubleClickEvent(event);
        return;
    }

    const int targetId = *annotationId;

    // The single click that preceded this double-click already executed
    // mousePressEvent in the active tool's branch. Roll back its side
    // effects so the user does not see a stray duplicate annotation when
    // we transition into text editing.
    switch (m_tool) {
    case Tool::Number:
        // mousePressEvent appended a number annotation and captured a
        // history snapshot. Undo both so the new digit doesn't linger.
        if (!m_annotations.isEmpty() && m_annotations.last().tool == Tool::Number) {
            m_annotations.removeLast();
            if (m_nextNumber > 1) {
                --m_nextNumber;
            }
            if (m_nextAnnotationId > 1) {
                --m_nextAnnotationId;
            }
        }
        if (!m_undoStack.isEmpty()) {
            m_undoStack.removeLast();
        }
        break;
    case Tool::Pen:
    case Tool::Highlighter:
    case Tool::Line:
    case Tool::Rectangle:
    case Tool::Ellipse:
    case Tool::Arrow:
    case Tool::Mosaic:
        // First press created an in-flight draft; discard it so the
        // upcoming mouseReleaseEvent (which still fires for the second
        // click of the double-click) does not commit a tiny stamp.
        m_draft.reset();
        break;
    case Tool::Laser:
        m_laserDraft.reset();
        break;
    case Tool::Text:
        // First press opened a fresh, empty text editor at the click point.
        // setTool(Select) below will call commitTextEditor() and tear it
        // down without producing an empty annotation.
        break;
    case Tool::Move:
    case Tool::Select:
        // No draft to discard.
        break;
    }

    m_dragging = false;
    m_annotationDrag = SelectionDrag::None;
    m_annotationHistoryCaptured = false;

    if (m_tool != Tool::Select) {
        setTool(Tool::Select);
    }
    setSelectedAnnotations({targetId});
    beginEditingSelectedTextAnnotation();
    update();
    event->accept();
}

void ShotWindow::mouseReleaseEvent(QMouseEvent *event)
{
    if ((event->button() == Qt::LeftButton || event->button() == Qt::MiddleButton) && m_imagePanning) {
        m_imagePanning = false;
        updateCursor();
        event->accept();
        return;
    }

    if (event->button() != Qt::LeftButton || !m_dragging) {
        return;
    }

    m_dragging = false;
    if (m_toolbarDragging) {
        m_toolbarDragging = false;
        updateCursor();
        updateOpenWithPanelGeometry();
        updateExtensionPanelGeometry();
        updateAnnotationPropertyPanelGeometry();
        return;
    }

    if (m_tool == Tool::Select && m_annotationDrag != SelectionDrag::None) {
        m_annotationDrag = SelectionDrag::None;
        m_annotationHistoryCaptured = false;
        updateAnnotationPropertyPanel();
        updateCursor();
        update();
        return;
    }

    if (m_tool == Tool::Select && m_annotationSelectionBoxActive) {
        if (m_imageNavigationEnabled && m_imageSelected) {
            const QRectF box = m_annotationSelectionBox.normalized();
            if (box.width() < kMinSelectionSize || box.height() < kMinSelectionSize) {
                m_annotationSelectionBoxActive = false;
                m_annotationSelectionBox = {};
                updateAnnotationPropertyPanel();
                updateCursor();
                update();
                return;
            }
            m_imageSelected = false;
        }
        commitAnnotationSelectionBox();
        updateCursor();
        update();
        return;
    }

    if (m_mode == Mode::Selecting) {
        const QPointF releasePos = event->position();
        const qreal clickDistance = QLineF(m_selectionClickStart, releasePos).length();
        if (clickDistance < 5.0 && m_hoveredWindowRect.has_value()) {
            m_selection = QRectF(*m_hoveredWindowRect);
            m_hoveredWindowRect.reset();
            m_dragging = false;
            if (!hasUsableSelection()) {
                m_selection = {};
                update();
                return;
            }
            m_mode = Mode::Editing;
            m_fullscreenAnnotation = false;
            m_toolbarUserPlaced = false;
            setTool(Tool::Pen);
            setFullscreenActionButtonsVisible(false);
            m_toolbar->show();
            m_actionToolbar->show();
            revealSelectionInfo();
            updateToolbarGeometry();
            updateActionToolbarGeometry();
            update();
            return;
        }
        m_hoveredWindowRect.reset();
        m_selection = normalizedSelection();
        if (!hasUsableSelection()) {
            m_selection = {};
            update();
            return;
        }
        m_mode = Mode::Editing;
        m_fullscreenAnnotation = false;
        m_toolbarUserPlaced = false;
        setTool(Tool::Pen);
        setFullscreenActionButtonsVisible(false);
        m_toolbar->show();
        m_actionToolbar->show();
        revealSelectionInfo();
        updateToolbarGeometry();
        updateActionToolbarGeometry();
        update();
        return;
    }

    if (m_tool == Tool::Move && !m_fullscreenAnnotation && m_selectionDrag != SelectionDrag::None) {
        m_selection = normalizedSelection();
        m_selectionDrag = SelectionDrag::None;
        revealSelectionInfo();
        updateCursor();
        updateToolbarGeometry();
        updateActionToolbarGeometry();
        updateOpenWithPanelGeometry();
        updateExtensionPanelGeometry();
        update();
        return;
    }

    if (m_tool == Tool::Laser && m_laserDraft.has_value()) {
        commitLaserStroke();
        updateCursor();
        update();
        return;
    }

    commitDraft();
}

void ShotWindow::wheelEvent(QWheelEvent *event)
{
    const int steps = event->angleDelta().y() / 120;
    if (steps == 0 || m_mode != Mode::Editing) {
        QWidget::wheelEvent(event);
        return;
    }

    if (m_tool == Tool::Select && !selectedAnnotationIds().isEmpty()) {
        pushHistorySnapshot();
        for (int id : selectedAnnotationIds()) {
            if (Annotation *annotation = annotationById(id)) {
                if (annotation->tool == Tool::Mosaic) {
                    annotation->width = std::clamp(annotation->width + steps * 2.0, kMinMosaicBlockSize, kMaxMosaicBlockSize);
                } else if (annotation->tool == Tool::Number) {
                    annotation->width = std::clamp(annotation->width + steps * 2.0, kMinNumberWidth, kMaxNumberWidth);
                } else if (annotation->tool == Tool::Text) {
                    const qreal oldWidth = annotation->width;
                    annotation->width = std::clamp(annotation->width + steps * 1.5, 1.0, 1000.0);
                    const qreal factor = ((19.0 + annotation->width) / (19.0 + oldWidth)) * 1.05;
                    annotation->rect.setWidth(annotation->rect.width() * factor);
                    annotation->rect = textContentRect(*annotation, false);
                    if (!annotation->points.isEmpty()) {
                        annotation->points[0] = annotation->rect.topLeft();
                    }
                } else {
                    annotation->width = std::clamp(annotation->width + steps * 1.0, kMinStrokeWidth, kMaxStrokeWidth);
                }
            }
        }
        updateColorPalettePreview();
        updateAnnotationPropertyPanel();
        event->accept();
        update();
        return;
    }

    if (wheelZoomsImage()) {
        const qreal factor = imageNavigationWheelFactor(event);
        if (qFuzzyCompare(factor, 1.0)) {
            QWidget::wheelEvent(event);
            return;
        }
        zoomImageAt(factor, event->position());
        m_showWheelPreview = true;
        m_wheelPreviewPosition = event->position();
        m_wheelPreviewTimer.restart();
        updateCursor();
        event->accept();
        update();
        return;
    }

    if (m_tool == Tool::Mosaic) {
        m_mosaicBlockSize = std::clamp(m_mosaicBlockSize + steps * 2.0, kMinMosaicBlockSize, kMaxMosaicBlockSize);
    } else if (m_tool == Tool::Number) {
        m_numberWidth = std::clamp(m_numberWidth + steps * 2.0, kMinNumberWidth, kMaxNumberWidth);
    } else if (m_tool == Tool::Laser) {
        m_laserWidth = std::clamp(m_laserWidth + steps * 2.0, kMinLaserWidth, kMaxLaserWidth);
    } else if (m_tool == Tool::Pen || m_tool == Tool::Highlighter) {
        m_penWidth = std::clamp(m_penWidth + steps * 1.0, kMinStrokeWidth, kMaxStrokeWidth);
    } else if (m_tool == Tool::Text) {
        m_shapeWidth = std::clamp(m_shapeWidth + steps * 1.5, 1.0, 1000.0);
    } else {
        m_shapeWidth = std::clamp(m_shapeWidth + steps * 1.0, kMinStrokeWidth, kMaxStrokeWidth);
    }

    if (m_draft.has_value()) {
        m_draft->width = currentToolWidth();
    }
    m_showWheelPreview = true;
    m_wheelPreviewPosition = event->position();
    m_wheelPreviewTimer.restart();
    updateCursor();
    updateColorPalettePreview();
    updateAnnotationPropertyPanel();
    event->accept();
    update();
}

void ShotWindow::keyPressEvent(QKeyEvent *event)
{
    clearWheelPreview();

    if (imageNavigationAvailable() && event->key() == Qt::Key_Control && !event->isAutoRepeat()) {
        if (m_ctrlTapTimer.isValid() && m_ctrlTapTimer.elapsed() <= kCtrlDoubleTapMs) {
            resetImageZoom();
            m_ctrlTapTimer.invalidate();
        } else {
            m_ctrlTapTimer.restart();
        }
        event->accept();
        return;
    }

    if (event->key() == Qt::Key_Escape) {
        close();
        return;
    }

    if (event->matches(QKeySequence::Copy)) {
        commitTextEditor();
        copySelection();
        return;
    }

    if (event->matches(QKeySequence::Save)) {
        commitTextEditor();
        saveSelection();
        return;
    }

    if (event->matches(QKeySequence::Undo)) {
        undoAnnotationEdit();
        return;
    }

    if (event->matches(QKeySequence::Redo)) {
        redoAnnotation();
        return;
    }

    if ((event->key() == Qt::Key_Backspace || event->key() == Qt::Key_Delete)
        && m_mode == Mode::Editing
        && m_tool == Tool::Select
        && !selectedAnnotationIds().isEmpty()) {
        commitTextEditor();
        deleteSelectedAnnotation();
        return;
    }

    switch (event->key()) {
    case Qt::Key_Return:
    case Qt::Key_Enter:
        saveSelection();
        break;
    case Qt::Key_V:
        setTool(Tool::Move);
        break;
    case Qt::Key_S:
        setTool(Tool::Select);
        break;
    case Qt::Key_P:
        setTool(Tool::Pen);
        break;
    case Qt::Key_L:
        setTool(Tool::Line);
        break;
    case Qt::Key_H:
        setTool(Tool::Highlighter);
        break;
    case Qt::Key_R:
        setTool(Tool::Rectangle);
        break;
    case Qt::Key_E:
        setTool(Tool::Ellipse);
        break;
    case Qt::Key_A:
        setTool(Tool::Arrow);
        break;
    case Qt::Key_T:
        setTool(Tool::Text);
        break;
    case Qt::Key_N:
        setTool(Tool::Number);
        break;
    case Qt::Key_M:
        setTool(Tool::Mosaic);
        break;
    case Qt::Key_G:
        setTool(Tool::Laser);
        break;
    case Qt::Key_F:
        toggleCaptureScope();
        break;
    default:
        QWidget::keyPressEvent(event);
        break;
    }
}

void ShotWindow::beginSelection(QPointF imagePoint)
{
    m_dragging = true;
    m_fullscreenAnnotation = false;
    m_toolbarDragging = false;
    m_toolbarUserPlaced = false;
    m_selectionDrag = SelectionDrag::None;
    m_selectionBeforeFullscreenAnnotation.reset();
    m_selectionStart = imagePoint;
    m_selection = QRectF(imagePoint, imagePoint);
    if (m_textEditor) {
        m_textEditor->hide();
        m_textEditor->clear();
    }
    if (m_openWithPanel) {
        m_openWithPanel->hide();
    }
    if (m_extensionPanel) {
        m_extensionPanel->hide();
    }
    if (m_annotationPropertyPanel) {
        m_annotationPropertyPanel->hide();
    }
    if (m_propertyColorDialogPanel) {
        m_propertyColorDialogPanel->hide();
    }
    if (m_propertyFontPanel) {
        m_propertyFontPanel->hide();
    }
    setFullscreenActionButtonsVisible(false);
    m_annotations.clear();
    m_undoStack.clear();
    m_redoStack.clear();
    m_draft.reset();
    m_laserStrokes.clear();
    m_laserDraft.reset();
    setSelectedAnnotations({});
    m_nextNumber = 1;
    m_nextAnnotationId = 1;
    revealSelectionInfo();
    update();
}

void ShotWindow::commitDraft()
{
    if (!m_draft.has_value()) {
        return;
    }

    if ((m_draft->tool == Tool::Pen || m_draft->tool == Tool::Highlighter) && m_draft->points.size() < 2) {
        m_draft.reset();
        update();
        return;
    }

    if ((m_draft->tool == Tool::Line || m_draft->tool == Tool::Arrow) && m_draft->points.size() >= 2
        && QLineF(m_draft->points.first(), m_draft->points.last()).length() < 2.0) {
        m_draft.reset();
        update();
        return;
    }

    if (m_draft->tool != Tool::Pen && m_draft->tool != Tool::Highlighter && m_draft->tool != Tool::Line
        && m_draft->tool != Tool::Arrow && m_draft->tool != Tool::Text
        && (m_draft->rect.width() < 2.0 || m_draft->rect.height() < 2.0)) {
        m_draft.reset();
        update();
        return;
    }

    pushHistorySnapshot();
    if (m_draft->id == 0) {
        m_draft->id = m_nextAnnotationId++;
    }
    m_annotations.append(*m_draft);
    m_draft.reset();
    update();
}

void ShotWindow::setTool(Tool tool)
{
    clearWheelPreview();
    commitTextEditor();
    m_selectionDrag = SelectionDrag::None;
    m_annotationDrag = SelectionDrag::None;
    m_annotationSelectionBoxActive = false;
    if (tool != Tool::Laser) {
        m_laserDraft.reset();
    }
    m_tool = tool;
    if (m_tool != Tool::Select) {
        setSelectedAnnotations({});
        m_imageSelected = false;
        m_imagePanning = false;
    }
    updateAnnotationPropertyPanel();
    updateCursor();
    updateToolbarState();
    update();
}

void ShotWindow::updateCursor()
{
    if (m_showWheelPreview && m_wheelPreviewTimer.isValid() && m_wheelPreviewTimer.elapsed() <= 900) {
        setCursor(Qt::BlankCursor);
        return;
    }

    if (m_imagePanning) {
        setCursor(Qt::ClosedHandCursor);
        return;
    }

    if (m_imageNavigationEnabled && m_tool == Tool::Select && m_imageSelected) {
        setCursor(m_imagePanning ? Qt::ClosedHandCursor : Qt::OpenHandCursor);
        return;
    }

    if (m_tool == Tool::Move && !m_fullscreenAnnotation) {
        switch (m_selectionDrag) {
        case SelectionDrag::Left:
        case SelectionDrag::Right:
            setCursor(Qt::SizeHorCursor);
            return;
        case SelectionDrag::Top:
        case SelectionDrag::Bottom:
            setCursor(Qt::SizeVerCursor);
            return;
        case SelectionDrag::TopLeft:
        case SelectionDrag::BottomRight:
            setCursor(Qt::SizeFDiagCursor);
            return;
        case SelectionDrag::TopRight:
        case SelectionDrag::BottomLeft:
            setCursor(Qt::SizeBDiagCursor);
            return;
        case SelectionDrag::Move:
            setCursor(Qt::SizeAllCursor);
            return;
        case SelectionDrag::None:
            setCursor(Qt::CrossCursor);
            return;
        }
    }

    if (m_tool == Tool::Select) {
        switch (m_annotationDrag) {
        case SelectionDrag::Left:
        case SelectionDrag::Right:
            setCursor(Qt::SizeHorCursor);
            return;
        case SelectionDrag::Top:
        case SelectionDrag::Bottom:
            setCursor(Qt::SizeVerCursor);
            return;
        case SelectionDrag::TopLeft:
        case SelectionDrag::BottomRight:
            setCursor(Qt::SizeFDiagCursor);
            return;
        case SelectionDrag::TopRight:
        case SelectionDrag::BottomLeft:
            setCursor(Qt::SizeBDiagCursor);
            return;
        case SelectionDrag::Move:
            setCursor(Qt::SizeAllCursor);
            return;
        case SelectionDrag::None:
            setCursor(Qt::ArrowCursor);
            return;
        }
    }

    setCursor(m_tool == Tool::Text ? Qt::IBeamCursor : Qt::CrossCursor);
}

void ShotWindow::clearWheelPreview()
{
    if (!m_showWheelPreview) {
        return;
    }

    m_showWheelPreview = false;
    m_wheelPreviewTimer.invalidate();
    updateCursor();
    update();
}

bool ShotWindow::hasUsableSelection() const
{
    const QRectF selection = normalizedSelection();
    return selection.width() >= kMinSelectionSize && selection.height() >= kMinSelectionSize;
}

bool ShotWindow::imageNavigationAvailable() const
{
    return m_imageNavigationEnabled || m_mode == Mode::Editing;
}

bool ShotWindow::wheelZoomsImage() const
{
    return m_imageNavigationEnabled || (m_mode == Mode::Editing && m_tool == Tool::Select);
}

qreal ShotWindow::annotationSizeScale(bool widgetCoordinates) const
{
    if (!widgetCoordinates || m_frozenFrame.isNull()) {
        return 1.0;
    }

    return !m_frozenImageRect.isEmpty()
        ? m_frozenImageRect.width() / std::max<qreal>(1.0, m_frozenFrame.width())
        : 1.0;
}

ShotWindow::SelectionDrag ShotWindow::selectionDragAt(QPointF imagePoint) const
{
    const QRectF selection = normalizedSelection();
    if (selection.isEmpty() || m_frozenImageRect.isEmpty()) {
        return SelectionDrag::None;
    }

    const qreal imageTolerance = 10.0 * m_frozenFrame.width() / std::max<qreal>(1.0, m_frozenImageRect.width());
    if (!selection.adjusted(-imageTolerance, -imageTolerance, imageTolerance, imageTolerance).contains(imagePoint)) {
        return SelectionDrag::None;
    }

    const bool nearLeft = std::abs(imagePoint.x() - selection.left()) <= imageTolerance;
    const bool nearRight = std::abs(imagePoint.x() - selection.right()) <= imageTolerance;
    const bool nearTop = std::abs(imagePoint.y() - selection.top()) <= imageTolerance;
    const bool nearBottom = std::abs(imagePoint.y() - selection.bottom()) <= imageTolerance;

    if (nearLeft && nearTop) {
        return SelectionDrag::TopLeft;
    }
    if (nearRight && nearTop) {
        return SelectionDrag::TopRight;
    }
    if (nearLeft && nearBottom) {
        return SelectionDrag::BottomLeft;
    }
    if (nearRight && nearBottom) {
        return SelectionDrag::BottomRight;
    }
    if (nearLeft) {
        return SelectionDrag::Left;
    }
    if (nearRight) {
        return SelectionDrag::Right;
    }
    if (nearTop) {
        return SelectionDrag::Top;
    }
    if (nearBottom) {
        return SelectionDrag::Bottom;
    }
    return selection.contains(imagePoint) ? SelectionDrag::Move : SelectionDrag::None;
}

ShotWindow::Annotation *ShotWindow::annotationById(int id)
{
    for (Annotation &annotation : m_annotations) {
        if (annotation.id == id) {
            return &annotation;
        }
    }
    return nullptr;
}

const ShotWindow::Annotation *ShotWindow::annotationById(int id) const
{
    for (const Annotation &annotation : m_annotations) {
        if (annotation.id == id) {
            return &annotation;
        }
    }
    return nullptr;
}

QRectF ShotWindow::annotationBounds(const Annotation &annotation) const
{
    auto pointsBounds = [&annotation] {
        if (annotation.points.isEmpty()) {
            return QRectF();
        }
        qreal left = annotation.points.first().x();
        qreal right = left;
        qreal top = annotation.points.first().y();
        qreal bottom = top;
        for (const QPointF &point : annotation.points) {
            left = std::min(left, point.x());
            right = std::max(right, point.x());
            top = std::min(top, point.y());
            bottom = std::max(bottom, point.y());
        }
        return QRectF(QPointF(left, top), QPointF(right, bottom)).normalized();
    };

    QRectF bounds;
    switch (annotation.tool) {
    case Tool::Move:
    case Tool::Select:
    case Tool::Laser:
        return {};
    case Tool::Pen:
    case Tool::Highlighter:
    case Tool::Line:
    case Tool::Arrow:
        bounds = pointsBounds();
        bounds.adjust(-annotation.width, -annotation.width, annotation.width, annotation.width);
        break;
    case Tool::Rectangle:
    case Tool::Ellipse:
    case Tool::Mosaic:
        bounds = annotation.rect.normalized();
        break;
    case Tool::Text:
        bounds = textContentRect(annotation, false);
        break;
    case Tool::Number: {
        if (annotation.points.isEmpty()) {
            return {};
        }
        const qreal radius = std::max<qreal>(13.0, 13.0 + annotation.width * 1.35);
        const QPointF center = annotation.points.first();
        bounds = QRectF(center.x() - radius, center.y() - radius, radius * 2.0, radius * 2.0);
        break;
    }
    }

    return bounds.normalized().intersected(QRectF(QPointF(0, 0), QSizeF(m_frozenFrame.size())));
}

QVector<int> ShotWindow::selectedAnnotationIds() const
{
    QVector<int> ids;
    for (int id : m_selectedAnnotationIds) {
        if (annotationById(id) && !ids.contains(id)) {
            ids.append(id);
        }
    }
    if (m_selectedAnnotationId.has_value() && annotationById(*m_selectedAnnotationId) && !ids.contains(*m_selectedAnnotationId)) {
        ids.append(*m_selectedAnnotationId);
    }
    return ids;
}

void ShotWindow::setSelectedAnnotations(QVector<int> annotationIds)
{
    QVector<int> validIds;
    for (int id : annotationIds) {
        if (annotationById(id) && !validIds.contains(id)) {
            validIds.append(id);
        }
    }
    m_selectedAnnotationIds = validIds;
    m_selectedAnnotationId = validIds.size() == 1
        ? std::optional<int>(validIds.first())
        : std::nullopt;
    if (!validIds.isEmpty()) {
        m_imageSelected = false;
        m_imagePanning = false;
    }
}

QRectF ShotWindow::selectedAnnotationsBounds() const
{
    QRectF bounds;
    for (int id : selectedAnnotationIds()) {
        const Annotation *annotation = annotationById(id);
        if (!annotation) {
            continue;
        }
        const QRectF annotationRect = annotationBounds(*annotation);
        if (annotationRect.isEmpty()) {
            continue;
        }
        bounds = bounds.isEmpty() ? annotationRect : bounds.united(annotationRect);
    }
    return bounds.normalized();
}

QVector<int> ShotWindow::annotationsInRect(QRectF imageRect) const
{
    imageRect = imageRect.normalized();
    QVector<int> ids;
    if (imageRect.width() < 2.0 || imageRect.height() < 2.0) {
        return ids;
    }
    for (const Annotation &annotation : m_annotations) {
        const QRectF bounds = annotationBounds(annotation);
        if (!bounds.isEmpty() && imageRect.intersects(bounds)) {
            ids.append(annotation.id);
        }
    }
    return ids;
}

ShotWindow::SelectionDrag ShotWindow::annotationBoundsDragAt(QPointF imagePoint, QRectF bounds) const
{
    bounds = bounds.normalized();
    if (bounds.isEmpty()) {
        return SelectionDrag::None;
    }

    const qreal imageTolerance = 10.0 * m_frozenFrame.width() / std::max<qreal>(1.0, m_frozenImageRect.width());
    const bool nearLeft = std::abs(imagePoint.x() - bounds.left()) <= imageTolerance;
    const bool nearRight = std::abs(imagePoint.x() - bounds.right()) <= imageTolerance;
    const bool nearTop = std::abs(imagePoint.y() - bounds.top()) <= imageTolerance;
    const bool nearBottom = std::abs(imagePoint.y() - bounds.bottom()) <= imageTolerance;

    if (nearLeft && nearTop) {
        return SelectionDrag::TopLeft;
    }
    if (nearRight && nearTop) {
        return SelectionDrag::TopRight;
    }
    if (nearLeft && nearBottom) {
        return SelectionDrag::BottomLeft;
    }
    if (nearRight && nearBottom) {
        return SelectionDrag::BottomRight;
    }
    if (nearLeft) {
        return SelectionDrag::Left;
    }
    if (nearRight) {
        return SelectionDrag::Right;
    }
    if (nearTop) {
        return SelectionDrag::Top;
    }
    if (nearBottom) {
        return SelectionDrag::Bottom;
    }
    return bounds.adjusted(-imageTolerance, -imageTolerance, imageTolerance, imageTolerance).contains(imagePoint)
        ? SelectionDrag::Move
        : SelectionDrag::None;
}

QVector<QPointF> ShotWindow::selectionHandlePoints(QRectF rect) const
{
    rect = rect.normalized();
    return {
        rect.topLeft(), QPointF(rect.center().x(), rect.top()), rect.topRight(),
        QPointF(rect.left(), rect.center().y()), QPointF(rect.right(), rect.center().y()),
        rect.bottomLeft(), QPointF(rect.center().x(), rect.bottom()), rect.bottomRight(),
    };
}

QRectF ShotWindow::selectedAnnotationDeleteButtonRect() const
{
    const QRectF bounds = imageRectToWidget(selectedAnnotationsBounds());
    if (bounds.isEmpty()) {
        return {};
    }
    constexpr qreal buttonSize = 20.0;
    const qreal x = std::clamp(bounds.right() + 8.0, 8.0, std::max<qreal>(8.0, width() - buttonSize - 8.0));
    const qreal y = std::clamp(bounds.top() - buttonSize - 8.0, 8.0, std::max<qreal>(8.0, height() - buttonSize - 8.0));
    return QRectF(x,
                  y,
                  buttonSize,
                  buttonSize);
}

QRectF ShotWindow::resizedBounds(QRectF start, SelectionDrag drag, QPointF imagePoint, bool keepAspectRatio) const
{
    start = start.normalized();
    const QPointF clamped = clampImagePoint(imagePoint);
    qreal left = start.left();
    qreal top = start.top();
    qreal right = start.right();
    qreal bottom = start.bottom();
    const qreal maxWidth = m_frozenFrame.width();
    const qreal maxHeight = m_frozenFrame.height();

    if (keepAspectRatio && drag != SelectionDrag::Move && start.width() > 0.0 && start.height() > 0.0) {
        const qreal minScale = std::max(kMinSelectionSize / start.width(), kMinSelectionSize / start.height());

        auto boundedScale = [minScale](qreal rawScale, qreal maxScale) {
            maxScale = std::max<qreal>(0.0, maxScale);
            const qreal lower = std::min(minScale, maxScale);
            return std::clamp(rawScale, lower, maxScale);
        };

        auto rectFromCorner = [&](QPointF anchor, qreal xSign, qreal ySign) {
            const qreal xDistance = std::abs(clamped.x() - anchor.x());
            const qreal yDistance = std::abs(clamped.y() - anchor.y());
            const qreal rawScale = std::max(xDistance / start.width(), yDistance / start.height());
            const qreal maxXScale = (xSign > 0.0 ? maxWidth - anchor.x() : anchor.x()) / start.width();
            const qreal maxYScale = (ySign > 0.0 ? maxHeight - anchor.y() : anchor.y()) / start.height();
            const qreal scale = boundedScale(rawScale, std::min(maxXScale, maxYScale));
            return QRectF(anchor,
                          QPointF(anchor.x() + xSign * start.width() * scale,
                                  anchor.y() + ySign * start.height() * scale)).normalized();
        };

        auto rectFromHorizontalEdge = [&](qreal anchorX, qreal xSign, qreal centerY) {
            const qreal rawScale = std::abs(clamped.x() - anchorX) / start.width();
            const qreal maxXScale = (xSign > 0.0 ? maxWidth - anchorX : anchorX) / start.width();
            const qreal maxYScale = (2.0 * std::min(centerY, maxHeight - centerY)) / start.height();
            const qreal scale = boundedScale(rawScale, std::min(maxXScale, maxYScale));
            const qreal newWidth = start.width() * scale;
            const qreal newHeight = start.height() * scale;
            return QRectF(QPointF(anchorX, centerY - newHeight / 2.0),
                          QPointF(anchorX + xSign * newWidth, centerY + newHeight / 2.0)).normalized();
        };

        auto rectFromVerticalEdge = [&](qreal anchorY, qreal ySign, qreal centerX) {
            const qreal rawScale = std::abs(clamped.y() - anchorY) / start.height();
            const qreal maxYScale = (ySign > 0.0 ? maxHeight - anchorY : anchorY) / start.height();
            const qreal maxXScale = (2.0 * std::min(centerX, maxWidth - centerX)) / start.width();
            const qreal scale = boundedScale(rawScale, std::min(maxXScale, maxYScale));
            const qreal newWidth = start.width() * scale;
            const qreal newHeight = start.height() * scale;
            return QRectF(QPointF(centerX - newWidth / 2.0, anchorY),
                          QPointF(centerX + newWidth / 2.0, anchorY + ySign * newHeight)).normalized();
        };

        switch (drag) {
        case SelectionDrag::TopLeft:
            return rectFromCorner(start.bottomRight(), -1.0, -1.0);
        case SelectionDrag::TopRight:
            return rectFromCorner(start.bottomLeft(), 1.0, -1.0);
        case SelectionDrag::BottomLeft:
            return rectFromCorner(start.topRight(), -1.0, 1.0);
        case SelectionDrag::BottomRight:
            return rectFromCorner(start.topLeft(), 1.0, 1.0);
        case SelectionDrag::Left:
            return rectFromHorizontalEdge(start.right(), -1.0, start.center().y());
        case SelectionDrag::Right:
            return rectFromHorizontalEdge(start.left(), 1.0, start.center().y());
        case SelectionDrag::Top:
            return rectFromVerticalEdge(start.bottom(), -1.0, start.center().x());
        case SelectionDrag::Bottom:
            return rectFromVerticalEdge(start.top(), 1.0, start.center().x());
        case SelectionDrag::Move:
        case SelectionDrag::None:
            break;
        }
    }

    if (drag == SelectionDrag::Left || drag == SelectionDrag::TopLeft || drag == SelectionDrag::BottomLeft) {
        left = std::clamp(clamped.x(), 0.0, right - kMinSelectionSize);
    }
    if (drag == SelectionDrag::Right || drag == SelectionDrag::TopRight || drag == SelectionDrag::BottomRight) {
        right = std::clamp(clamped.x(), left + kMinSelectionSize, maxWidth);
    }
    if (drag == SelectionDrag::Top || drag == SelectionDrag::TopLeft || drag == SelectionDrag::TopRight) {
        top = std::clamp(clamped.y(), 0.0, bottom - kMinSelectionSize);
    }
    if (drag == SelectionDrag::Bottom || drag == SelectionDrag::BottomLeft || drag == SelectionDrag::BottomRight) {
        bottom = std::clamp(clamped.y(), top + kMinSelectionSize, maxHeight);
    }

    return QRectF(QPointF(left, top), QPointF(right, bottom)).normalized();
}

ShotWindow::SelectionDrag ShotWindow::annotationDragAt(QPointF imagePoint, int annotationId) const
{
    const Annotation *annotation = annotationById(annotationId);
    if (!annotation) {
        return SelectionDrag::None;
    }

    const QRectF bounds = annotationBounds(*annotation);
    if (bounds.isEmpty()) {
        return SelectionDrag::None;
    }

    return annotationBoundsDragAt(imagePoint, bounds);
}

std::optional<int> ShotWindow::annotationAt(QPointF imagePoint) const
{
    const qreal imageTolerance = 8.0 * m_frozenFrame.width() / std::max<qreal>(1.0, m_frozenImageRect.width());
    for (int i = m_annotations.size() - 1; i >= 0; --i) {
        const Annotation &annotation = m_annotations.at(i);
        const QRectF bounds = annotationBounds(annotation).adjusted(-imageTolerance, -imageTolerance, imageTolerance, imageTolerance);
        if (bounds.contains(imagePoint)) {
            return annotation.id;
        }
    }
    return std::nullopt;
}

void ShotWindow::drawSelectedAnnotationFrame(QPainter &painter) const
{
    if (m_imageNavigationEnabled && m_tool == Tool::Select && m_imageSelected && selectedAnnotationIds().isEmpty()) {
        painter.save();
        painter.setPen(QPen(QColor(45, 212, 191), 2.0, Qt::DashLine));
        painter.setBrush(Qt::NoBrush);
        painter.drawRoundedRect(m_frozenImageRect.adjusted(2.0, 2.0, -2.0, -2.0), 6.0, 6.0);
        painter.restore();
        return;
    }

    const QVector<int> selectedIds = selectedAnnotationIds();
    if (selectedIds.isEmpty()) {
        return;
    }

    const QRectF bounds = imageRectToWidget(selectedAnnotationsBounds());
    if (bounds.isEmpty()) {
        return;
    }

    painter.save();
    painter.setPen(QPen(QColor(251, 146, 60), 2.0, Qt::DashLine));
    painter.setBrush(Qt::NoBrush);
    painter.drawRoundedRect(bounds, 4.0, 4.0);
    if (selectedIds.size() > 1) {
        painter.setPen(QPen(QColor(251, 146, 60, 150), 1.0, Qt::DashLine));
        for (int id : selectedIds) {
            if (const Annotation *annotation = annotationById(id)) {
                painter.drawRoundedRect(imageRectToWidget(annotationBounds(*annotation)), 3.0, 3.0);
            }
        }
    }
    painter.setPen(Qt::NoPen);
    painter.setBrush(QColor(251, 146, 60));
    for (const QPointF &handle : selectionHandlePoints(bounds)) {
        painter.drawRoundedRect(QRectF(handle.x() - 4.5, handle.y() - 4.5, 9.0, 9.0), 2.0, 2.0);
    }
    const QRectF deleteButton = selectedAnnotationDeleteButtonRect();
    if (!deleteButton.isEmpty()) {
        painter.setBrush(QColor(239, 68, 68));
        painter.setPen(QPen(QColor(255, 255, 255), 2.0, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
        painter.drawEllipse(deleteButton);
        painter.drawLine(deleteButton.center() + QPointF(-4.5, -4.5), deleteButton.center() + QPointF(4.5, 4.5));
        painter.drawLine(deleteButton.center() + QPointF(4.5, -4.5), deleteButton.center() + QPointF(-4.5, 4.5));
    }
    painter.restore();
}

void ShotWindow::moveAnnotation(Annotation &annotation, QPointF delta) const
{
    annotation.rect.translate(delta);
    for (QPointF &point : annotation.points) {
        point = clampImagePoint(point + delta);
    }
}

void ShotWindow::transformAnnotation(Annotation &annotation, QRectF oldBounds, QRectF newBounds) const
{
    oldBounds = oldBounds.normalized();
    newBounds = newBounds.normalized();
    if (oldBounds.width() <= 0.0 || oldBounds.height() <= 0.0) {
        moveAnnotation(annotation, newBounds.topLeft() - oldBounds.topLeft());
        return;
    }

    auto mapPoint = [this, oldBounds, newBounds](QPointF point) {
        const qreal xRatio = (point.x() - oldBounds.left()) / oldBounds.width();
        const qreal yRatio = (point.y() - oldBounds.top()) / oldBounds.height();
        return clampImagePoint(QPointF(newBounds.left() + xRatio * newBounds.width(),
                                      newBounds.top() + yRatio * newBounds.height()));
    };
    const qreal scaleFactor = std::max(newBounds.width() / oldBounds.width(),
                                       newBounds.height() / oldBounds.height());

    switch (annotation.tool) {
    case Tool::Move:
    case Tool::Select:
    case Tool::Laser:
        return;
    case Tool::Rectangle:
    case Tool::Ellipse:
    case Tool::Mosaic:
        annotation.rect = QRectF(mapPoint(annotation.rect.normalized().topLeft()),
                                 mapPoint(annotation.rect.normalized().bottomRight())).normalized();
        break;
    case Tool::Text:
        annotation.rect = QRectF(mapPoint(annotation.rect.normalized().topLeft()),
                                 mapPoint(annotation.rect.normalized().bottomRight())).normalized();
        if (m_annotationDrag == SelectionDrag::TopLeft ||
            m_annotationDrag == SelectionDrag::BottomRight ||
            m_annotationDrag == SelectionDrag::TopRight ||
            m_annotationDrag == SelectionDrag::BottomLeft) {
            annotation.width = std::clamp((19.0 + annotation.width) * scaleFactor - 19.0, 1.0, 1000.0);
            if (!annotation.points.isEmpty()) {
                annotation.points[0] = annotation.rect.topLeft();
            }
        }
        break;
    case Tool::Pen:
    case Tool::Highlighter:
    case Tool::Line:
    case Tool::Arrow:
        for (QPointF &point : annotation.points) {
            point = mapPoint(point);
        }
        break;
    case Tool::Number:
        if (!annotation.points.isEmpty()) {
            annotation.points[0] = mapPoint(annotation.points.first());
            annotation.width = std::clamp(annotation.width * scaleFactor, kMinNumberWidth, kMaxNumberWidth);
        }
        break;
    }
}

void ShotWindow::beginAnnotationDrag(int annotationId, SelectionDrag drag, QPointF imagePoint)
{
    Annotation *annotation = annotationById(annotationId);
    if (!annotation || drag == SelectionDrag::None) {
        return;
    }
    if (!selectedAnnotationIds().contains(annotationId)) {
        setSelectedAnnotations({annotationId});
    }
    m_annotationDrag = drag;
    m_annotationBeforeDrag = *annotation;
    m_annotationsBeforeDrag.clear();
    for (int id : selectedAnnotationIds()) {
        if (const Annotation *selected = annotationById(id)) {
            m_annotationsBeforeDrag.append(*selected);
        }
    }
    m_annotationBoundsBeforeDrag = selectedAnnotationsBounds();
    m_dragStart = imagePoint;
    m_dragging = true;
    m_annotationHistoryCaptured = false;
    updateCursor();
    updateAnnotationPropertyPanel();
    update();
}

void ShotWindow::updateAnnotationDrag(QPointF imagePoint, bool keepAspectRatio)
{
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (selectedIds.isEmpty() || m_annotationDrag == SelectionDrag::None) {
        return;
    }
    if (!m_annotationHistoryCaptured) {
        pushHistorySnapshot();
        m_annotationHistoryCaptured = true;
    }

    for (const Annotation &before : m_annotationsBeforeDrag) {
        if (Annotation *annotation = annotationById(before.id)) {
            *annotation = before;
        }
    }

    if (m_annotationDrag == SelectionDrag::Move) {
        const QRectF startBounds = m_annotationBoundsBeforeDrag;
        QPointF delta = clampImagePoint(imagePoint) - m_dragStart;
        delta.setX(std::clamp(delta.x(), -startBounds.left(), m_frozenFrame.width() - startBounds.right()));
        delta.setY(std::clamp(delta.y(), -startBounds.top(), m_frozenFrame.height() - startBounds.bottom()));
        for (int id : selectedIds) {
            if (Annotation *annotation = annotationById(id)) {
                moveAnnotation(*annotation, delta);
            }
        }
    } else {
        const QRectF newBounds = resizedBounds(m_annotationBoundsBeforeDrag, m_annotationDrag, imagePoint, keepAspectRatio);
        for (int id : selectedIds) {
            if (Annotation *annotation = annotationById(id)) {
                transformAnnotation(*annotation, m_annotationBoundsBeforeDrag, newBounds);
            }
        }
    }
    update();
}

void ShotWindow::beginAnnotationSelectionBox(QPointF imagePoint)
{
    setSelectedAnnotations({});
    m_annotationDrag = SelectionDrag::None;
    m_annotationSelectionBoxActive = true;
    m_dragging = true;
    m_dragStart = clampImagePoint(imagePoint);
    m_annotationSelectionBox = QRectF(m_dragStart, m_dragStart);
    updateAnnotationPropertyPanel();
    updateCursor();
    update();
}

void ShotWindow::updateAnnotationSelectionBox(QPointF imagePoint)
{
    m_annotationSelectionBox = QRectF(m_dragStart, clampImagePoint(imagePoint)).normalized();
    update();
}

void ShotWindow::commitAnnotationSelectionBox()
{
    m_annotationSelectionBoxActive = false;
    setSelectedAnnotations(annotationsInRect(m_annotationSelectionBox));
    m_annotationSelectionBox = {};
    updateAnnotationPropertyPanel();
}

ShotWindow::HistorySnapshot ShotWindow::currentHistorySnapshot() const
{
    return {m_annotations, m_selectedAnnotationId, selectedAnnotationIds(), m_nextNumber, m_nextAnnotationId};
}

void ShotWindow::restoreHistorySnapshot(const HistorySnapshot &snapshot)
{
    m_annotations = snapshot.annotations;
    setSelectedAnnotations(snapshot.selectedAnnotationIds.isEmpty()
                               ? (snapshot.selectedAnnotationId.has_value() ? QVector<int>{*snapshot.selectedAnnotationId} : QVector<int>{})
                               : snapshot.selectedAnnotationIds);
    m_nextNumber = snapshot.nextNumber;
    m_nextAnnotationId = snapshot.nextAnnotationId;
    m_draft.reset();
    m_annotationDrag = SelectionDrag::None;
    m_annotationSelectionBoxActive = false;
    m_annotationHistoryCaptured = false;
    updateAnnotationPropertyPanel();
    updateCursor();
    update();
}

void ShotWindow::pushHistorySnapshot()
{
    m_undoStack.append(currentHistorySnapshot());
    if (m_undoStack.size() > 100) {
        m_undoStack.removeFirst();
    }
    m_redoStack.clear();
}

void ShotWindow::undoAnnotationEdit()
{
    if (m_undoStack.isEmpty()) {
        return;
    }
    const HistorySnapshot current = currentHistorySnapshot();
    const HistorySnapshot previous = m_undoStack.takeLast();
    m_redoStack.append(current);
    restoreHistorySnapshot(previous);
}

qreal ShotWindow::currentToolWidth() const
{
    switch (m_tool) {
    case Tool::Move:
    case Tool::Select:
        return m_shapeWidth;
    case Tool::Pen:
        return m_penWidth;
    case Tool::Highlighter:
        return m_penWidth * 2.0;
    case Tool::Line:
    case Tool::Arrow:
    case Tool::Rectangle:
    case Tool::Ellipse:
    case Tool::Text:
        return m_shapeWidth;
    case Tool::Number:
        return m_numberWidth;
    case Tool::Mosaic:
        return m_mosaicBlockSize;
    case Tool::Laser:
        return m_laserWidth;
    }

    return m_shapeWidth;
}

qreal ShotWindow::currentToolPreviewSize() const
{
    const qreal scale = annotationSizeScale(true);

    switch (m_tool) {
    case Tool::Move:
    case Tool::Select:
        return 8.0;
    case Tool::Pen:
    case Tool::Line:
    case Tool::Arrow:
    case Tool::Rectangle:
    case Tool::Ellipse:
        return std::max<qreal>(1.5, currentToolWidth() * scale);
    case Tool::Highlighter:
        return std::max<qreal>(6.0, currentToolWidth() * scale);
    case Tool::Text:
        return std::max<qreal>(10.0, (19.0 + currentToolWidth()) * scale);
    case Tool::Number:
        return std::max<qreal>(26.0, (13.0 + currentToolWidth() * 1.35) * scale * 2.0);
    case Tool::Mosaic:
        return std::max<qreal>(2.0, currentToolWidth() * scale);
    case Tool::Laser:
        return std::max<qreal>(8.0, currentToolWidth() * scale);
    }

    return std::max<qreal>(1.5, currentToolWidth() * scale);
}

void ShotWindow::setCurrentColor(QColor color)
{
    if (!color.isValid()) {
        return;
    }

    m_currentColor = color;
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (m_tool == Tool::Select && !selectedIds.isEmpty()) {
        pushHistorySnapshot();
        for (int id : selectedIds) {
            if (Annotation *annotation = annotationById(id)) {
                annotation->color = color;
            }
        }
        updateAnnotationPropertyPanel();
    }
    if (m_draft.has_value()) {
        m_draft->color = color;
    }
    if (m_colorPalette) {
        m_colorPalette->hide();
    }
    if (m_textEditor && m_textEditor->isVisible() && !m_editingTextAnnotationId.has_value()) {
        m_textEditor->setStyleSheet(markshot::theme::textEditorStyleSheet(m_currentColor, m_textBackgroundColor, qRound(20.0 + m_shapeWidth)));
    }
    updateColorPalettePreview();
    updateAnnotationPropertyPanel();
    update();
}

void ShotWindow::revealSelectionInfo()
{
    m_showSelectionInfo = true;
    m_selectionInfoTimer.restart();
    QTimer::singleShot(1000, this, [this] {
        if (m_selectionDrag == SelectionDrag::None
            && m_selectionInfoTimer.isValid()
            && m_selectionInfoTimer.elapsed() >= 1000) {
            m_showSelectionInfo = false;
            update();
        }
    });
}

QRectF ShotWindow::normalizedSelection() const
{
    return m_selection.normalized().intersected(QRectF(QPointF(0, 0), QSizeF(m_frozenFrame.size())));
}

QString ShotWindow::slurpSelectionGeometry() const
{
    if (!hasUsableSelection()) {
        return {};
    }

    const QRect sourceBounds(QPoint(0, 0), m_frozenFrame.size());
    QRect selectionRect = normalizedSelection().toAlignedRect().intersected(sourceBounds);
    if (selectionRect.isEmpty()) {
        return {};
    }

    if (m_sourceGeometry.isValid() && !m_sourceGeometry.isEmpty()) {
        selectionRect.translate(m_sourceGeometry.topLeft());
    }
    return slurpGeometry(selectionRect);
}

QPointF ShotWindow::widgetToImage(QPointF point) const
{
    if (m_frozenImageRect.isEmpty() || m_frozenFrame.isNull()) {
        return {};
    }

    const qreal x = (point.x() - m_frozenImageRect.left()) * m_frozenFrame.width() / m_frozenImageRect.width();
    const qreal y = (point.y() - m_frozenImageRect.top()) * m_frozenFrame.height() / m_frozenImageRect.height();
    return clampImagePoint({x, y});
}

QPointF ShotWindow::imageToWidget(QPointF point) const
{
    if (m_frozenImageRect.isEmpty() || m_frozenFrame.isNull()) {
        return {};
    }

    const qreal x = m_frozenImageRect.left() + point.x() * m_frozenImageRect.width() / m_frozenFrame.width();
    const qreal y = m_frozenImageRect.top() + point.y() * m_frozenImageRect.height() / m_frozenFrame.height();
    return {x, y};
}

QPointF ShotWindow::clampImagePoint(QPointF point) const
{
    return {
        std::clamp(point.x(), 0.0, static_cast<qreal>(std::max(0, m_frozenFrame.width() - 1))),
        std::clamp(point.y(), 0.0, static_cast<qreal>(std::max(0, m_frozenFrame.height() - 1))),
    };
}

QString ShotWindow::currentToolName() const
{
    switch (m_tool) {
    case Tool::Move:
        return QStringLiteral("Move");
    case Tool::Select:
        return QStringLiteral("Select");
    case Tool::Pen:
        return QStringLiteral("Pen");
    case Tool::Line:
        return QStringLiteral("Line");
    case Tool::Highlighter:
        return QStringLiteral("Highlighter");
    case Tool::Rectangle:
        return QStringLiteral("Rect");
    case Tool::Ellipse:
        return QStringLiteral("Ellipse");
    case Tool::Arrow:
        return QStringLiteral("Arrow");
    case Tool::Text:
        return QStringLiteral("Text");
    case Tool::Number:
        return QStringLiteral("Number");
    case Tool::Mosaic:
        return QStringLiteral("Mosaic");
    case Tool::Laser:
        return QStringLiteral("Laser");
    }

    return QStringLiteral("Tool");
}

QImage ShotWindow::mosaicImage(QRect sourceRect, int blockSize) const
{
    sourceRect = sourceRect.normalized().intersected(QRect(QPoint(0, 0), m_frozenFrame.size()));
    if (sourceRect.isEmpty()) {
        return {};
    }

    blockSize = std::clamp(blockSize, 2, 96);
    const QImage source = m_frozenFrame.copy(sourceRect).convertToFormat(QImage::Format_ARGB32_Premultiplied);
    QImage output(source.size(), QImage::Format_ARGB32_Premultiplied);
    output.fill(Qt::transparent);

    QPainter blockPainter(&output);
    blockPainter.setPen(Qt::NoPen);
    blockPainter.setRenderHint(QPainter::Antialiasing, false);

    for (int y = 0; y < source.height(); y += blockSize) {
        const int blockHeight = std::min(blockSize, source.height() - y);
        for (int x = 0; x < source.width(); x += blockSize) {
            const int blockWidth = std::min(blockSize, source.width() - x);
            quint64 red = 0;
            quint64 green = 0;
            quint64 blue = 0;
            quint64 alpha = 0;
            for (int py = y; py < y + blockHeight; ++py) {
                const QRgb *line = reinterpret_cast<const QRgb *>(source.constScanLine(py));
                for (int px = x; px < x + blockWidth; ++px) {
                    const QRgb pixel = line[px];
                    red += qRed(pixel);
                    green += qGreen(pixel);
                    blue += qBlue(pixel);
                    alpha += qAlpha(pixel);
                }
            }

            const int count = blockWidth * blockHeight;
            QColor average(qRound(static_cast<double>(red) / count),
                           qRound(static_cast<double>(green) / count),
                           qRound(static_cast<double>(blue) / count),
                           qRound(static_cast<double>(alpha) / count));
            blockPainter.setBrush(average);
            blockPainter.drawRect(QRect(x, y, blockWidth, blockHeight));
        }
    }

    blockPainter.end();
    return output;
}

QRectF ShotWindow::imageRectToWidget(QRectF rect) const
{
    const QPointF topLeft = imageToWidget(rect.topLeft());
    const QPointF bottomRight = imageToWidget(rect.bottomRight());
    return QRectF(topLeft, bottomRight).normalized();
}

QRectF ShotWindow::textContentRect(const Annotation &annotation, bool widgetCoordinates) const
{
    const qreal scale = annotationSizeScale(widgetCoordinates);
    const QRectF baseRect = annotation.rect.isEmpty()
        ? QRectF(annotation.points.value(0), QSizeF(360.0, 140.0))
        : annotation.rect.normalized();
    const QPointF topLeft = widgetCoordinates ? imageToWidget(baseRect.topLeft()) : baseRect.topLeft();
    const qreal wrapWidth = std::max<qreal>(16.0, baseRect.width() * scale - kTextBackgroundPaddingX * 2.0 * scale);

    QFont font(annotation.fontFamily.isEmpty() ? QStringLiteral("Sans Serif") : annotation.fontFamily,
               qRound((19.0 + annotation.width) * scale),
               QFont::DemiBold);
    QTextOption option;
    option.setWrapMode(QTextOption::WrapAtWordBoundaryOrAnywhere);
    option.setAlignment(Qt::AlignLeft | Qt::AlignTop);

    QTextDocument document;
    document.setDocumentMargin(0.0);
    document.setDefaultFont(font);
    document.setDefaultTextOption(option);
    document.setPlainText(annotation.text);
    document.setTextWidth(wrapWidth);

    const QSizeF documentSize = document.size();
    qreal textWidth = 0.0;
    qreal textHeight = 0.0;
    for (QTextBlock block = document.begin(); block.isValid(); block = block.next()) {
        const QTextLayout *layout = block.layout();
        if (!layout) {
            continue;
        }
        for (int i = 0; i < layout->lineCount(); ++i) {
            const QTextLine line = layout->lineAt(i);
            textWidth = std::max(textWidth, line.naturalTextWidth());
            textHeight = std::max(textHeight, layout->position().y() + line.y() + line.height());
        }
    }
    if (textWidth <= 0.0 || textHeight <= 0.0) {
        textWidth = documentSize.width();
        textHeight = documentSize.height();
    }

    const qreal rectWidth = std::max<qreal>(1.0, std::ceil(textWidth + kTextBackgroundPaddingX * 2.0 * scale));
    const qreal rectHeight = std::max<qreal>(1.0, std::ceil(textHeight + kTextBackgroundPaddingY * 2.0 * scale));
    return QRectF(topLeft, QSizeF(rectWidth, rectHeight));
}

QRectF ShotWindow::constrainedRect(QPointF start, QPointF end) const
{
    const qreal dx = end.x() - start.x();
    const qreal dy = end.y() - start.y();
    const qreal side = std::max(std::abs(dx), std::abs(dy));
    const QPointF constrainedEnd(start.x() + std::copysign(side, dx == 0.0 ? 1.0 : dx),
                                 start.y() + std::copysign(side, dy == 0.0 ? 1.0 : dy));
    return normalizedRect(start, clampImagePoint(constrainedEnd));
}

void ShotWindow::updateFrozenImageRect()
{
    if (m_frozenFrame.isNull()) {
        m_frozenImageRect = {};
        m_imageCenterInitialized = false;
        return;
    }

    QSizeF frameSize = m_frozenFrame.size();
    frameSize.scale(size(), Qt::KeepAspectRatio);
    if (!imageNavigationAvailable()) {
        const QPointF topLeft((width() - frameSize.width()) / 2.0, (height() - frameSize.height()) / 2.0);
        m_frozenImageRect = QRectF(topLeft, frameSize);
        return;
    }

    const qreal fitScale = frameSize.width() / std::max<qreal>(1.0, m_frozenFrame.width());
    const qreal scale = fitScale * m_imageZoom;
    frameSize = QSizeF(m_frozenFrame.width() * scale, m_frozenFrame.height() * scale);
    if (!m_imageCenterInitialized) {
        m_imageCenter = QPointF(m_frozenFrame.width() / 2.0, m_frozenFrame.height() / 2.0);
        m_imageCenterInitialized = true;
    }

    if (frameSize.width() <= width()) {
        m_imageCenter.setX(m_frozenFrame.width() / 2.0);
    } else {
        const qreal halfVisibleWidth = width() / (2.0 * scale);
        m_imageCenter.setX(std::clamp(m_imageCenter.x(), halfVisibleWidth, m_frozenFrame.width() - halfVisibleWidth));
    }
    if (frameSize.height() <= height()) {
        m_imageCenter.setY(m_frozenFrame.height() / 2.0);
    } else {
        const qreal halfVisibleHeight = height() / (2.0 * scale);
        m_imageCenter.setY(std::clamp(m_imageCenter.y(), halfVisibleHeight, m_frozenFrame.height() - halfVisibleHeight));
    }

    const QPointF widgetCenter(width() / 2.0, height() / 2.0);
    const QPointF topLeft(widgetCenter.x() - m_imageCenter.x() * scale,
                          widgetCenter.y() - m_imageCenter.y() * scale);
    m_frozenImageRect = QRectF(topLeft, frameSize);
}

void ShotWindow::zoomImageAt(qreal factor, QPointF widgetAnchor)
{
    if (!imageNavigationAvailable() || m_frozenFrame.isNull() || m_frozenImageRect.isEmpty() || factor <= 0.0) {
        return;
    }

    const QPointF anchorImage = m_frozenImageRect.contains(widgetAnchor)
        ? widgetToImage(widgetAnchor)
        : (m_imageCenterInitialized ? m_imageCenter : QPointF(m_frozenFrame.width() / 2.0, m_frozenFrame.height() / 2.0));
    m_imageZoom = std::clamp(m_imageZoom * factor, kMinImageZoom, kMaxImageZoom);

    QSizeF fitSize = m_frozenFrame.size();
    fitSize.scale(size(), Qt::KeepAspectRatio);
    const qreal scale = fitSize.width() / std::max<qreal>(1.0, m_frozenFrame.width()) * m_imageZoom;
    const QPointF widgetCenter(width() / 2.0, height() / 2.0);
    m_imageCenter = QPointF(anchorImage.x() - (widgetAnchor.x() - widgetCenter.x()) / scale,
                            anchorImage.y() - (widgetAnchor.y() - widgetCenter.y()) / scale);
    m_imageCenterInitialized = true;
    updateFrozenImageRect();
    refreshViewGeometry();
    update();
}

void ShotWindow::resetImageZoom()
{
    if (m_frozenFrame.isNull()) {
        return;
    }

    m_imageZoom = 1.0;
    m_imageCenter = QPointF(m_frozenFrame.width() / 2.0, m_frozenFrame.height() / 2.0);
    m_imageCenterInitialized = true;
    updateFrozenImageRect();
    refreshViewGeometry();
    update();
}

void ShotWindow::panImageTo(QPointF widgetPosition)
{
    if (!imageNavigationAvailable() || m_frozenFrame.isNull() || m_frozenImageRect.isEmpty()) {
        return;
    }

    const qreal scale = m_frozenImageRect.width() / std::max<qreal>(1.0, m_frozenFrame.width());
    if (scale <= 0.0) {
        return;
    }

    const QPointF delta = widgetPosition - m_imagePanStartWidget;
    m_imageCenter = m_imagePanStartCenter - delta / scale;
    m_imageCenterInitialized = true;
    updateFrozenImageRect();
    refreshViewGeometry();
    update();
}

void ShotWindow::refreshViewGeometry()
{
    updateTextEditorGeometry();
    updateToolbarGeometry();
    updateActionToolbarGeometry();
    updateAnnotationPropertyPanelGeometry();
    updateOpenWithPanelGeometry();
    updateExtensionPanelGeometry();
}

QRect ShotWindow::clampedToolbarGeometry(QRect toolbarGeometry) const
{
    toolbarGeometry.moveLeft(std::clamp(toolbarGeometry.left(), 8, std::max(8, width() - toolbarGeometry.width() - 8)));
    toolbarGeometry.moveTop(std::clamp(toolbarGeometry.top(), 8, std::max(8, height() - toolbarGeometry.height() - 8)));
    return toolbarGeometry;
}

void ShotWindow::updateToolbarGeometry()
{
    if (!m_toolbar || !hasUsableSelection()) {
        return;
    }

    m_toolbar->adjustSize();
    if (m_fullscreenAnnotation && m_toolbarUserPlaced) {
        const QSize toolbarSize = m_toolbar->sizeHint();
        QRect toolbarGeometry = m_toolbar->geometry();
        toolbarGeometry.setSize(toolbarSize);
        m_toolbar->setGeometry(clampedToolbarGeometry(toolbarGeometry));
        updateAnnotationPropertyPanelGeometry();
        return;
    }
    if (m_imageNavigationEnabled && m_fullscreenAnnotation) {
        const QSize toolbarSize = m_toolbar->sizeHint();
        const QRect toolbarGeometry(QPoint(qRound((width() - toolbarSize.width()) / 2.0), 12), toolbarSize);
        m_toolbar->setGeometry(clampedToolbarGeometry(toolbarGeometry));
        updateAnnotationPropertyPanelGeometry();
        return;
    }

    const QRectF selection = imageRectToWidget(normalizedSelection());
    const QSize toolbarSize = m_toolbar->sizeHint();
    int x = qRound(selection.center().x() - toolbarSize.width() / 2.0);
    int y = qRound(selection.bottom() + kToolbarMargin);

    x = std::clamp(x, 8, std::max(8, width() - toolbarSize.width() - 8));
    if (y + toolbarSize.height() > height() - 8) {
        y = qRound(selection.top() - toolbarSize.height() - kToolbarMargin);
    }
    y = std::clamp(y, 8, std::max(8, height() - toolbarSize.height() - 8));
    m_toolbar->setGeometry(x, y, toolbarSize.width(), toolbarSize.height());
    updateAnnotationPropertyPanelGeometry();
}

void ShotWindow::updateActionToolbarGeometry()
{
    if (!m_actionToolbar || !hasUsableSelection() || m_fullscreenAnnotation) {
        return;
    }

    m_actionToolbar->adjustSize();
    const QRectF selection = imageRectToWidget(normalizedSelection());
    const QSize toolbarSize = m_actionToolbar->sizeHint();
    const QRect selectionRect = selection.toAlignedRect();
    const QRect toolbarRect = m_toolbar && m_toolbar->isVisible() ? m_toolbar->geometry() : QRect();
    const QRect propertyRect = m_annotationPropertyPanel && m_annotationPropertyPanel->isVisible()
        ? m_annotationPropertyPanel->geometry()
        : QRect();

    auto clamped = [this, toolbarSize](QPoint topLeft) {
        const int x = std::clamp(topLeft.x(), 8, std::max(8, width() - toolbarSize.width() - 8));
        const int y = std::clamp(topLeft.y(), 8, std::max(8, height() - toolbarSize.height() - 8));
        return QRect(QPoint(x, y), toolbarSize);
    };
    auto clearOfPanels = [toolbarRect, propertyRect, selectionRect](const QRect &candidate) {
        const QRect padded = candidate.adjusted(-4, -4, 4, 4);
        return (toolbarRect.isNull() || !padded.intersects(toolbarRect))
            && (propertyRect.isNull() || !padded.intersects(propertyRect))
            && !padded.intersects(selectionRect);
    };

    const int selectionCenterY = qRound(selection.center().y() - toolbarSize.height() / 2.0);
    QVector<QRect> candidates = {
        clamped(QPoint(qRound(selection.right() + kToolbarMargin), selectionCenterY)),
        clamped(QPoint(qRound(selection.left() - toolbarSize.width() - kToolbarMargin), selectionCenterY)),
    };
    if (!toolbarRect.isNull()) {
        candidates.append(clamped(QPoint(toolbarRect.right() + kToolbarMargin, toolbarRect.top())));
        candidates.append(clamped(QPoint(toolbarRect.left() - toolbarSize.width() - kToolbarMargin, toolbarRect.top())));
        candidates.append(clamped(QPoint(toolbarRect.right() - toolbarSize.width(), toolbarRect.bottom() + kToolbarMargin)));
        candidates.append(clamped(QPoint(toolbarRect.right() - toolbarSize.width(), toolbarRect.top() - toolbarSize.height() - kToolbarMargin)));
    }

    for (const QRect &candidate : candidates) {
        if (clearOfPanels(candidate)) {
            m_actionToolbar->setGeometry(candidate);
            return;
        }
    }
    m_actionToolbar->setGeometry(candidates.first());
}

void ShotWindow::updateAnnotationPropertyPanel()
{
    if (!m_annotationPropertyPanel) {
        return;
    }

    const QVector<int> selectedIds = selectedAnnotationIds();
    const Annotation *annotation = selectedIds.size() == 1
        ? annotationById(selectedIds.first())
        : nullptr;
    const Annotation *firstSelectedAnnotation = !selectedIds.isEmpty()
        ? annotationById(selectedIds.first())
        : nullptr;
    const bool groupSelection = m_tool == Tool::Select && selectedIds.size() > 1;
    const bool editingAnnotation = m_tool == Tool::Select && !selectedIds.isEmpty();
    const bool editingTool = m_mode == Mode::Editing
        && m_tool != Tool::Move
        && m_tool != Tool::Select;
    if (!editingAnnotation && !editingTool) {
        m_annotationPropertyPanel->hide();
        if (m_propertyColorDialogPanel) {
            m_propertyColorDialogPanel->hide();
        }
        if (m_propertyFontPanel) {
            m_propertyFontPanel->hide();
        }
        return;
    }

    QString title = QStringLiteral("Object");
    const Tool panelTool = groupSelection ? Tool::Select : (annotation ? annotation->tool : m_tool);
    const QColor panelColor = firstSelectedAnnotation ? firstSelectedAnnotation->color : m_currentColor;
    const QColor panelTextBackgroundColor = annotation && annotation->tool == Tool::Text
        ? annotation->backgroundColor
        : m_textBackgroundColor;
    const qreal panelWidth = firstSelectedAnnotation ? firstSelectedAnnotation->width : currentToolWidth();
    const int panelOpacity = qRound(panelColor.alphaF() * 100.0);
    const bool panelFilled = annotation ? annotation->filled : m_shapeFilled;
    const qreal panelRadius = annotation ? annotation->cornerRadius : m_rectangleCornerRadius;
    const QString panelFontFamily = annotation ? annotation->fontFamily : m_textFontFamily;

    switch (panelTool) {
    case Tool::Move:
    case Tool::Select:
        title = QStringLiteral("Object");
        break;
    case Tool::Pen:
        title = QStringLiteral("Pen");
        break;
    case Tool::Highlighter:
        title = QStringLiteral("Highlighter");
        break;
    case Tool::Line:
        title = QStringLiteral("Line");
        break;
    case Tool::Rectangle:
        title = QStringLiteral("Rect");
        break;
    case Tool::Ellipse:
        title = QStringLiteral("Ellipse");
        break;
    case Tool::Arrow:
        title = QStringLiteral("Arrow");
        break;
    case Tool::Text:
        title = QStringLiteral("Text");
        break;
    case Tool::Number:
        title = QStringLiteral("Number");
        break;
    case Tool::Mosaic:
        title = QStringLiteral("Mosaic");
        break;
    case Tool::Laser:
        title = QStringLiteral("Laser");
        break;
    }

    if (m_annotationPropertyTitle) {
        m_annotationPropertyTitle->setText(groupSelection
                                               ? MS_TR("Group %1").arg(selectedIds.size())
                                               : markshot::i18n::translate(title));
    }
    if (m_propertyEditTextButton) {
        m_propertyEditTextButton->setVisible(!groupSelection && editingAnnotation && panelTool == Tool::Text);
    }
    if (m_propertyFontButton) {
        m_propertyFontButton->setVisible(!groupSelection && panelTool == Tool::Text);
        if (!groupSelection && panelTool == Tool::Text) {
            const QString family = panelFontFamily.isEmpty() ? QStringLiteral("Sans Serif") : panelFontFamily;
            m_propertyFontButton->setText(MS_TR("Font"));
            m_propertyFontButton->setToolTip(family);
            if (m_propertyFontList) {
                const auto matches = m_propertyFontList->findItems(family, Qt::MatchExactly);
                if (!matches.isEmpty()) {
                    m_propertyFontList->setCurrentItem(matches.first());
                    m_propertyFontList->scrollToItem(matches.first(), QAbstractItemView::PositionAtCenter);
                }
            }
        } else if (m_propertyFontPanel) {
            m_propertyFontPanel->hide();
            m_propertyFontButton->setToolTip(MS_TR("Text font"));
        }
    }
    if (m_propertyFillButton) {
        const bool supportsFill = !groupSelection && (panelTool == Tool::Rectangle || panelTool == Tool::Ellipse);
        m_propertyFillButton->setVisible(supportsFill);
        const QSignalBlocker blocker(m_propertyFillButton);
        m_propertyFillButton->setChecked(panelFilled);
        m_propertyFillButton->setIcon(markshot::ui::makeFillIcon(panelFilled));
    }
    if (m_propertyRadiusLabel) {
        m_propertyRadiusLabel->setVisible(!groupSelection && panelTool == Tool::Rectangle);
        m_propertyRadiusLabel->setText(MS_TR("Radius %1").arg(qRound(panelRadius)));
    }
    if (m_propertyRadiusSlider) {
        m_propertyRadiusSlider->setVisible(!groupSelection && panelTool == Tool::Rectangle);
        const QSignalBlocker blocker(m_propertyRadiusSlider);
        m_propertyRadiusSlider->setValue(qRound(panelRadius));
    }
    if (m_propertyWidthLabel) {
        m_propertyWidthLabel->setText(MS_TR("Width %1").arg(qRound(panelWidth)));
    }
    if (m_propertyWidthSlider) {
        const QSignalBlocker blocker(m_propertyWidthSlider);
        if (panelTool == Tool::Mosaic) {
            m_propertyWidthSlider->setRange(qRound(kMinMosaicBlockSize), qRound(kMaxMosaicBlockSize));
        } else if (panelTool == Tool::Number) {
            m_propertyWidthSlider->setRange(qRound(kMinNumberWidth), qRound(kMaxNumberWidth));
        } else if (panelTool == Tool::Laser) {
            m_propertyWidthSlider->setRange(qRound(kMinLaserWidth), qRound(kMaxLaserWidth));
        } else if (panelTool == Tool::Text) {
            m_propertyWidthSlider->setRange(1, 1000);
        } else {
            m_propertyWidthSlider->setRange(qRound(kMinStrokeWidth), qRound(kMaxStrokeWidth));
        }
        m_propertyWidthSlider->setValue(qRound(panelWidth));
    }
    if (m_propertyOpacityLabel) {
        m_propertyOpacityLabel->setText(MS_TR("Opacity %1%").arg(panelOpacity));
    }
    if (m_propertyOpacitySlider) {
        const QSignalBlocker blocker(m_propertyOpacitySlider);
        m_propertyOpacitySlider->setValue(panelOpacity);
    }
    if (m_propertyColorButton) {
        m_propertyColorButton->setStyleSheet(markshot::theme::propertyColorButtonStyleSheet(panelColor));
        m_propertyColorButton->setVisible(panelTool != Tool::Mosaic);
        if (panelTool == Tool::Mosaic && m_propertyColorDialogPanel) {
            m_propertyColorDialogPanel->hide();
        }
    }
    if (m_propertyTextBackgroundButton) {
        const bool supportsTextBackground = !groupSelection && panelTool == Tool::Text;
        m_propertyTextBackgroundButton->setVisible(supportsTextBackground);
        m_propertyTextBackgroundButton->setStyleSheet(markshot::theme::propertyColorButtonStyleSheet(panelTextBackgroundColor));
        if (!supportsTextBackground && m_propertyColorDialogPanel && m_propertyColorEditingTextBackground) {
            m_propertyColorDialogPanel->hide();
        }
    }
    if (m_propertyColorPicker && m_propertyColorDialogPanel && m_propertyColorDialogPanel->isVisible()) {
        const QSignalBlocker blocker(m_propertyColorPicker);
        m_propertyColorPicker->setColor(m_propertyColorEditingTextBackground ? panelTextBackgroundColor : panelColor);
    }

    m_annotationPropertyPanel->show();
    if (QLayout *panelLayout = m_annotationPropertyPanel->layout()) {
        panelLayout->activate();
    }
    updateAnnotationPropertyPanelGeometry();
    m_annotationPropertyPanel->raise();
    if (m_propertyColorDialogPanel && m_propertyColorDialogPanel->isVisible()) {
        updatePropertyColorDialogGeometry();
        m_propertyColorDialogPanel->raise();
    }
}

void ShotWindow::updateAnnotationPropertyPanelGeometry()
{
    if (!m_annotationPropertyPanel) {
        return;
    }

    m_annotationPropertyPanel->adjustSize();
    const QSize panelSize = m_annotationPropertyPanel->sizeHint();
    const QRect toolbarRect = m_toolbar && m_toolbar->isVisible()
        ? m_toolbar->geometry()
        : QRect(8, 8, 0, 0);
    int x = toolbarRect.left();
    int y = toolbarRect.bottom() + kToolbarMargin;
    if (y + panelSize.height() > height() - 8) {
        y = toolbarRect.top() - panelSize.height() - kToolbarMargin;
    }
    if (x + panelSize.width() > width() - 8) {
        x = toolbarRect.right() - panelSize.width();
    }
    x = std::clamp(x, 8, std::max(8, width() - panelSize.width() - 8));
    y = std::clamp(y, 8, std::max(8, height() - panelSize.height() - 8));
    m_annotationPropertyPanel->setGeometry(x, y, panelSize.width(), panelSize.height());
    updatePropertyColorDialogGeometry();
    updatePropertyFontPanelGeometry();
}

void ShotWindow::updatePropertyColorDialogGeometry()
{
    if (!m_propertyColorDialogPanel || !m_annotationPropertyPanel) {
        return;
    }

    m_propertyColorDialogPanel->adjustSize();
    QSize panelSize = m_propertyColorDialogPanel->sizeHint();
    panelSize.setWidth(std::min(panelSize.width(), std::max(160, width() - 16)));
    panelSize.setHeight(std::min(panelSize.height(), std::max(180, height() - 16)));

    // Anchor on the color button's centre so the picker stays put when the
    // property panel resizes (e.g. fill/radius/font slots showing or hiding).
    // Falling back to the property panel keeps geometry valid when the
    // button is hidden (mosaic case).
    QPoint anchor;
    if (m_propertyColorEditingTextBackground && m_propertyTextBackgroundButton && m_propertyTextBackgroundButton->isVisible()) {
        anchor = m_propertyTextBackgroundButton->mapTo(this, m_propertyTextBackgroundButton->rect().center());
    } else if (m_propertyColorButton && m_propertyColorButton->isVisible()) {
        anchor = m_propertyColorButton->mapTo(this, m_propertyColorButton->rect().center());
    } else {
        const QRect propertyRect = m_annotationPropertyPanel->geometry();
        anchor = QPoint(propertyRect.center().x(), propertyRect.bottom());
    }

    int x = anchor.x() - panelSize.width() / 2;
    int y = anchor.y() + 14;

    const QRect propertyRect = m_annotationPropertyPanel->geometry();
    if (y + panelSize.height() > height() - 8) {
        // Place above the property panel instead.
        y = propertyRect.top() - panelSize.height() - 8;
    }
    x = std::clamp(x, 8, std::max(8, width() - panelSize.width() - 8));
    y = std::clamp(y, 8, std::max(8, height() - panelSize.height() - 8));
    m_propertyColorDialogPanel->setGeometry(x, y, panelSize.width(), panelSize.height());
}

void ShotWindow::updatePropertyFontPanelGeometry()
{
    if (!m_propertyFontPanel || !m_annotationPropertyPanel || !m_propertyFontButton) {
        return;
    }

    const int visibleRows = std::min(10, m_propertyFontList ? std::max(1, m_propertyFontList->count()) : 1);
    const int rowHeight = m_propertyFontList ? std::max(24, m_propertyFontList->sizeHintForRow(0)) : 28;
    QSize panelSize(260, std::min(280, visibleRows * rowHeight + 18));
    panelSize.setWidth(std::min(panelSize.width(), std::max(180, width() - 16)));
    panelSize.setHeight(std::min(panelSize.height(), std::max(120, height() - 16)));

    QPoint anchor = m_propertyFontButton->mapTo(this, QPoint(m_propertyFontButton->width() / 2,
                                                            m_propertyFontButton->height()));
    int x = anchor.x() - panelSize.width() / 2;
    int y = anchor.y() + 10;
    const QRect propertyRect = m_annotationPropertyPanel->geometry();
    if (y + panelSize.height() > height() - 8) {
        y = propertyRect.top() - panelSize.height() - 8;
    }
    x = std::clamp(x, 8, std::max(8, width() - panelSize.width() - 8));
    y = std::clamp(y, 8, std::max(8, height() - panelSize.height() - 8));
    if (m_propertyFontList) {
        m_propertyFontList->setFixedHeight(std::max(80, panelSize.height() - 16));
    }
    m_propertyFontPanel->setFixedSize(panelSize);
    m_propertyFontPanel->setGeometry(x, y, panelSize.width(), panelSize.height());
}

void ShotWindow::adjustSelectedAnnotationWidth(qreal delta)
{
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (selectedIds.isEmpty()) {
        return;
    }

    pushHistorySnapshot();
    for (int id : selectedIds) {
        if (Annotation *annotation = annotationById(id)) {
            if (annotation->tool == Tool::Mosaic) {
                annotation->width = std::clamp(annotation->width + delta * 2.0, kMinMosaicBlockSize, kMaxMosaicBlockSize);
            } else if (annotation->tool == Tool::Number) {
                annotation->width = std::clamp(annotation->width + delta * 2.0, kMinNumberWidth, kMaxNumberWidth);
            } else {
                annotation->width = std::clamp(annotation->width + delta, kMinStrokeWidth, kMaxStrokeWidth);
            }
        }
    }
    updateAnnotationPropertyPanel();
    update();
}

void ShotWindow::setSelectedAnnotationWidth(int width)
{
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (!selectedIds.isEmpty()) {
        bool changed = false;
        for (int id : selectedIds) {
            const Annotation *annotation = annotationById(id);
            if (annotation && qRound(annotation->width) != width) {
                changed = true;
                break;
            }
        }
        if (!changed) {
            return;
        }
        pushHistorySnapshot();
        for (int id : selectedIds) {
            if (Annotation *annotation = annotationById(id)) {
                if (annotation->tool == Tool::Mosaic) {
                    annotation->width = std::clamp<qreal>(width, kMinMosaicBlockSize, kMaxMosaicBlockSize);
                } else if (annotation->tool == Tool::Number) {
                    annotation->width = std::clamp<qreal>(width, kMinNumberWidth, kMaxNumberWidth);
                } else if (annotation->tool == Tool::Text) {
                    const qreal oldWidth = annotation->width;
                    annotation->width = std::clamp<qreal>(width, 1.0, 1000.0);
                    const qreal factor = ((19.0 + annotation->width) / (19.0 + oldWidth)) * 1.05;
                    annotation->rect.setWidth(annotation->rect.width() * factor);
                    annotation->rect = textContentRect(*annotation, false);
                    if (!annotation->points.isEmpty()) {
                        annotation->points[0] = annotation->rect.topLeft();
                    }
                } else {
                    annotation->width = std::clamp<qreal>(width, kMinStrokeWidth, kMaxStrokeWidth);
                }
            }
        }
    } else {
        switch (m_tool) {
        case Tool::Pen:
        case Tool::Highlighter:
            m_penWidth = width;
            break;
        case Tool::Mosaic:
            m_mosaicBlockSize = width;
            break;
        case Tool::Laser:
            m_laserWidth = std::clamp<qreal>(width, kMinLaserWidth, kMaxLaserWidth);
            break;
        case Tool::Move:
        case Tool::Select:
            return;
        case Tool::Line:
        case Tool::Rectangle:
        case Tool::Ellipse:
        case Tool::Arrow:
            m_shapeWidth = width;
            break;
        case Tool::Text:
            m_shapeWidth = std::clamp<qreal>(width, 1.0, 1000.0);
            break;
        case Tool::Number:
            m_numberWidth = std::clamp<qreal>(width, kMinNumberWidth, kMaxNumberWidth);
            break;
        }
    }
    updateAnnotationPropertyPanel();
    updateColorPalettePreview();
    update();
}

void ShotWindow::setSelectedAnnotationOpacity(int opacity)
{
    opacity = std::clamp(opacity, 0, 100);
    const int alpha = qRound(opacity * 255.0 / 100.0);
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (!selectedIds.isEmpty()) {
        bool changed = false;
        for (int id : selectedIds) {
            const Annotation *annotation = annotationById(id);
            if (annotation && annotation->color.alpha() != alpha) {
                changed = true;
                break;
            }
        }
        if (!changed) {
            return;
        }
        pushHistorySnapshot();
        for (int id : selectedIds) {
            if (Annotation *annotation = annotationById(id)) {
                annotation->color.setAlpha(alpha);
            }
        }
    } else {
        if (m_currentColor.alpha() == alpha) {
            return;
        }
        m_currentColor.setAlpha(alpha);
    }

    if (m_draft.has_value()) {
        m_draft->color.setAlpha(alpha);
    }
    if (m_laserDraft.has_value()) {
        m_laserDraft->color.setAlpha(alpha);
    }
    if (m_propertyColorPicker && m_propertyColorDialogPanel && m_propertyColorDialogPanel->isVisible()) {
        const QSignalBlocker blocker(m_propertyColorPicker);
        m_propertyColorPicker->setColor(selectedIds.isEmpty() ? m_currentColor : annotationById(selectedIds.first())->color);
    }
    updateColorPalettePreview();
    updateAnnotationPropertyPanel();
    update();
}

void ShotWindow::setSelectedAnnotationFilled(bool filled)
{
    if (m_selectedAnnotationId.has_value()) {
        Annotation *annotation = annotationById(*m_selectedAnnotationId);
        if (!annotation || annotation->filled == filled) {
            return;
        }
        if (annotation->tool != Tool::Rectangle && annotation->tool != Tool::Ellipse) {
            return;
        }
        pushHistorySnapshot();
        annotation->filled = filled;
    } else {
        if (m_tool != Tool::Rectangle && m_tool != Tool::Ellipse) {
            return;
        }
        m_shapeFilled = filled;
    }
    updateAnnotationPropertyPanel();
    update();
}

void ShotWindow::setSelectedAnnotationCornerRadius(int radius)
{
    if (m_selectedAnnotationId.has_value()) {
        Annotation *annotation = annotationById(*m_selectedAnnotationId);
        if (!annotation || annotation->tool != Tool::Rectangle || qRound(annotation->cornerRadius) == radius) {
            return;
        }
        pushHistorySnapshot();
        annotation->cornerRadius = radius;
    } else {
        if (m_tool != Tool::Rectangle || qRound(m_rectangleCornerRadius) == radius) {
            return;
        }
        m_rectangleCornerRadius = radius;
    }
    updateAnnotationPropertyPanel();
    update();
}

void ShotWindow::deleteSelectedAnnotation()
{
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (selectedIds.isEmpty()) {
        return;
    }
    pushHistorySnapshot();
    for (int i = m_annotations.size() - 1; i >= 0; --i) {
        if (selectedIds.contains(m_annotations.at(i).id)) {
            m_annotations.removeAt(i);
        }
    }
    setSelectedAnnotations({});
    m_annotationDrag = SelectionDrag::None;
    updateAnnotationPropertyPanel();
    updateCursor();
    update();
}

void ShotWindow::openSelectedAnnotationColorPalette()
{
    if (!m_propertyColorDialogPanel || !m_propertyColorPicker || !m_annotationPropertyPanel) {
        return;
    }
    const bool wasEditingTextBackground = m_propertyColorEditingTextBackground;
    m_propertyColorEditingTextBackground = false;

    if (m_propertyColorDialogPanel->isVisible() && !wasEditingTextBackground) {
        m_propertyColorDialogPanel->hide();
        return;
    }

    if (m_colorPalette) {
        m_colorPalette->hide();
    }
    QColor color = m_currentColor;
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (!selectedIds.isEmpty()) {
        if (const Annotation *annotation = annotationById(selectedIds.first())) {
            color = annotation->color;
        }
    }
    m_propertyColorEditHistoryCaptured = false;
    {
        const QSignalBlocker blocker(m_propertyColorPicker);
        m_propertyColorPicker->setColor(color);
    }
    updateAnnotationPropertyPanel();
    if (m_annotationPropertyPanel) {
        m_annotationPropertyPanel->show();
        m_annotationPropertyPanel->raise();
        if (QLayout *panelLayout = m_annotationPropertyPanel->layout()) {
            panelLayout->activate();
        }
        updateAnnotationPropertyPanelGeometry();
    }
    if (QLayout *colorLayout = m_propertyColorDialogPanel->layout()) {
        colorLayout->activate();
    }
    updatePropertyColorDialogGeometry();
    m_propertyColorDialogPanel->show();
    updatePropertyColorDialogGeometry();
    m_propertyColorDialogPanel->raise();
    QTimer::singleShot(0, this, [this] {
        if (m_propertyColorDialogPanel && m_propertyColorDialogPanel->isVisible()) {
            updatePropertyColorDialogGeometry();
            m_propertyColorDialogPanel->raise();
        }
    });
}

void ShotWindow::openSelectedTextBackgroundColorPalette()
{
    if (!m_propertyColorDialogPanel || !m_propertyColorPicker || !m_annotationPropertyPanel) {
        return;
    }
    const bool wasEditingTextBackground = m_propertyColorEditingTextBackground;
    m_propertyColorEditingTextBackground = true;

    if (m_propertyColorDialogPanel->isVisible() && wasEditingTextBackground) {
        m_propertyColorDialogPanel->hide();
        return;
    }

    if (m_colorPalette) {
        m_colorPalette->hide();
    }
    QColor color = m_textBackgroundColor;
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (selectedIds.size() == 1) {
        if (const Annotation *annotation = annotationById(selectedIds.first());
            annotation && annotation->tool == Tool::Text) {
            color = annotation->backgroundColor;
        }
    }
    m_propertyColorEditHistoryCaptured = false;
    {
        const QSignalBlocker blocker(m_propertyColorPicker);
        m_propertyColorPicker->setColor(color);
    }
    updateAnnotationPropertyPanel();
    if (m_annotationPropertyPanel) {
        m_annotationPropertyPanel->show();
        m_annotationPropertyPanel->raise();
        if (QLayout *panelLayout = m_annotationPropertyPanel->layout()) {
            panelLayout->activate();
        }
        updateAnnotationPropertyPanelGeometry();
    }
    if (QLayout *colorLayout = m_propertyColorDialogPanel->layout()) {
        colorLayout->activate();
    }
    updatePropertyColorDialogGeometry();
    m_propertyColorDialogPanel->show();
    updatePropertyColorDialogGeometry();
    m_propertyColorDialogPanel->raise();
    QTimer::singleShot(0, this, [this] {
        if (m_propertyColorDialogPanel && m_propertyColorDialogPanel->isVisible()) {
            updatePropertyColorDialogGeometry();
            m_propertyColorDialogPanel->raise();
        }
    });
}

void ShotWindow::toggleSelectedTextFontPanel()
{
    if (!m_propertyFontPanel || !m_propertyFontList || !m_propertyFontButton) {
        return;
    }

    if (m_propertyFontPanel->isVisible()) {
        m_propertyFontPanel->hide();
        return;
    }

    if (m_propertyColorDialogPanel) {
        m_propertyColorDialogPanel->hide();
    }
    updateAnnotationPropertyPanel();
    if (QLayout *fontLayout = m_propertyFontPanel->layout()) {
        fontLayout->activate();
    }
    updatePropertyFontPanelGeometry();
    m_propertyFontPanel->show();
    updatePropertyFontPanelGeometry();
    m_propertyFontPanel->raise();
}

void ShotWindow::applyPropertyColor(QColor color)
{
    if (!color.isValid()) {
        return;
    }
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (m_propertyColorEditingTextBackground) {
        if (!selectedIds.isEmpty()) {
            if (!m_propertyColorEditHistoryCaptured) {
                pushHistorySnapshot();
                m_propertyColorEditHistoryCaptured = true;
            }
            for (int id : selectedIds) {
                if (Annotation *annotation = annotationById(id);
                    annotation && annotation->tool == Tool::Text) {
                    annotation->backgroundColor = color;
                }
            }
        } else if (m_tool == Tool::Text) {
            m_textBackgroundColor = color;
        }
    } else if (!selectedIds.isEmpty()) {
        if (!m_propertyColorEditHistoryCaptured) {
            pushHistorySnapshot();
            m_propertyColorEditHistoryCaptured = true;
        }
        for (int id : selectedIds) {
            if (Annotation *annotation = annotationById(id)) {
                annotation->color = color;
            }
        }
    } else {
        m_currentColor = color;
    }
    if (m_draft.has_value()) {
        m_draft->color = color;
    }
    if (m_textEditor && m_textEditor->isVisible()) {
        QColor editorColor = m_currentColor;
        QColor editorBackgroundColor = m_textBackgroundColor;
        qreal editorWidth = m_shapeWidth;
        if (m_editingTextAnnotationId.has_value()) {
            if (const Annotation *annotation = annotationById(*m_editingTextAnnotationId)) {
                editorColor = annotation->color;
                editorBackgroundColor = annotation->backgroundColor;
                editorWidth = annotation->width;
            }
        }
        m_textEditor->setStyleSheet(markshot::theme::textEditorStyleSheet(editorColor, editorBackgroundColor, qRound(20.0 + editorWidth)));
    }
    updateColorPalettePreview();
    updateAnnotationPropertyPanel();
    update();
}

void ShotWindow::clearAnnotations()
{
    commitTextEditor();
    if (m_annotations.isEmpty() && !m_draft.has_value() && m_laserStrokes.isEmpty() && !m_laserDraft.has_value()) {
        return;
    }

    pushHistorySnapshot();
    m_annotations.clear();
    m_draft.reset();
    m_laserStrokes.clear();
    m_laserDraft.reset();
    setSelectedAnnotations({});
    m_annotationDrag = SelectionDrag::None;
    m_annotationSelectionBoxActive = false;
    m_annotationHistoryCaptured = false;
    m_nextNumber = 1;
    m_nextAnnotationId = 1;
    if (m_propertyColorDialogPanel) {
        m_propertyColorDialogPanel->hide();
    }
    if (m_propertyFontPanel) {
        m_propertyFontPanel->hide();
    }
    updateAnnotationPropertyPanel();
    updateCursor();
    update();
}

void ShotWindow::setSelectedTextFontFamily(const QString &fontFamily)
{
    if (fontFamily.isEmpty()) {
        return;
    }

    if (m_selectedAnnotationId.has_value()) {
        Annotation *annotation = annotationById(*m_selectedAnnotationId);
        if (!annotation || annotation->tool != Tool::Text || annotation->fontFamily == fontFamily) {
            return;
        }
        pushHistorySnapshot();
        annotation->fontFamily = fontFamily;
    } else {
        if (m_tool != Tool::Text || m_textFontFamily == fontFamily) {
            return;
        }
        m_textFontFamily = fontFamily;
        if (m_textEditor && m_textEditor->isVisible() && !m_editingTextAnnotationId.has_value()) {
            m_textEditor->setFont(QFont(m_textFontFamily, qRound(20.0 + m_shapeWidth), QFont::DemiBold));
        }
    }
    updateAnnotationPropertyPanel();
    update();
}

void ShotWindow::toggleOpenWithPanel()
{
    commitTextEditor();
    if (!m_openWithPanel || !hasUsableSelection()) {
        return;
    }
    if (m_colorPalette) {
        m_colorPalette->hide();
    }
    if (m_extensionPanel) {
        m_extensionPanel->hide();
    }

    if (m_openWithPanel->isVisible()) {
        m_openWithPanel->hide();
        return;
    }

    updateOpenWithPanel();
    updateOpenWithPanelGeometry();
    m_openWithPanel->show();
    m_openWithPanel->raise();
}

void ShotWindow::updateOpenWithPanel()
{
    if (!m_openWithPanel) {
        return;
    }

    QLayout *layout = m_openWithPanel->layout();
    while (QLayoutItem *item = layout->takeAt(0)) {
        if (QWidget *widget = item->widget()) {
            delete widget;
        }
        delete item;
    }

    auto *title = new QLabel(MS_TR("Open with"), m_openWithPanel);
    layout->addWidget(title);

    const QVector<DesktopApp> apps = imageDesktopApps();
    if (apps.isEmpty()) {
        auto *empty = new QLabel(MS_TR("No image desktop entries found"), m_openWithPanel);
        empty->setWordWrap(true);
        layout->addWidget(empty);
        m_openWithPanel->adjustSize();
        return;
    }

    auto *list = new QListWidget(m_openWithPanel);
    list->setFocusPolicy(Qt::NoFocus);
    list->setVerticalScrollMode(QAbstractItemView::ScrollPerPixel);
    list->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    list->setIconSize(QSize(22, 22));
    for (const DesktopApp &app : apps) {
        auto *item = new QListWidgetItem(app.name, list);
        item->setToolTip(app.desktopPath);
        item->setData(Qt::UserRole, app.desktopPath);
        item->setData(Qt::UserRole + 1, app.exec);
        item->setData(Qt::UserRole + 2, app.icon);
        QIcon icon;
        if (!app.icon.isEmpty()) {
            if (app.icon.startsWith(QLatin1Char('/')) && QFile::exists(app.icon)) {
                icon = QIcon(app.icon);
            } else {
                icon = QIcon::fromTheme(app.icon);
            }
        }
        if (!icon.isNull()) {
            item->setIcon(icon);
        }
    }
    list->setFixedHeight(std::min(420, std::max(58, static_cast<int>(apps.size()) * 44)));
    connect(list, &QListWidget::itemClicked, this, [this](QListWidgetItem *item) {
        if (!item) {
            return;
        }
        DesktopApp app;
        app.name = item->text();
        app.desktopPath = item->data(Qt::UserRole).toString();
        app.exec = item->data(Qt::UserRole + 1).toString();
        app.icon = item->data(Qt::UserRole + 2).toString();
        openSelectionWithDesktop(app);
    });
    layout->addWidget(list);

    m_openWithPanel->adjustSize();
}

void ShotWindow::updateOpenWithPanelGeometry()
{
    if (!m_openWithPanel) {
        return;
    }

    m_openWithPanel->adjustSize();
    const QSize panelSize(std::min(340, std::max(280, m_openWithPanel->sizeHint().width())),
                          std::min(540, std::max(80, m_openWithPanel->sizeHint().height())));
    const QRect toolbarRect = m_fullscreenAnnotation && m_toolbar
        ? m_toolbar->geometry()
        : (m_actionToolbar ? m_actionToolbar->geometry() : QRect(width() - 64, height() / 2 - 80, 56, 160));
    int x = toolbarRect.left() - panelSize.width() - kToolbarMargin;
    int y = toolbarRect.top();
    if (x < 8) {
        x = toolbarRect.right() + kToolbarMargin;
    }
    x = std::clamp(x, 8, std::max(8, width() - panelSize.width() - 8));
    y = std::clamp(y, 8, std::max(8, height() - panelSize.height() - 8));
    m_openWithPanel->setGeometry(x, y, panelSize.width(), panelSize.height());
}

void ShotWindow::toggleExtensionPanel()
{
    commitTextEditor();
    if (!m_extensionPanel || !hasUsableSelection()) {
        return;
    }
    if (m_colorPalette) {
        m_colorPalette->hide();
    }
    if (m_openWithPanel) {
        m_openWithPanel->hide();
    }

    if (m_extensionPanel->isVisible()) {
        m_extensionPanel->hide();
        return;
    }

    updateExtensionPanel();
    updateExtensionPanelGeometry();
    m_extensionPanel->show();
    m_extensionPanel->raise();
}

void ShotWindow::updateExtensionPanel()
{
    if (!m_extensionPanel) {
        return;
    }

    QLayout *layout = m_extensionPanel->layout();
    while (QLayoutItem *item = layout->takeAt(0)) {
        if (QWidget *widget = item->widget()) {
            delete widget;
        }
        delete item;
    }

    auto *title = new QLabel(MS_TR("Extensions"), m_extensionPanel);
    layout->addWidget(title);

    QString errorMessage;
    const QVector<ExtensionCommand> commands = extensionCommands(&errorMessage);
    if (!errorMessage.isEmpty()) {
        auto *error = new QLabel(errorMessage, m_extensionPanel);
        error->setWordWrap(true);
        layout->addWidget(error);
        m_extensionPanel->adjustSize();
        return;
    }

    if (commands.isEmpty()) {
        auto *empty = new QLabel(MS_TR("No extension commands configured.\nCreate %1").arg(extensionCommandsConfigPath()),
                                 m_extensionPanel);
        empty->setWordWrap(true);
        layout->addWidget(empty);
        m_extensionPanel->adjustSize();
        return;
    }

    auto *list = new QListWidget(m_extensionPanel);
    list->setFocusPolicy(Qt::NoFocus);
    list->setVerticalScrollMode(QAbstractItemView::ScrollPerPixel);
    list->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    for (const ExtensionCommand &command : commands) {
        auto *item = new QListWidgetItem(command.name, list);
        const QString tooltip = command.description.isEmpty()
            ? command.command
            : QStringLiteral("%1\n%2").arg(command.description, command.command);
        item->setToolTip(tooltip);
        item->setData(Qt::UserRole, command.command);
        item->setData(Qt::UserRole + 1, command.workingDirectory);
        item->setData(Qt::UserRole + 2, command.description);
        item->setData(Qt::UserRole + 3, command.saveImage);
        item->setData(Qt::UserRole + 4, command.closeOnStart);
    }
    list->setFixedHeight(std::min(420, std::max(58, static_cast<int>(commands.size()) * 44)));
    connect(list, &QListWidget::itemClicked, this, [this](QListWidgetItem *item) {
        if (!item) {
            return;
        }

        ExtensionCommand command;
        command.name = item->text();
        command.command = item->data(Qt::UserRole).toString();
        command.workingDirectory = item->data(Qt::UserRole + 1).toString();
        command.description = item->data(Qt::UserRole + 2).toString();
        command.saveImage = item->data(Qt::UserRole + 3).toBool();
        command.closeOnStart = item->data(Qt::UserRole + 4).toBool();
        runExtensionCommand(command);
    });
    layout->addWidget(list);

    m_extensionPanel->adjustSize();
}

void ShotWindow::updateExtensionPanelGeometry()
{
    if (!m_extensionPanel) {
        return;
    }

    m_extensionPanel->adjustSize();
    const QSize panelSize(std::min(380, std::max(300, m_extensionPanel->sizeHint().width())),
                          std::min(540, std::max(80, m_extensionPanel->sizeHint().height())));
    const QRect toolbarRect = m_fullscreenAnnotation && m_toolbar
        ? m_toolbar->geometry()
        : (m_actionToolbar ? m_actionToolbar->geometry() : QRect(width() - 64, height() / 2 - 80, 56, 160));
    int x = toolbarRect.left() - panelSize.width() - kToolbarMargin;
    int y = toolbarRect.top();
    if (x < 8) {
        x = toolbarRect.right() + kToolbarMargin;
    }
    x = std::clamp(x, 8, std::max(8, width() - panelSize.width() - 8));
    y = std::clamp(y, 8, std::max(8, height() - panelSize.height() - 8));
    m_extensionPanel->setGeometry(x, y, panelSize.width(), panelSize.height());
}

void ShotWindow::toggleColorPalette(QPoint position)
{
    commitTextEditor();
    if (m_openWithPanel) {
        m_openWithPanel->hide();
    }
    if (m_extensionPanel) {
        m_extensionPanel->hide();
    }
    if (!m_colorPalette) {
        return;
    }

    m_colorPaletteAnchor = position;
    if (m_colorPalette->isVisible()) {
        m_colorPalette->hide();
    } else {
        updateColorPaletteGeometry(position);
        m_colorPalette->show();
        m_colorPalette->raise();
    }
    update();
}

void ShotWindow::updateColorPaletteGeometry(QPoint anchor)
{
    if (!m_colorPalette) {
        return;
    }

    const QSize paletteSize(178, 178);
    int x = anchor.x() - paletteSize.width() / 2;
    int y = anchor.y() - paletteSize.height() / 2;
    x = std::clamp(x, 8, std::max(8, width() - paletteSize.width() - 8));
    y = std::clamp(y, 8, std::max(8, height() - paletteSize.height() - 8));
    m_colorPalette->setGeometry(x, y, paletteSize.width(), paletteSize.height());

    const QPoint center(paletteSize.width() / 2, paletteSize.height() / 2);
    const qreal radius = 68.0;
    const auto buttons = m_colorPalette->findChildren<QPushButton *>(QString(), Qt::FindDirectChildrenOnly);
    for (int i = 0; i < buttons.size(); ++i) {
        const qreal angle = -M_PI / 2.0 + (2.0 * M_PI * i / std::max<qsizetype>(1, buttons.size()));
        const QPoint pos(qRound(center.x() + std::cos(angle) * radius - 15.0),
                         qRound(center.y() + std::sin(angle) * radius - 15.0));
        buttons.at(i)->setGeometry(QRect(pos, QSize(30, 30)));
    }
    updateColorPalettePreview();
}

void ShotWindow::updateColorPalettePreview()
{
    if (!m_colorPalettePreview) {
        return;
    }

    const int size = std::clamp(qRound(currentToolPreviewSize()), 8, 34);
    const QPoint center(89, 89);
    m_colorPalettePreview->setGeometry(center.x() - size / 2, center.y() - size / 2, size, size);
    m_colorPalettePreview->setStyleSheet(QStringLiteral(
        "QWidget#colorPalettePreview {"
        " background: %1;"
        " border: 0;"
        " border-radius: 3px;"
        "}").arg(m_currentColor.name()));
}

void ShotWindow::updateTextEditorGeometry()
{
    if (!m_textEditor || !m_textEditor->isVisible()) {
        return;
    }
    if (m_editingTextAnnotationId.has_value()) {
        if (const Annotation *annotation = annotationById(*m_editingTextAnnotationId)) {
            QRect editorRect = textContentRect(*annotation, true).toAlignedRect().adjusted(0, 0, 1, 1);
            editorRect.moveLeft(std::clamp(editorRect.left(), 8, std::max(8, width() - editorRect.width() - 8)));
            editorRect.moveTop(std::clamp(editorRect.top(), 8, std::max(8, height() - editorRect.height() - 8)));
            m_textEditor->setGeometry(editorRect);
        }
        return;
    }

    const QPointF topLeft = imageToWidget(m_textEditorImagePoint);
    const QRectF selection = imageRectToWidget(normalizedSelection());
    constexpr int kMinTextEditorWidth = 96;
    constexpr int kMinTextEditorHeight = 38;
    const int availableRight = std::max(kMinTextEditorWidth, qRound(selection.right() - topLeft.x() - 12));
    const int availableBottom = std::max(kMinTextEditorHeight, qRound(selection.bottom() - topLeft.y() - 12));
    const int editorWidth = std::clamp(220, kMinTextEditorWidth, availableRight);
    const int editorHeight = std::clamp(m_textEditor->fontMetrics().height() + 18, kMinTextEditorHeight, availableBottom);
    QRect editorRect(qRound(topLeft.x()), qRound(topLeft.y()), editorWidth, editorHeight);
    editorRect.moveLeft(std::clamp(editorRect.left(), 8, std::max(8, width() - editorRect.width() - 8)));
    editorRect.moveTop(std::clamp(editorRect.top(), 8, std::max(8, height() - editorRect.height() - 8)));
    m_textEditor->setGeometry(editorRect);
}

void ShotWindow::redoAnnotation()
{
    if (m_redoStack.isEmpty()) {
        return;
    }

    const HistorySnapshot current = currentHistorySnapshot();
    const HistorySnapshot next = m_redoStack.takeLast();
    m_undoStack.append(current);
    restoreHistorySnapshot(next);
}

void ShotWindow::updateToolbarState()
{
    if (!m_toolbar) {
        return;
    }

    const QString active = currentToolName();
    const QString scopeAction = markshot::ui::actionName(Action::ToggleCaptureScope);
    const QString layoutAction = markshot::ui::actionName(Action::ToggleToolbarLayout);
    const auto buttons = m_toolbar->findChildren<QPushButton *>();
    for (QPushButton *button : buttons) {
        const QString action = button->property("action").toString();
        const bool isActiveTool = action == active
            || (action == scopeAction && m_fullscreenAnnotation)
            || (action == layoutAction && m_toolbarVerticalLayout);
        button->setProperty("active", isActiveTool);
        button->style()->unpolish(button);
        button->style()->polish(button);
        button->update();
    }
}

void ShotWindow::drawAnnotation(QPainter &painter, const Annotation &annotation, bool widgetCoordinates) const
{
    auto mapPoint = [this, widgetCoordinates](QPointF point) {
        return widgetCoordinates ? imageToWidget(point) : point;
    };

    auto mapRect = [this, widgetCoordinates](QRectF rect) {
        return widgetCoordinates ? imageRectToWidget(rect) : rect;
    };

    const qreal scale = annotationSizeScale(widgetCoordinates);
    const qreal penWidth = std::max<qreal>(1.5, annotation.width * scale);

    painter.save();
    QPen pen(annotation.color, penWidth, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin);
    painter.setPen(pen);
    painter.setBrush(Qt::NoBrush);

    switch (annotation.tool) {
    case Tool::Move:
    case Tool::Select:
    case Tool::Laser:
        break;
    case Tool::Pen: {
        if (annotation.points.size() < 2) {
            break;
        }
        QPainterPath path(mapPoint(annotation.points.first()));
        for (int i = 1; i < annotation.points.size(); ++i) {
            path.lineTo(mapPoint(annotation.points.at(i)));
        }
        painter.drawPath(path);
        break;
    }
    case Tool::Highlighter: {
        if (annotation.points.size() < 2) {
            break;
        }
        QColor color = annotation.color;
        color.setAlpha(qRound(annotation.color.alphaF() * 120.0));
        painter.save();
        painter.setCompositionMode(QPainter::CompositionMode_SourceOver);
        painter.setPen(QPen(color, std::max<qreal>(6.0, penWidth), Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
        QPainterPath path(mapPoint(annotation.points.first()));
        for (int i = 1; i < annotation.points.size(); ++i) {
            path.lineTo(mapPoint(annotation.points.at(i)));
        }
        painter.drawPath(path);
        painter.restore();
        break;
    }
    case Tool::Line:
        if (annotation.points.size() >= 2) {
            painter.drawLine(mapPoint(annotation.points.first()), mapPoint(annotation.points.last()));
        }
        break;
    case Tool::Rectangle: {
        painter.setBrush(annotation.filled ? QBrush(annotation.color) : QBrush(Qt::NoBrush));
        const QRectF rect = mapRect(annotation.rect);
        const qreal radius = annotation.cornerRadius * scale;
        if (radius > 0.0) {
            painter.drawRoundedRect(rect, radius, radius);
        } else {
            painter.drawRect(rect);
        }
        break;
    }
    case Tool::Ellipse:
        painter.setBrush(annotation.filled ? QBrush(annotation.color) : QBrush(Qt::NoBrush));
        painter.drawEllipse(mapRect(annotation.rect));
        break;
    case Tool::Arrow:
        if (annotation.points.size() >= 2) {
            drawArrow(painter, mapPoint(annotation.points.first()), mapPoint(annotation.points.last()), penWidth);
        }
        break;
    case Tool::Text: {
        QFont font(annotation.fontFamily.isEmpty() ? QStringLiteral("Sans Serif") : annotation.fontFamily,
                   qRound((19.0 + annotation.width) * scale),
                   QFont::DemiBold);
        QRectF backgroundRect = textContentRect(annotation, widgetCoordinates);
        QRectF textRect = backgroundRect.adjusted(kTextBackgroundPaddingX * scale,
                                                  kTextBackgroundPaddingY * scale,
                                                  -kTextBackgroundPaddingX * scale,
                                                  -kTextBackgroundPaddingY * scale);
        QTextOption option;
        option.setWrapMode(QTextOption::WrapAtWordBoundaryOrAnywhere);
        option.setAlignment(Qt::AlignLeft | Qt::AlignTop);
        painter.save();
        painter.setFont(font);
        if (annotation.backgroundColor.alpha() > 0) {
            painter.setPen(Qt::NoPen);
            painter.setBrush(annotation.backgroundColor);
            painter.drawRoundedRect(backgroundRect, 4.0 * scale, 4.0 * scale);
        }
        painter.setPen(annotation.color);
        painter.setBrush(Qt::NoBrush);
        painter.drawText(textRect, annotation.text, option);
        painter.restore();
        break;
    }
    case Tool::Number:
        if (!annotation.points.isEmpty()) {
            drawNumber(painter, annotation.points.first(), annotation.number, annotation.color, annotation.width, widgetCoordinates);
        }
        break;
    case Tool::Mosaic:
        painter.save();
        painter.setOpacity(annotation.color.alphaF());
        drawMosaic(painter, annotation.rect, annotation.width, widgetCoordinates);
        painter.restore();
        break;
    }
    painter.restore();
}

void ShotWindow::drawArrow(QPainter &painter, QPointF start, QPointF end, qreal width) const
{
    const QLineF line(start, end);
    const qreal L = line.length();
    if (L < 1.0) {
        return;
    }

    const QColor color = painter.pen().color();

    // 1. Calculate normalized direction and normal vectors
    const QPointF direction = QPointF(line.dx() / L, line.dy() / L);
    const QPointF normal(-direction.y(), direction.x());

    // 2. Compute physical body half-width (perfectly aligned with pen brush width)
    const qreal bodyHalfWidth = width * 0.5;

    // 3. Compute adaptive arrow head length based on L (golden stretch ratio)
    qreal headLength = L * 0.18;
    headLength = std::clamp(headLength, width * 5.0, width * 9.0);
    if (headLength > L * 0.62) {
        headLength = L * 0.62;
    }
    headLength = std::clamp(headLength, 12.0, 60.0);
    if (headLength > L * 0.62) {
        headLength = L * 0.62;
    }

    // 4. Compute head half-width (sleeker, sharper aerodynamic 28% stretch ratio for acute angle nose)
    qreal headHalfWidth = headLength * 0.28;
    const qreal minHeadHalfWidth = bodyHalfWidth * 1.5;
    if (headHalfWidth < minHeadHalfWidth) {
        headHalfWidth = minHeadHalfWidth;
    }

    // 5. Locate headBase position
    const QPointF headBase = end - direction * headLength;

    // 6. Construct the elegant 6-vertex classic pointy-tailed gradient triangle polygon path
    QPainterPath arrow;
    arrow.moveTo(start);                              // 1. Pointy start (sharp tail converges to 0 width)
    arrow.lineTo(headBase + normal * bodyHalfWidth);  // 2. Left side body (gradient shaft)
    arrow.lineTo(headBase + normal * headHalfWidth);  // 3. Left wing base (vertical fold-out)
    arrow.lineTo(end);                                // 4. Arrow tip (aerodynamic nose)
    arrow.lineTo(headBase - normal * headHalfWidth);  // 5. Right wing base (aerodynamic nose)
    arrow.lineTo(headBase - normal * bodyHalfWidth);  // 6. Right side body (vertical fold-in)
    arrow.closeSubpath();                             // 7. Close back to pointy start

    // 7. Render the gorgeous hard-line polygon with anti-aliasing
    painter.save();
    painter.setPen(Qt::NoPen);
    painter.setBrush(color);
    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.drawPath(arrow);
    painter.restore();
}

void ShotWindow::drawWheelPreview(QPainter &painter)
{
    if (!m_showWheelPreview || !m_wheelPreviewTimer.isValid() || m_wheelPreviewTimer.elapsed() > 900) {
        m_showWheelPreview = false;
        updateCursor();
        return;
    }

    if (wheelZoomsImage()) {
        const QString zoomText = QStringLiteral("%1%").arg(qRound(m_imageZoom * 100.0));
        painter.save();
        painter.setRenderHint(QPainter::Antialiasing, true);
        painter.setFont(QFont(QStringLiteral("Sans Serif"), 12, QFont::DemiBold));
        const QFontMetrics metrics(painter.font());
        const QRectF textBounds = metrics.boundingRect(zoomText);
        QRectF bubble(m_wheelPreviewPosition.x() + 14.0,
                      m_wheelPreviewPosition.y() + 14.0,
                      textBounds.width() + 24.0,
                      textBounds.height() + 14.0);
        bubble.moveLeft(std::min<qreal>(bubble.left(), width() - bubble.width() - 8.0));
        bubble.moveTop(std::min<qreal>(bubble.top(), height() - bubble.height() - 8.0));
        bubble.moveLeft(std::max<qreal>(8.0, bubble.left()));
        bubble.moveTop(std::max<qreal>(8.0, bubble.top()));

        painter.setPen(Qt::NoPen);
        painter.setBrush(QColor(8, 13, 19, 230));
        painter.drawRoundedRect(bubble, 10.0, 10.0);
        painter.setPen(QColor(204, 251, 241, 245));
        painter.drawText(bubble, Qt::AlignCenter, zoomText);
        painter.restore();
        return;
    }

    const qreal size = std::clamp(currentToolPreviewSize(), 2.0, 96.0);
    QRectF preview(m_wheelPreviewPosition.x() - size / 2.0,
                   m_wheelPreviewPosition.y() - size / 2.0,
                   size,
                   size);

    painter.save();
    painter.setRenderHint(QPainter::Antialiasing, false);
    painter.setPen(Qt::NoPen);
    painter.setBrush(m_currentColor);
    painter.drawRect(preview);
    painter.restore();
}

void ShotWindow::drawLaserStroke(QPainter &painter, const LaserStroke &stroke, bool widgetCoordinates, qreal opacity) const
{
    if (stroke.points.size() < 2 || opacity <= 0.0) {
        return;
    }

    auto mapPoint = [this, widgetCoordinates](QPointF point) {
        return widgetCoordinates ? imageToWidget(point) : point;
    };
    const qreal scale = annotationSizeScale(widgetCoordinates);
    const qreal width = std::max<qreal>(3.0, stroke.width * scale);

    QPainterPath path(mapPoint(stroke.points.first()));
    for (int i = 1; i < stroke.points.size(); ++i) {
        path.lineTo(mapPoint(stroke.points.at(i)));
    }

    const qreal configuredOpacity = stroke.color.alphaF();
    QColor glow = stroke.color;
    glow.setAlpha(qRound(80 * opacity * configuredOpacity));
    QColor core = stroke.color;
    core.setAlpha(qRound(230 * opacity * configuredOpacity));
    QColor hot(255, 255, 255, qRound(170 * opacity * configuredOpacity));

    painter.save();
    painter.setCompositionMode(QPainter::CompositionMode_SourceOver);
    painter.setPen(QPen(glow, width * 2.6, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
    painter.drawPath(path);
    painter.setPen(QPen(core, width, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
    painter.drawPath(path);
    painter.setPen(QPen(hot, std::max<qreal>(1.4, width * 0.22), Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
    painter.drawPath(path);
    painter.restore();
}

void ShotWindow::beginLaserStroke(QPointF imagePoint)
{
    m_dragging = true;
    m_dragStart = imagePoint;
    LaserStroke stroke;
    stroke.points.append(clampImagePoint(imagePoint));
    stroke.color = m_currentColor;
    stroke.width = m_laserWidth;
    stroke.expiresAt = m_laserClock.elapsed() + kLaserLifetimeMs;
    m_laserDraft = stroke;
    update();
}

void ShotWindow::updateLaserStroke(QPointF imagePoint)
{
    if (!m_laserDraft.has_value()) {
        return;
    }
    m_laserDraft->points.append(clampImagePoint(imagePoint));
    update();
}

void ShotWindow::commitLaserStroke()
{
    if (!m_laserDraft.has_value()) {
        return;
    }
    if (m_laserDraft->points.size() >= 2) {
        m_laserDraft->expiresAt = m_laserClock.elapsed() + kLaserLifetimeMs;
        m_laserStrokes.append(*m_laserDraft);
        if (m_laserTimer && !m_laserTimer->isActive()) {
            m_laserTimer->start();
        }
    }
    m_laserDraft.reset();
    update();
}

void ShotWindow::cleanupLaserStrokes()
{
    const qint64 now = m_laserClock.elapsed();
    for (int i = m_laserStrokes.size() - 1; i >= 0; --i) {
        if (m_laserStrokes.at(i).expiresAt <= now) {
            m_laserStrokes.removeAt(i);
        }
    }
    if (m_laserStrokes.isEmpty() && m_laserTimer) {
        m_laserTimer->stop();
    }
    update();
}

void ShotWindow::drawNumber(QPainter &painter,
                            QPointF imagePoint,
                            int number,
                            QColor color,
                            qreal width,
                            bool widgetCoordinates) const
{
    const QPointF center = widgetCoordinates ? imageToWidget(imagePoint) : imagePoint;
    const qreal scale = annotationSizeScale(widgetCoordinates);
    const qreal radius = std::max<qreal>(13.0, (13.0 + width * 1.35) * scale);
    const QRectF bubble(center.x() - radius, center.y() - radius, radius * 2.0, radius * 2.0);

    painter.save();
    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.setPen(QPen(QColor(255, 255, 255), std::clamp(width * 0.22 * scale, 2.0, 9.0)));
    painter.setBrush(color);
    painter.drawEllipse(bubble);

    QFont font(QStringLiteral("Sans Serif"), qRound(std::clamp(radius * 0.92, 12.0, 54.0)), QFont::Black);
    painter.setFont(font);
    painter.setPen(Qt::white);
    painter.drawText(bubble, Qt::AlignCenter, QString::number(number));
    painter.restore();
}

void ShotWindow::drawMosaic(QPainter &painter, QRectF imageRect, qreal blockSize, bool widgetCoordinates) const
{
    QRect sourceRect = imageRect.normalized().toAlignedRect().intersected(QRect(QPoint(0, 0), m_frozenFrame.size()));
    if (sourceRect.isEmpty()) {
        return;
    }

    const QImage mosaic = mosaicImage(sourceRect, qRound(blockSize));
    if (mosaic.isNull()) {
        return;
    }

    painter.save();
    painter.setRenderHint(QPainter::SmoothPixmapTransform, false);
    painter.drawImage(widgetCoordinates ? imageRectToWidget(sourceRect) : QRectF(sourceRect), mosaic);
    painter.restore();
}

void ShotWindow::beginTextAnnotation(QPointF imagePoint)
{
    m_editingTextAnnotationId.reset();
    m_textEditorImagePoint = imagePoint;
    m_draft.reset();
    m_textEditor->clear();
    m_textEditor->setStyleSheet(markshot::theme::textEditorStyleSheet(m_currentColor, m_textBackgroundColor, qRound(20.0 + m_shapeWidth)));
    m_textEditor->setFont(QFont(m_textFontFamily, qRound(20.0 + m_shapeWidth), QFont::DemiBold));
    m_textEditor->show();
    m_textEditor->raise();
    updateTextEditorGeometry();
    m_textEditor->setFocus(Qt::MouseFocusReason);
    update();
}

void ShotWindow::beginEditingSelectedTextAnnotation()
{
    if (!m_selectedAnnotationId.has_value()) {
        return;
    }
    Annotation *annotation = annotationById(*m_selectedAnnotationId);
    if (!annotation || annotation->tool != Tool::Text) {
        return;
    }

    m_editingTextAnnotationId = annotation->id;
    m_textEditorImagePoint = annotation->rect.normalized().topLeft();
    m_draft.reset();
    m_textEditor->setPlainText(annotation->text);
    m_textEditor->setStyleSheet(markshot::theme::textEditorStyleSheet(annotation->color, annotation->backgroundColor, qRound(20.0 + annotation->width)));
    m_textEditor->setFont(QFont(annotation->fontFamily.isEmpty() ? QStringLiteral("Sans Serif") : annotation->fontFamily,
                                qRound(20.0 + annotation->width),
                                QFont::DemiBold));
    if (m_annotationPropertyPanel) {
        m_annotationPropertyPanel->hide();
    }
    if (m_propertyColorDialogPanel) {
        m_propertyColorDialogPanel->hide();
    }
    if (m_propertyFontPanel) {
        m_propertyFontPanel->hide();
    }
    m_textEditor->show();
    m_textEditor->raise();
    const QRectF widgetRect = textContentRect(*annotation, true);
    m_textEditor->setGeometry(widgetRect.toAlignedRect().adjusted(0, 0, 1, 1));
    m_textEditor->setFocus(Qt::MouseFocusReason);
    update();
}

void ShotWindow::commitTextEditor()
{
    if (m_committingText || !m_textEditor || !m_textEditor->isVisible()) {
        return;
    }

    m_committingText = true;
    const QString text = m_textEditor->toPlainText().trimmed();
    const QRect editorGeometry = m_textEditor->geometry();
    m_textEditor->hide();
    m_textEditor->clear();
    setFocus(Qt::OtherFocusReason);

    if (m_editingTextAnnotationId.has_value()) {
        if (Annotation *annotation = annotationById(*m_editingTextAnnotationId)) {
            pushHistorySnapshot();
            annotation->text = text;
            annotation->rect = QRectF(widgetToImage(editorGeometry.topLeft()),
                                      widgetToImage(editorGeometry.bottomRight())).normalized();
            annotation->fontFamily = m_textEditor->font().family();
            annotation->rect = textContentRect(*annotation, false);
            if (!annotation->points.isEmpty()) {
                annotation->points[0] = annotation->rect.topLeft();
            }
        }
        m_editingTextAnnotationId.reset();
        m_committingText = false;
        updateAnnotationPropertyPanel();
        update();
        return;
    }

    if (!text.isEmpty()) {
        pushHistorySnapshot();
        Annotation annotation;
        annotation.id = m_nextAnnotationId++;
        annotation.tool = Tool::Text;
        annotation.points.append(m_textEditorImagePoint);
        annotation.rect = QRectF(widgetToImage(editorGeometry.topLeft()),
                                 widgetToImage(editorGeometry.bottomRight())).normalized();
        annotation.text = text;
        annotation.color = m_currentColor;
        annotation.backgroundColor = m_textBackgroundColor;
        annotation.width = m_shapeWidth;
        annotation.fontFamily = m_textEditor->font().family();
        annotation.rect = textContentRect(annotation, false);
        m_textFontFamily = annotation.fontFamily;
        m_annotations.append(annotation);
    }

    m_committingText = false;
    update();
}

QString ShotWindow::saveSelectionToTempFile() const
{
    if (!hasUsableSelection()) {
        return {};
    }

    const QImage output = renderedSelection();
    if (output.isNull()) {
        return {};
    }

    const QString tempDir = QStandardPaths::writableLocation(QStandardPaths::TempLocation).isEmpty()
        ? QDir::tempPath()
        : QStandardPaths::writableLocation(QStandardPaths::TempLocation);
    const QString filename = QStringLiteral("mark-shot-open-%1.png")
                                 .arg(QDateTime::currentDateTimeUtc().toString(QStringLiteral("yyyyMMdd-hhmmss-zzz")));
    const QString path = QDir(tempDir).filePath(filename);
    return output.save(path, "PNG") ? path : QString();
}

void ShotWindow::openSelectionWithDesktop(const DesktopApp &app)
{
    commitTextEditor();
    if (m_openWithPanel) {
        m_openWithPanel->hide();
    }
    if (m_extensionPanel) {
        m_extensionPanel->hide();
    }

    const QString imagePath = saveSelectionToTempFile();
    if (imagePath.isEmpty()) {
        return;
    }

    QStringList command = expandDesktopExec(app, imagePath);
    if (command.isEmpty()) {
        return;
    }

    const QString program = command.takeFirst();
    if (QProcess::startDetached(program, command)) {
        close();
    }
}

void ShotWindow::runExtensionCommand(const ExtensionCommand &command)
{
    commitTextEditor();
    if (m_extensionPanel) {
        m_extensionPanel->hide();
    }
    if (m_openWithPanel) {
        m_openWithPanel->hide();
    }

    QString commandLine = command.command;
    if (commandLine.contains(QStringLiteral("{slurp}"))) {
        const QString geometry = slurpSelectionGeometry();
        if (geometry.isEmpty()) {
            return;
        }
        replaceExtensionSlurpPlaceholder(&commandLine, geometry);
    }

    bool replacedImagePlaceholder = false;
    QString imagePath;
    if (command.saveImage) {
        imagePath = saveSelectionToTempFile();
        if (imagePath.isEmpty()) {
            return;
        }
        replacedImagePlaceholder = replaceExtensionImagePlaceholders(&commandLine, imagePath);
        if (!replacedImagePlaceholder) {
            commandLine += QLatin1Char(' ');
            commandLine += shellQuote(imagePath);
        }
    }

    if (commandLine.trimmed().isEmpty()) {
        return;
    }

    QString shell = QProcessEnvironment::systemEnvironment().value(QStringLiteral("SHELL"), QStringLiteral("/bin/sh"));
    if (shell.isEmpty()) {
        shell = QStringLiteral("/bin/sh");
    }
    const QString workingDirectory = command.workingDirectory.isEmpty()
        ? QString()
        : expandUserPath(command.workingDirectory);

    if (command.closeOnStart) {
        hide();
        QApplication::processEvents();
    }

    const bool started = QProcess::startDetached(shell, {QStringLiteral("-c"), commandLine}, workingDirectory);
    if (started && command.closeOnStart) {
        close();
        return;
    }

    if (!started && command.closeOnStart) {
        show();
        raise();
        activateWindow();
        updateToolbarGeometry();
        updateActionToolbarGeometry();
        updateExtensionPanelGeometry();
    }
}

void ShotWindow::pinSelection()
{
    commitTextEditor();
    if (!hasUsableSelection()) {
        return;
    }

    const QImage output = renderedSelection();
    if (output.isNull()) {
        return;
    }

    auto *window = new PinnedImageWindow(output);
    window->show();
    window->raise();
    window->activateWindow();
    close();
}

void ShotWindow::ocrCopySelection()
{
    commitTextEditor();
    if (!hasUsableSelection()) {
        return;
    }

    const QString tempPath = saveSelectionToTempFile();
    if (tempPath.isEmpty()) {
        return;
    }

    const QString ocrProgram = helperProgramPath(QStringLiteral("mark-shot-ocr"));
    if (ocrProgram.isEmpty()) {
        QFile::remove(tempPath);
        showToast(MS_TR("OCR helper not found"));
        return;
    }

    const PinnedWindowConfig config = pinnedWindowConfig();

    QApplication::setOverrideCursor(Qt::WaitCursor);

    QProcess process;
    process.setProgram(ocrProgram);
    process.setArguments({QStringLiteral("--format"),
                          QStringLiteral("json"),
                          QStringLiteral("--backend"),
                          config.ocrBackend,
                          tempPath});
    process.start();
    if (!process.waitForFinished(config.ocrTimeoutMs)) {
        process.kill();
        process.waitForFinished(1000);
        QFile::remove(tempPath);
        QApplication::restoreOverrideCursor();
        showToast(MS_TR("OCR timed out"));
        return;
    }

    QFile::remove(tempPath);
    QApplication::restoreOverrideCursor();

    if (process.exitStatus() != QProcess::NormalExit || process.exitCode() != 0) {
        showToast(MS_TR("OCR failed"));
        return;
    }

    const QByteArray output = process.readAllStandardOutput();
    QJsonParseError parseError;
    const QJsonDocument document = QJsonDocument::fromJson(output, &parseError);
    if (parseError.error != QJsonParseError::NoError) {
        return;
    }

    QJsonArray tokenArray;
    if (document.isArray()) {
        tokenArray = document.array();
    } else if (document.isObject()) {
        tokenArray = document.object().value(QStringLiteral("tokens")).toArray();
    }

    auto rectFromJson = [](const QJsonValue &value) -> std::optional<QRectF> {
        if (!value.isArray()) {
            return std::nullopt;
        }
        const QJsonArray array = value.toArray();
        if (array.size() == 4 && array.at(0).isDouble()) {
            return QRectF(array.at(0).toDouble(), array.at(1).toDouble(),
                          array.at(2).toDouble(), array.at(3).toDouble());
        }
        if (array.size() < 2 || !array.at(0).isArray()) {
            return std::nullopt;
        }
        QRectF bounds;
        bool initialized = false;
        for (const QJsonValue &pv : array) {
            if (!pv.isArray()) continue;
            const QJsonArray pt = pv.toArray();
            if (pt.size() < 2) continue;
            const QPointF p(pt.at(0).toDouble(), pt.at(1).toDouble());
            bounds = initialized ? bounds.united(QRectF(p, QSizeF(0, 0))) : QRectF(p, QSizeF(0, 0));
            initialized = true;
        }
        return initialized ? std::optional(bounds) : std::nullopt;
    };

    auto ocrRect = [&](const QJsonObject &obj) -> std::optional<QRectF> {
        for (const auto &key : {QStringLiteral("box"), QStringLiteral("bbox"), QStringLiteral("points")}) {
            if (obj.contains(key)) return rectFromJson(obj.value(key));
        }
        if (obj.contains(QStringLiteral("x")) && obj.contains(QStringLiteral("y"))) {
            return QRectF(obj.value(QStringLiteral("x")).toDouble(), obj.value(QStringLiteral("y")).toDouble(),
                          obj.value(QStringLiteral("width")).toDouble(), obj.value(QStringLiteral("height")).toDouble());
        }
        if (obj.contains(QStringLiteral("left")) && obj.contains(QStringLiteral("top"))) {
            return QRectF(obj.value(QStringLiteral("left")).toDouble(), obj.value(QStringLiteral("top")).toDouble(),
                          obj.value(QStringLiteral("width")).toDouble(), obj.value(QStringLiteral("height")).toDouble());
        }
        return std::nullopt;
    };

    auto isNoSpacePunctuation = [](QChar ch) {
        switch (ch.unicode()) {
        case '.': case ',': case ';': case ':': case '!': case '?':
        case ')': case ']': case '}':
        case 0x3001: case 0x3002: case 0x300B: case 0x3011:
        case 0xFF01: case 0xFF09: case 0xFF0C: case 0xFF1A: case 0xFF1B: case 0xFF1F:
            return true;
        default:
            return false;
        }
    };

    struct LineToken {
        int line;
        int index;
        QString text;
        QRectF rect;
    };
    QVector<LineToken> tokens;
    int fallbackIndex = 0;
    for (const QJsonValue &value : tokenArray) {
        if (!value.isObject()) continue;
        const QJsonObject object = value.toObject();
        const QString text = object.value(QStringLiteral("text")).toString().trimmed();
        if (text.isEmpty()) continue;
        const auto rect = ocrRect(object);
        if (!rect) continue;
        LineToken token;
        token.text = text;
        token.line = object.value(QStringLiteral("line")).toInt(0);
        token.index = object.value(QStringLiteral("index")).toInt(fallbackIndex++);
        token.rect = rect->normalized();
        tokens.append(token);
    }

    if (tokens.isEmpty()) {
        showToast(MS_TR("No text recognized"));
        return;
    }    std::stable_sort(tokens.begin(), tokens.end(), [](const LineToken &a, const LineToken &b) {
        return a.line != b.line ? a.line < b.line : a.index < b.index;
    });

    QString result;
    int currentLine = -1;
    QRectF prevRect;
    QString prevText;
    for (const LineToken &token : tokens) {
        if (currentLine != token.line) {
            if (!result.isEmpty()) {
                result += QLatin1Char('\n');
            }
            currentLine = token.line;
        } else if (!prevText.isEmpty() && !token.text.isEmpty()
                   && !isNoSpacePunctuation(token.text.front())) {
            const qreal gap = token.rect.left() - prevRect.right();
            const qreal threshold = std::max<qreal>(3.0, std::min(prevRect.height(), token.rect.height()) * 0.28);
            if (gap > threshold) {
                result += QLatin1Char(' ');
            }
        }
        result += token.text;
        prevText = token.text;
        prevRect = token.rect;
    }

    QApplication::clipboard()->setText(result);
    showToast(MS_TR("OCR text copied"));
}

void ShotWindow::showToast(const QString &text, int durationMs)
{
    auto *label = new QLabel(text, this);
    label->setAlignment(Qt::AlignCenter);
    label->setFont(QFont(QStringLiteral("Sans Serif"), 12, QFont::DemiBold));
    label->setStyleSheet(QStringLiteral(
        "background: rgba(8, 13, 19, 220);"
        "color: rgba(204, 251, 241, 238);"
        "border-radius: 14px;"
        "padding: 8px 22px;"));
    label->adjustSize();
    label->move((width() - label->width()) / 2, height() - label->height() - 80);
    label->show();
    QTimer::singleShot(durationMs, label, &QObject::deleteLater);
}

QImage ShotWindow::renderedSelection() const
{
    const QRect sourceBounds(QPoint(0, 0), m_frozenFrame.size());
    const QRect selectionRect = normalizedSelection().toAlignedRect().intersected(sourceBounds);
    if (selectionRect.isEmpty()) {
        return {};
    }

    QImage output = m_frozenFrame.copy(selectionRect).convertToFormat(QImage::Format_ARGB32_Premultiplied);
    QPainter painter(&output);
    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.translate(-selectionRect.topLeft());
    for (const Annotation &annotation : m_annotations) {
        drawAnnotation(painter, annotation, false);
    }
    painter.end();
    return output;
}

QString ShotWindow::defaultSavePath() const
{
    const QString filename = QStringLiteral("mark-shot-%1.png").arg(QDateTime::currentDateTime().toString(QStringLiteral("yyyyMMdd-hhmmss")));
    return QDir(markShotPicturesDir()).filePath(filename);
}

void ShotWindow::saveSelection()
{
    commitTextEditor();

    if (!hasUsableSelection()) {
        return;
    }

    const QImage output = renderedSelection();
    if (output.isNull()) {
        return;
    }

    if (m_openWithPanel) {
        m_openWithPanel->hide();
    }
    if (m_extensionPanel) {
        m_extensionPanel->hide();
    }
    if (m_colorPalette) {
        m_colorPalette->hide();
    }
    if (m_annotationPropertyPanel) {
        m_annotationPropertyPanel->hide();
    }
    if (m_propertyColorDialogPanel) {
        m_propertyColorDialogPanel->hide();
    }
    if (m_propertyFontPanel) {
        m_propertyFontPanel->hide();
    }

    hide();

    auto *dialog = new QFileDialog(nullptr, MS_TR("Save Screenshot"));
    dialog->setAttribute(Qt::WA_DeleteOnClose);
    dialog->setAcceptMode(QFileDialog::AcceptSave);
    dialog->setFileMode(QFileDialog::AnyFile);
    dialog->setNameFilter(MS_TR("PNG Images (*.png)"));
    dialog->setDefaultSuffix(QStringLiteral("png"));
    dialog->setOption(QFileDialog::DontUseNativeDialog, true);
    dialog->selectFile(defaultSavePath());

    connect(dialog, &QFileDialog::accepted, this, [this, dialog, output] {
        const QStringList files = dialog->selectedFiles();
        if (!files.isEmpty() && output.save(files.first(), "PNG")) {
            close();
            return;
        }
        show();
        raise();
        activateWindow();
        updateToolbarGeometry();
        updateActionToolbarGeometry();
    });
    connect(dialog, &QFileDialog::rejected, this, [this] {
        show();
        raise();
        activateWindow();
        updateToolbarGeometry();
        updateActionToolbarGeometry();
    });
    dialog->open();
}

void ShotWindow::copySelection()
{
    commitTextEditor();

    if (!hasUsableSelection()) {
        return;
    }

    QImage output = renderedSelection();
    if (output.isNull()) {
        return;
    }

    QApplication::clipboard()->setImage(output);

    QByteArray png;
    QBuffer buffer(&png);
    buffer.open(QIODevice::WriteOnly);
    output.save(&buffer, "PNG");

    const bool isWayland = QProcessEnvironment::systemEnvironment()
        .value(QStringLiteral("XDG_SESSION_TYPE")).toLower() == QStringLiteral("wayland");

    if (isWayland) {
        QProcess wlCopy;
        wlCopy.setProgram(QStringLiteral("wl-copy"));
        wlCopy.setArguments({QStringLiteral("--type"), QStringLiteral("image/png")});
        wlCopy.start(QIODevice::WriteOnly);
        if (wlCopy.waitForStarted(1000)) {
            wlCopy.write(png);
            wlCopy.closeWriteChannel();
            wlCopy.waitForFinished(2500);
        }
    } else {
        QProcess xclip;
        xclip.setProgram(QStringLiteral("xclip"));
        xclip.setArguments({QStringLiteral("-selection"), QStringLiteral("clipboard"),
                            QStringLiteral("-t"), QStringLiteral("image/png")});
        xclip.start(QIODevice::WriteOnly);
        if (xclip.waitForStarted(1000)) {
            xclip.write(png);
            xclip.closeWriteChannel();
            xclip.waitForFinished(2500);
        }
    }

    close();
}
