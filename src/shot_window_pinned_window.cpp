#include "pinned_window/pinned_image_window.h"

namespace markshot::shot {

QWidget *createPinnedImageWindow(QImage image, std::optional<QPoint> initialTopLeft)
{
    return new PinnedImageWindow(std::move(image), initialTopLeft);
}

}  // namespace markshot::shot
