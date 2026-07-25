#include "ocr_result_window_geometry.h"

#include <algorithm>

namespace markshot::shot {

OcrResultWindowPlacement ocrResultWindowPlacement(const QRect &targetAvailableGeometry,
                                                  const QRect &fallbackAvailableGeometry)
{
    const QRect availableGeometry = targetAvailableGeometry.isValid()
            && !targetAvailableGeometry.isEmpty()
        ? targetAvailableGeometry
        : fallbackAvailableGeometry;
    QSize size(420, 520);
    if (availableGeometry.isValid() && !availableGeometry.isEmpty()) {
        size.setWidth(std::min(size.width(),
                               std::max(320, qRound(availableGeometry.width() * 0.9))));
        size.setHeight(std::min(size.height(),
                                std::max(260, qRound(availableGeometry.height() * 0.9))));
    }

    QPoint topLeft;
    if (availableGeometry.isValid() && !availableGeometry.isEmpty()) {
        QRect windowGeometry(QPoint(0, 0), size);
        windowGeometry.moveCenter(availableGeometry.center());
        topLeft = windowGeometry.topLeft();
    }
    return {size, topLeft};
}

}  // namespace markshot::shot
