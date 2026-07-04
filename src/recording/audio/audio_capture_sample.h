#pragma once

#include <QByteArray>

#include <cstdint>

namespace markshot::recording {

struct AudioCaptureSample {
    QByteArray pcm;
    std::int64_t sequence = 0;
};

}  // namespace markshot::recording
