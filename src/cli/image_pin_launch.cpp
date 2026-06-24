#include "cli/image_pin_launch.h"

#include "pinned_window/pinned_image_window.h"
#include "ui/i18n.h"

#include <QFileInfo>
#include <QImage>
#include <QImageReader>
#include <QWidget>

namespace markshot::cli {
namespace {

/// @brief 写入错误信息并返回空窗口指针。
/// @param error 错误信息输出指针。
/// @param message 错误信息。
/// @return 空窗口指针。
QWidget *fail(QString *error, const QString &message)
{
    if (error) {
        *error = message;
    }
    return nullptr;
}

}  // namespace

QWidget *launchPinnedImageFromPath(const QString &imagePath, QString *error)
{
    if (error) {
        error->clear();
    }

    const QString trimmedPath = imagePath.trimmed();
    if (trimmedPath.isEmpty()) {
        return fail(error, MS_TR("Image file does not exist: %1").arg(imagePath));
    }

    const QFileInfo imageFile(trimmedPath);
    if (!imageFile.exists() || !imageFile.isFile()) {
        return fail(error, MS_TR("Image file does not exist: %1").arg(trimmedPath));
    }

    QImageReader reader(imageFile.absoluteFilePath());
    reader.setAutoTransform(true);
    QImage image = reader.read();
    if (image.isNull()) {
        return fail(error,
                    MS_TR("Failed to load image: %1\n%2").arg(imageFile.absoluteFilePath(), reader.errorString()));
    }

    auto *window = new markshot::shot::PinnedImageWindow(std::move(image));
    window->show();
    window->raise();
    window->activateWindow();
    return window;
}

}  // namespace markshot::cli
