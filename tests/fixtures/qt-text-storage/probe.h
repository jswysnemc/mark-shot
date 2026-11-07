// 【文字存储对照】【输入类型】提供生产文字测量、存储矩形变换和收框所需字段，不替代被验证的算法
#pragma once
#include <QFont>
#include <QImage>
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
#include <optional>

namespace markshot::theme { QFont textFont(int, QFont::Weight, QString); }
namespace markshot::shot {
constexpr qreal kTextBackgroundPaddingX = 6.0;
constexpr qreal kTextBackgroundPaddingY = 4.0;
constexpr qreal kMinTextWrapWidth = 16.0;
constexpr qreal kTextWrapTolerance = 0.5;
constexpr qreal kUnwrappedTextBoxWidth = 100000.0;
constexpr qreal kMinSelectionSize = 8.0;
constexpr qreal kMinNumberWidth = 1.0;
constexpr qreal kMaxNumberWidth = 72.0;
namespace types {
enum class Tool { Move, Select, Laser, Rectangle, Ellipse, Marker, Mosaic, Magnifier, Ruler, Text, Pen, Highlighter, Line, Arrow, Number };
struct Annotation {
    Tool tool = Tool::Text;
    QRectF rect;
    QVector<QPointF> points;
    QString text = QStringLiteral("Rotation");
    QString fontFamily = QStringLiteral("Noto Sans");
    QFont::Weight fontWeight = QFont::DemiBold;
    qreal width = 1.0;
    bool textItalic = false;
};
}
QRectF textAnnotationBaseRect(const types::Annotation &);
bool textAnnotationWraps(const types::Annotation &);
QSizeF textAnnotationBoxSize(const types::Annotation &, qreal);
void fitTextAnnotationRect(types::Annotation &, std::optional<qreal> = std::nullopt);
}

class ShotWindow {
public:
    using Annotation = markshot::shot::types::Annotation;
    using Tool = markshot::shot::types::Tool;
    enum class SelectionDrag {
        None, Move, Left, Right, Top, Bottom, TopLeft, TopRight, BottomLeft, BottomRight,
        MagnifierSource, MagnifierLens, MagnifierSourceLeft, MagnifierSourceRight,
        MagnifierSourceTop, MagnifierSourceBottom, MagnifierSourceTopLeft,
        MagnifierSourceTopRight, MagnifierSourceBottomLeft, MagnifierSourceBottomRight,
        Rotate, LineControl, LineStart, LineEnd, NumberTip, NumberBubble,
        AngleVertex, AngleFirstArm, AngleSecondArm
    };
    SelectionDrag m_annotationDrag = SelectionDrag::Right;
    QImage m_frozenFrame = QImage(640, 480, QImage::Format_ARGB32_Premultiplied);
    QPointF clampImagePoint(QPointF) const;
    void moveAnnotation(Annotation &, QPointF) const;
    void transformAnnotation(Annotation &, QRectF, QRectF) const;
    QRectF resizedBounds(QRectF, SelectionDrag, QPointF, bool) const;
};
