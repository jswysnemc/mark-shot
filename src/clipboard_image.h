#pragma once

#include <QImage>
#include <QString>

namespace markshot {

bool copyImageToClipboard(const QImage &image);
bool copyTextToClipboard(const QString &text);

} // namespace markshot
