#include "shot_window_module.h"

#include "ocr_result_window.h"

#include <utility>

namespace markshot::shot {

QWidget *createOcrResultWindow(QString text)
{
    return new OcrResultWindow(std::move(text));
}

}  // namespace markshot::shot
