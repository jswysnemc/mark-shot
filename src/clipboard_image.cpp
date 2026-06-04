#include "clipboard_image.h"

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

enum class ClipboardBackend {
    None,
    Wayland,
    X11,
};

struct ClipboardOwnerCommand {
    QString executable;
    QString shellCommand;
};

enum class ClipboardPayload {
    ImagePng,
    Text,
};

constexpr qsizetype kInlineImageClipboardLimitBytes = 4 * 1024 * 1024;

ClipboardBackend clipboardBackend(const QProcessEnvironment &environment)
{
    const QString sessionType = environment.value(QStringLiteral("XDG_SESSION_TYPE")).toLower();
    if (sessionType == QStringLiteral("wayland")) {
        return ClipboardBackend::Wayland;
    }
    if (sessionType == QStringLiteral("x11") || !environment.value(QStringLiteral("DISPLAY")).isEmpty()) {
        return ClipboardBackend::X11;
    }
    return ClipboardBackend::None;
}

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

QByteArray encodePng(QImage image)
{
    QByteArray png;
    QBuffer buffer(&png);
    if (!buffer.open(QIODevice::WriteOnly) || !image.save(&buffer, "PNG")) {
        return {};
    }
    return png;
}

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

bool copyToPersistentClipboardOwner(const QByteArray &payload, const QString &suffix, const ClipboardOwnerCommand &owner)
{
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
}

bool copyImageDataToClipboard(const QImage &image, const QByteArray &png)
{
    bool copied = false;
    if (QClipboard *clipboard = QApplication::clipboard()) {
        clipboard->setImage(image);
        copied = true;
    }

    const std::optional<ClipboardOwnerCommand> owner = clipboardOwnerCommand(ClipboardPayload::ImagePng);
    if (owner.has_value()) {
        copied = copyToPersistentClipboardOwner(png, QStringLiteral(".png"), *owner) || copied;
    }
    return copied;
}

bool copyUrlToClipboard(const QUrl &url)
{
    const QString urlText = url.toString(QUrl::FullyEncoded);
    bool copied = false;

    if (QClipboard *clipboard = QApplication::clipboard()) {
        auto *mimeData = new QMimeData;
        mimeData->setText(urlText);
        mimeData->setUrls({url});
        clipboard->setMimeData(mimeData);
        copied = true;
    }

    const std::optional<ClipboardOwnerCommand> owner = clipboardOwnerCommand(ClipboardPayload::Text);
    if (owner.has_value()) {
        copied = copyToPersistentClipboardOwner(urlText.toUtf8(), QStringLiteral(".txt"), *owner) || copied;
    }
    return copied;
}

} // namespace

bool copyTextToClipboard(const QString &text)
{
    if (text.isEmpty()) {
        return false;
    }

    bool copied = false;
    if (QClipboard *clipboard = QApplication::clipboard()) {
        clipboard->setText(text);
        copied = true;
    }

    const std::optional<ClipboardOwnerCommand> owner = clipboardOwnerCommand(ClipboardPayload::Text);
    if (owner.has_value()) {
        copied = copyToPersistentClipboardOwner(text.toUtf8(), QStringLiteral(".txt"), *owner) || copied;
    }
    return copied;
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

    if (png.size() > kInlineImageClipboardLimitBytes) {
        const std::optional<QUrl> cachedUrl = savePngToClipboardCache(png);
        if (cachedUrl.has_value()) {
            return copyUrlToClipboard(*cachedUrl);
        }
    }

    return copyImageDataToClipboard(image, png);
}

} // namespace markshot
