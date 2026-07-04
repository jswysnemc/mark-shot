#pragma once

#include <QByteArray>
#include <QSize>

namespace markshot::platform::windows {

struct WindowsWgcFrame {
    QByteArray bgra;
    QSize size;
    int stride = 0;
    qint64 timestampMs = 0;
};

}  // namespace markshot::platform::windows
