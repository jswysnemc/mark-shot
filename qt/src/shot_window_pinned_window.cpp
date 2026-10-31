#include "pinned_window/pinned_image_window.h"

#include "translation_layout/translation_seed.h"

namespace markshot::shot {

QWidget *createPinnedImageWindow(QImage image, std::optional<QPoint> initialTopLeft,
                                 const markshot::translation_layout::TranslationSeed *translationSeed)
{
    auto *window = new PinnedImageWindow(std::move(image), initialTopLeft);
    if (translationSeed) {
        window->applyTranslationSeed(*translationSeed);
    }
    return window;
}

}  // namespace markshot::shot
