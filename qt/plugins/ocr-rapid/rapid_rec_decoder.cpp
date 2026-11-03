#include "rapid_rec_decoder.h"

#include <limits>

namespace markshot::ocr_rapid {

bool decodeRapidRecognition(const float *output, qsizetype count, const TensorShape &outputShape,
                            const QStringList &dictionary, RapidRecResult *result, QString *error)
{
    if (error) error->clear();
    if (!result) {
        if (error) *error = QStringLiteral("recognition result is missing");
        return false;
    }
    *result = {};
    if (outputShape.dims.size() != 3 || outputShape.dims.front() != 1) {
        if (error) {
            *error = QStringLiteral("unexpected recognition output shape");
        }
        return false;
    }
    if (!output || count < 0 || outputShape.dims.at(1) > std::numeric_limits<int>::max()
        || outputShape.dims.at(2) > std::numeric_limits<int>::max()
        || outputShape.dims.at(1) <= 0 || outputShape.dims.at(2) <= 1) {
        if (error) *error = QStringLiteral("recognition output size mismatch");
        return false;
    }
    const int steps = static_cast<int>(outputShape.dims.at(1));
    const int classes = static_cast<int>(outputShape.dims.at(2));
    if (steps <= 0 || classes <= 1
        || steps > count / classes) {
        if (error) {
            *error = QStringLiteral("recognition output size mismatch");
        }
        return false;
    }
    result->stepCount = steps;

    // 3. 逐时间步取 argmax，先保留类别与置信度
    QVector<int> bestIndexes(steps);
    QVector<float> bestScores(steps);
    for (int step = 0; step < steps; ++step) {
        const float *probabilities = output + static_cast<qsizetype>(step) * classes;
        int best = 0;
        float bestScore = probabilities[0];
        for (int c = 1; c < classes; ++c) {
            if (probabilities[c] > bestScore) {
                bestScore = probabilities[c];
                best = c;
            }
        }
        bestIndexes[step] = best;
        bestScores[step] = bestScore;
    }

    // 4. CTC 解码：按连续 run 折叠重复并跳过 blank(0)，同时保留字符跨度
    QString text;
    QVector<RapidRecCharacter> characters;
    double confidenceSum = 0.0;
    int keptSteps = 0;
    int previousIndex = 0;
    int step = 0;
    while (step < steps) {
        const int currentIndex = bestIndexes.at(step);
        const int startStep = step;
        double runConfidenceSum = 0.0;
        int runStepCount = 0;
        while (step < steps && bestIndexes.at(step) == currentIndex) {
            runConfidenceSum += bestScores.at(step);
            ++runStepCount;
            ++step;
        }

        if (currentIndex != 0 && currentIndex != previousIndex) {
            // 类别布局：0=blank，1..dictSize=字典字符，dictSize+1=空格
            const QString characterText =
                currentIndex <= dictionary.size()
                ? dictionary.at(currentIndex - 1)
                : QStringLiteral(" ");
            RapidRecCharacter character;
            character.text = characterText;
            character.startStep = startStep;
            character.endStep = step - 1;
            character.confidence = runStepCount > 0 ? runConfidenceSum / runStepCount : 0.0;
            characters.append(character);
            text += characterText;
            confidenceSum += character.confidence;
            ++keptSteps;
        }
        previousIndex = currentIndex;
    }

    result->text = text.trimmed();
    result->confidence = keptSteps > 0 ? confidenceSum / keptSteps : 0.0;
    result->characters = characters;
    return true;
}

}  // namespace markshot::ocr_rapid
