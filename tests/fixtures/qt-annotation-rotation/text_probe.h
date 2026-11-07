// 【旋转文字对照】【输入类型】只提供完整生产文字测量函数所需的字体和标注字段
#pragma once
#include <QFont>
#include <QPointF>
#include <QRectF>
#include <QSizeF>
#include <QString>
#include <QTextBlock>
#include <QTextDocument>
#include <QTextLayout>
#include <QTextOption>
#include <QVector>
#include <algorithm>
#include <cmath>

namespace markshot::theme {
QFont textFont(int, QFont::Weight, QString);
}

namespace markshot::shot {
constexpr qreal kTextBackgroundPaddingX = 6.0;
constexpr qreal kTextBackgroundPaddingY = 4.0;
constexpr qreal kMinTextWrapWidth = 16.0;
constexpr qreal kTextWrapTolerance = 0.5;
namespace types {
struct Annotation {
    QRectF rect;
    QVector<QPointF> points;
    QString text;
    QString fontFamily = QStringLiteral("Noto Sans");
    QFont::Weight fontWeight = QFont::DemiBold;
    qreal width = 1.0;
    bool textItalic = false;
};
}
QRectF textAnnotationBaseRect(const types::Annotation &);
bool textAnnotationWraps(const types::Annotation &);
QSizeF textAnnotationBoxSize(const types::Annotation &, qreal);
}
