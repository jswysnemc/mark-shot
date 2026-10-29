#include "clipboard_image.h"

#include "clipboard_image_config.h"
#include "clipboard_image_mime_data.h"
#include "clipboard_publish_policy.h"

#include <QApplication>
#include <QBuffer>
#include <QClipboard>
#include <QDir>
#include <QFile>
#include <QMimeData>
#include <QProcess>
#include <QProcessEnvironment>
#include <QStandardPaths>
#include <QTemporaryFile>
#include <QUrl>

#include <optional>

namespace markshot {
namespace {

/// @brief 当前进程是否会在截图窗口关闭后继续运行（托盘模式）。
bool g_clipboardOwnershipLongLived = false;

enum class ClipboardBackend {
    None,
    Wayland,
    X11,
};

/// @brief Command and executable information for persistently owning clipboard data.
struct ClipboardOwnerCommand {
    /// @brief The path or filename of the executable that owns the clipboard data.
    QString executable;
    /// @brief The command line template to execute to launch the owner process.
    QString shellCommand;
};

enum class ClipboardPayload {
    ImagePng,
    Text,
};

/// @brief Detects the active clipboard backend based on the environment variables.
/// @param environment The current process environment.
/// @return The detected ClipboardBackend enum value.
ClipboardBackend clipboardBackend(const QProcessEnvironment &environment)
{
#if defined(Q_OS_WIN)
    Q_UNUSED(environment);
    return ClipboardBackend::None;
#else
    const QString sessionType = environment.value(QStringLiteral("XDG_SESSION_TYPE")).toLower();
    if (sessionType == QStringLiteral("wayland")) {
        return ClipboardBackend::Wayland;
    }
    if (sessionType == QStringLiteral("x11") || !environment.value(QStringLiteral("DISPLAY")).isEmpty()) {
        return ClipboardBackend::X11;
    }
    return ClipboardBackend::None;
#endif
}

/// @brief Finds the executable name/path for the corresponding clipboard backend.
/// @param backend The active clipboard backend.
/// @return The resolved executable path, or an empty string if not found.
QString clipboardExecutable(ClipboardBackend backend)
{
    switch (backend) {
    case ClipboardBackend::Wayland:
        return QStandardPaths::findExecutable(QStringLiteral("wl-copy"));
    case ClipboardBackend::X11:
        return QStandardPaths::findExecutable(QStringLiteral("xclip"));
    case ClipboardBackend::None:
        break;
    }
    return {};
}

/// @brief Gets the shell command template used to invoke the persistent clipboard owner.
/// @param backend The active clipboard backend.
/// @param payload The type of clipboard payload.
/// @return The shell command string.
QString clipboardShellCommand(ClipboardBackend backend, ClipboardPayload payload)
{
    switch (backend) {
    case ClipboardBackend::Wayland:
        switch (payload) {
        case ClipboardPayload::ImagePng:
            return QStringLiteral("\"$2\" --foreground --type image/png < \"$1\"; rm -f \"$1\"");
        case ClipboardPayload::Text:
            return QStringLiteral("\"$2\" --foreground --type text/plain < \"$1\"; rm -f \"$1\"");
        }
        break;
    case ClipboardBackend::X11:
        switch (payload) {
        case ClipboardPayload::ImagePng:
            return QStringLiteral("\"$2\" -selection clipboard -t image/png < \"$1\"; rm -f \"$1\"");
        case ClipboardPayload::Text:
            return QStringLiteral("\"$2\" -selection clipboard -t text/plain < \"$1\"; rm -f \"$1\"");
        }
        break;
    case ClipboardBackend::None:
        break;
    }
    return {};
}

/// @brief Resolves the command and executable needed to persistently own the clipboard content.
/// @param payload The type of clipboard payload (e.g. ImagePng, Text).
/// @return A ClipboardOwnerCommand struct, or std::nullopt if the current environment is unsupported.
std::optional<ClipboardOwnerCommand> clipboardOwnerCommand(ClipboardPayload payload)
{
    const ClipboardBackend backend = clipboardBackend(QProcessEnvironment::systemEnvironment());
    const QString executable = clipboardExecutable(backend);
    const QString command = clipboardShellCommand(backend, payload);
    if (executable.isEmpty() || command.isEmpty()) {
        return std::nullopt;
    }
    return ClipboardOwnerCommand{executable, command};
}

/// @brief Encodes a QImage object into a PNG byte array.
/// @param image The source image to encode.
/// @return The encoded PNG bytes, or an empty byte array on failure.
QByteArray encodePng(QImage image)
{
    QByteArray png;
    QBuffer buffer(&png);
    if (!buffer.open(QIODevice::WriteOnly) || !image.save(&buffer, "PNG")) {
        return {};
    }
    return png;
}

/// @brief Locates or creates a writeable cache directory for storing clipboard assets.
/// @return The directory path as a QString, or an empty string on failure.
QString clipboardCacheDir()
{
    QString baseDir = QStandardPaths::writableLocation(QStandardPaths::CacheLocation);
    if (baseDir.isEmpty()) {
        const QString genericCacheDir = QStandardPaths::writableLocation(QStandardPaths::GenericCacheLocation);
        if (!genericCacheDir.isEmpty()) {
            baseDir = QDir(genericCacheDir).filePath(QStringLiteral("mark-shot"));
        }
    }
    if (baseDir.isEmpty()) {
        baseDir = QDir(QDir::tempPath()).filePath(QStringLiteral("mark-shot"));
    }

    const QString cacheDir = QDir(baseDir).filePath(QStringLiteral("clipboard"));
    QDir dir(cacheDir);
    if (dir.exists() || dir.mkpath(QStringLiteral("."))) {
        return cacheDir;
    }
    return {};
}

/// @brief Saves a PNG image byte array to a temporary file in the clipboard cache directory.
/// @param png The PNG image byte data.
/// @return A QUrl pointing to the cached PNG file, or std::nullopt on failure.
std::optional<QUrl> savePngToClipboardCache(const QByteArray &png)
{
    const QString cacheDir = clipboardCacheDir();
    if (cacheDir.isEmpty()) {
        return std::nullopt;
    }

    QTemporaryFile cacheFile(QDir(cacheDir).filePath(QStringLiteral("mark-shot-clipboard-XXXXXX.png")));
    cacheFile.setAutoRemove(false);
    if (!cacheFile.open()) {
        return std::nullopt;
    }

    if (cacheFile.write(png) != png.size()) {
        const QString cachePath = cacheFile.fileName();
        cacheFile.close();
        QFile::remove(cachePath);
        return std::nullopt;
    }

    const QString cachePath = cacheFile.fileName();
    cacheFile.close();
    return QUrl::fromLocalFile(cachePath);
}

/// @brief Starts a detached process to own the clipboard data persistently after this process exits.
/// @param payload The raw byte content of the clipboard data.
/// @param suffix The file extension suffix for the temporary file (e.g., ".png" or ".txt").
/// @param owner Struct containing the executable path and shell command to invoke.
/// @return True if the persistent owner process was successfully started.
bool copyToPersistentClipboardOwner(const QByteArray &payload, const QString &suffix, const ClipboardOwnerCommand &owner)
{
#if defined(Q_OS_WIN)
    Q_UNUSED(payload);
    Q_UNUSED(suffix);
    Q_UNUSED(owner);
    return false;
#else
    if (payload.isEmpty()) {
        return false;
    }

    QTemporaryFile tempFile(QDir(QDir::tempPath()).filePath(QStringLiteral("mark-shot-clipboard-XXXXXX%1").arg(suffix)));
    tempFile.setAutoRemove(false);
    if (!tempFile.open()) {
        return false;
    }

    if (tempFile.write(payload) != payload.size()) {
        const QString tempPath = tempFile.fileName();
        tempFile.close();
        QFile::remove(tempPath);
        return false;
    }

    const QString tempPath = tempFile.fileName();
    tempFile.close();

    const bool started = QProcess::startDetached(QStringLiteral("sh"),
                                                 {QStringLiteral("-c"),
                                                  owner.shellCommand,
                                                  QStringLiteral("mark-shot-clipboard"),
                                                  tempPath,
                                                  owner.executable});
    if (!started) {
        QFile::remove(tempPath);
    }
    return started;
#endif
}

#if !defined(Q_OS_WIN) && !defined(Q_OS_MACOS)
/**
 * 【剪贴板】【退出转交】进程退出前把仍由本进程持有的图片交给 wl-copy / xclip。
 *
 * 托盘进程退出后 Qt 持有的选择随之消失，转交后剪贴板里仍保留 PNG。
 * @return 无返回值。
 */
void handOffClipboardImageBeforeQuit()
{
    QClipboard *clipboard = QApplication::clipboard();
    if (!clipboard || !clipboard->ownsClipboard()) {
        return;
    }
    const auto *data = qobject_cast<const ClipboardImageMimeData *>(clipboard->mimeData());
    const std::optional<ClipboardOwnerCommand> owner =
        clipboardOwnerCommand(ClipboardPayload::ImagePng);
    if (!data || !owner.has_value()) {
        return;
    }
    copyToPersistentClipboardOwner(data->png(), QStringLiteral(".png"), *owner);
}

/**
 * 【剪贴板】【退出转交】首次由本进程持有图片时登记退出前的转交。
 * @return 无返回值。
 */
void ensureClipboardQuitHandoff()
{
    static bool connected = false;
    if (connected || !QCoreApplication::instance()) {
        return;
    }
    connected = true;
    QObject::connect(QCoreApplication::instance(), &QCoreApplication::aboutToQuit,
                     &handOffClipboardImageBeforeQuit);
}
#endif

/**
 * 【剪贴板】【多格式图片】通过 QClipboard 发布图片。
 * @param clipboard Qt 剪贴板。
 * @param image 原始图片，Windows 与 macOS 由平台插件转换为系统位图。
 * @param png 图片的 PNG 编码字节，Linux 只保存这一份数据。
 * @return 无返回值。
 */
void publishImageWithQtClipboard(QClipboard *clipboard, const QImage &image, const QByteArray &png)
{
#if defined(Q_OS_WIN) || defined(Q_OS_MACOS)
    Q_UNUSED(png);
    clipboard->setImage(image);
#else
    // 1. 同时声明 image/png 与 image/bmp，BMP 在被请求时再转换
    Q_UNUSED(image);
    clipboard->setMimeData(new ClipboardImageMimeData(png));
    ensureClipboardQuitHandoff();
#endif
}

/// @brief Copies an image and its PNG byte array representation to the system clipboard and persistent owner.
/// @param image The QImage to be set on the system clipboard.
/// @param png The encoded PNG byte data to be saved to the persistent clipboard owner.
/// @return True if the image was successfully copied.
bool copyImageDataToClipboard(const QImage &image, const QByteArray &png)
{
    QClipboard *clipboard = QApplication::clipboard();
    const std::optional<ClipboardOwnerCommand> owner =
        clipboardOwnerCommand(ClipboardPayload::ImagePng);
    switch (chooseImageClipboardPublishPath(owner.has_value(), clipboard != nullptr,
                                            g_clipboardOwnershipLongLived)) {
    case ClipboardPublishPath::PersistentOwner:
        return copyToPersistentClipboardOwner(png, QStringLiteral(".png"), *owner);
    case ClipboardPublishPath::QtClipboard:
        publishImageWithQtClipboard(clipboard, image, png);
        return true;
    case ClipboardPublishPath::None:
        break;
    }
    return false;
}

/// @brief Copies a URL to both the system clipboard and a persistent clipboard owner process.
/// @param url The URL to be copied.
/// @return True if the copy operation was successful.
bool copyUrlToClipboard(const QUrl &url)
{
    const QString urlText = url.toString(QUrl::FullyEncoded);
    QClipboard *clipboard = QApplication::clipboard();
    const std::optional<ClipboardOwnerCommand> owner =
        clipboardOwnerCommand(ClipboardPayload::Text);
    switch (chooseClipboardPublishPath(owner.has_value(), clipboard != nullptr)) {
    case ClipboardPublishPath::PersistentOwner:
        return copyToPersistentClipboardOwner(urlText.toUtf8(), QStringLiteral(".txt"), *owner);
    case ClipboardPublishPath::QtClipboard: {
        auto *mimeData = new QMimeData;
        mimeData->setText(urlText);
        mimeData->setUrls({url});
        clipboard->setMimeData(mimeData);
        return true;
    }
    case ClipboardPublishPath::None:
        break;
    }
    return false;
}

/**
 * 将图片缓存为文件 URL 后复制到剪贴板。
 * @param png 图片 PNG 字节数据。
 * @return 复制成功时返回 true，否则返回 false。
 */
bool copyImageUrlToClipboard(const QByteArray &png)
{
    const std::optional<QUrl> cachedUrl = savePngToClipboardCache(png);
    return cachedUrl.has_value() && copyUrlToClipboard(*cachedUrl);
}

} // namespace

void setClipboardOwnershipLongLived(bool longLived)
{
    g_clipboardOwnershipLongLived = longLived;
}

bool copyTextToClipboard(const QString &text)
{
    if (text.isEmpty()) {
        return false;
    }

    QClipboard *clipboard = QApplication::clipboard();
    const std::optional<ClipboardOwnerCommand> owner =
        clipboardOwnerCommand(ClipboardPayload::Text);
    switch (chooseClipboardPublishPath(owner.has_value(), clipboard != nullptr)) {
    case ClipboardPublishPath::PersistentOwner:
        return copyToPersistentClipboardOwner(text.toUtf8(), QStringLiteral(".txt"), *owner);
    case ClipboardPublishPath::QtClipboard:
        clipboard->setText(text);
        return true;
    case ClipboardPublishPath::None:
        break;
    }
    return false;
}

bool copyImageToClipboard(const QImage &image)
{
    if (image.isNull()) {
        return false;
    }

    const QByteArray png = encodePng(image);
    if (png.isEmpty()) {
        return false;
    }

    const ClipboardImageConfig config = configuredClipboardImageConfig();
    switch (config.mode) {
    case ClipboardImageMode::ImagePng:
        return copyImageDataToClipboard(image, png);
    case ClipboardImageMode::Url:
        return copyImageUrlToClipboard(png);
    case ClipboardImageMode::Threshold:
        // 1. 图片超过阈值时优先复制文件 URL
        if (png.size() > clipboardImageThresholdBytes(config) && copyImageUrlToClipboard(png)) {
            return true;
        }
        // 2. 未超过阈值或 URL 写入失败时复制 image/png
        return copyImageDataToClipboard(image, png);
    }

    return copyImageDataToClipboard(image, png);
}

} // namespace markshot
