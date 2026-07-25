#include "shot_window_module.h"

#include "ocr_result_window.h"

#include <utility>

namespace markshot::shot {

QWidget *createOcrResultWindow(QString text, QScreen *targetScreen)
{
    return new OcrResultWindow(std::move(text), targetScreen);
}

}  // namespace markshot::shot
