// 【普通标注旋转对照】【输入类型】只提供生产角度及缩放函数需要的固定画布，不代替原始算法
#pragma once
#include <QImage>
#include <QJsonArray>
#include <QJsonObject>
#include <QLineF>
#include <QRectF>
#include <algorithm>
#include <cmath>
#include <optional>

constexpr qreal kMinSelectionSize = 8.0;
constexpr qreal kMinimumRotationRadius = 0.1;
constexpr qreal kRotationSnapStep = 45.0;

class ShotWindow {
public:
    enum class SelectionDrag {
        None, Move, Left, Right, Top, Bottom, TopLeft, TopRight, BottomLeft, BottomRight,
        MagnifierSource, MagnifierLens, MagnifierSourceLeft, MagnifierSourceRight,
        MagnifierSourceTop, MagnifierSourceBottom, MagnifierSourceTopLeft,
        MagnifierSourceTopRight, MagnifierSourceBottomLeft, MagnifierSourceBottomRight,
        Rotate, LineControl, LineStart, LineEnd, NumberTip, NumberBubble,
        AngleVertex, AngleFirstArm, AngleSecondArm
    };
    QImage m_frozenFrame = QImage(640, 480, QImage::Format_ARGB32_Premultiplied);
    QPointF clampImagePoint(QPointF) const;
    QRectF resizedBounds(QRectF, SelectionDrag, QPointF, bool) const;
};

qreal normalizedRotationDegrees(qreal);
qreal snappedRotationDegrees(qreal);
std::optional<qreal> rotationDragDegrees(QPointF, QPointF, QPointF);
