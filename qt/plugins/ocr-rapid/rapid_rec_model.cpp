#include "rapid_rec_model.h"

#include "rapid_rec_decoder.h"

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

void RapidRecModel::release()
{
    m_session.release();
    m_characters.clear();
    m_characters.squeeze();
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

    // 2. 【OCR】【字符解码】在原生输出仍有效时直接解码，不复制完整的 [1,T,C] 概率矩阵
    return m_session.visitOutput(input, {{1, 3, kRecInputHeight, inputWidth}},
        [&](const float *output, qsizetype count, const TensorShape &shape, QString *failure) {
            return decodeRapidRecognition(output, count, shape, m_characters, result, failure);
        }, error);
}

}  // namespace markshot::ocr_rapid
