#include "rapid_rec_model.h"

#include <QFile>
#include <QTextStream>

#include <algorithm>
#include <cmath>

namespace markshot::ocr_rapid {
namespace {

constexpr int kRecInputHeight = 48;
constexpr int kRecMinInputWidth = 16;
constexpr int kRecMaxInputWidth = 1280;

/**
 * 把文本行图像归一化为 NCHW 浮点输入。
 * @param image 已缩放到输入尺寸的 ARGB32 图像。
 * @param input 输出浮点数据。
 * @return 无返回值。
 */
void fillRecInput(const QImage &image, QVector<float> *input)
{
    // 与 rapidocr 一致：BGR 通道顺序，(x/255-0.5)/0.5 归一化
    const int width = image.width();
    const int height = image.height();
    const qsizetype plane = static_cast<qsizetype>(width) * height;
    input->resize(plane * 3);
    float *blue = input->data();
    float *green = blue + plane;
    float *red = green + plane;
    for (int y = 0; y < height; ++y) {
        const QRgb *row = reinterpret_cast<const QRgb *>(image.constScanLine(y));
        for (int x = 0; x < width; ++x) {
            const qsizetype offset = static_cast<qsizetype>(y) * width + x;
            blue[offset] = (qBlue(row[x]) / 255.0f - 0.5f) / 0.5f;
            green[offset] = (qGreen(row[x]) / 255.0f - 0.5f) / 0.5f;
            red[offset] = (qRed(row[x]) / 255.0f - 0.5f) / 0.5f;
        }
    }
}

}  // namespace

bool RapidRecModel::load(const QString &modelPath, const QString &dictionaryPath, QString *error)
{
    if (!m_session.load(modelPath, error)) {
        return false;
    }

    // 1. 加载字符字典，一行一个字符
    QFile dictionaryFile(dictionaryPath);
    if (!dictionaryFile.open(QIODevice::ReadOnly | QIODevice::Text)) {
        if (error) {
            *error = QStringLiteral("cannot open recognition dictionary: %1").arg(dictionaryPath);
        }
        return false;
    }
    m_characters.clear();
    QTextStream stream(&dictionaryFile);
    while (!stream.atEnd()) {
        QString line = stream.readLine();
        // 字典中的空格行等原样保留，只去掉行尾换行残留
        while (line.endsWith(QLatin1Char('\r'))) {
            line.chop(1);
        }
        m_characters.append(line);
    }
    if (m_characters.isEmpty()) {
        if (error) {
            *error = QStringLiteral("recognition dictionary is empty: %1").arg(dictionaryPath);
        }
        return false;
    }
    return true;
}

bool RapidRecModel::recognize(const QImage &lineImage, RapidRecResult *result, QString *error)
{
    result->text.clear();
    result->confidence = 0.0;
    result->characters.clear();
    result->stepCount = 0;
    if (lineImage.isNull() || lineImage.width() <= 0 || lineImage.height() <= 0) {
        if (error) {
            *error = QStringLiteral("recognition input image is empty");
        }
        return false;
    }

    // 1. 保持纵横比缩放到高度 48
    const qreal aspect = static_cast<qreal>(lineImage.width()) / lineImage.height();
    const int inputWidth = std::clamp(static_cast<int>(std::round(kRecInputHeight * aspect)),
                                      kRecMinInputWidth,
                                      kRecMaxInputWidth);
    const QImage scaled = lineImage
                              .scaled(inputWidth,
                                      kRecInputHeight,
                                      Qt::IgnoreAspectRatio,
                                      Qt::SmoothTransformation)
                              .convertToFormat(QImage::Format_ARGB32);
    QVector<float> input;
    fillRecInput(scaled, &input);

    // 2. 推理得到 [1, T, C] 概率序列
    QVector<float> output;
    TensorShape outputShape;
    if (!m_session.run(input,
                       {{1, 3, kRecInputHeight, inputWidth}},
                       &output,
                       &outputShape,
                       error)) {
        return false;
    }
    if (outputShape.dims.size() != 3) {
        if (error) {
            *error = QStringLiteral("unexpected recognition output shape");
        }
        return false;
    }
    const int steps = static_cast<int>(outputShape.dims.at(1));
    const int classes = static_cast<int>(outputShape.dims.at(2));
    if (steps <= 0 || classes <= 1
        || static_cast<qsizetype>(steps) * classes > output.size()) {
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
        const float *probabilities = output.constData() + static_cast<qsizetype>(step) * classes;
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
                currentIndex <= m_characters.size()
                ? m_characters.at(currentIndex - 1)
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
