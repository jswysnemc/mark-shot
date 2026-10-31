#pragma once

#include <QJsonArray>
#include <QJsonObject>
#include <QJsonValue>
#include <QRectF>
#include <QSize>
#include <QString>
#include <QVector>

#include <optional>

namespace markshot::ocr {

struct Token {
    QString text;
    QRectF imageRect;
    int line = 0;
    int index = 0;
    qreal confidence = 0.0;
};

struct ParsedOutput {
    bool validJson = false;
    QVector<Token> tokens;
};

QJsonArray rectToJson(QRectF rect);
std::optional<QRectF> rectFromJsonValue(const QJsonValue &value);
std::optional<QRectF> tokenRect(const QJsonObject &object);
ParsedOutput parseOutput(const QByteArray &output, QSize imageSize = {});
QVector<Token> tokensFromJsonOutput(const QByteArray &output, QSize imageSize = {});
bool isNoLeadingSpacePunctuation(QChar ch);
bool isCjkCodePoint(uint codePoint);
bool shouldInsertSpace(const QString &previousText,
                       const QString &currentText,
                       QRectF previousRect,
                       QRectF currentRect);
QString tokenRangeText(const QVector<Token> &tokens, int first, int last);
QString tokensText(const QVector<Token> &tokens);

}  // namespace markshot::ocr
