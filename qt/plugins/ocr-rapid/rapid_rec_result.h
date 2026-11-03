#pragma once

#include <QString>
#include <QVector>

namespace markshot::ocr_rapid {

struct RapidRecCharacter {
    QString text;
    int startStep = 0;
    int endStep = 0;
    qreal confidence = 0.0;
};

struct RapidRecResult {
    QString text;
    qreal confidence = 0.0;
    QVector<RapidRecCharacter> characters;
    int stepCount = 0;
};

}  // namespace markshot::ocr_rapid
