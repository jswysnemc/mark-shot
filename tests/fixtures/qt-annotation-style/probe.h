#pragma once

#include "shot_window_types.h"
#include "annotation/arrow_rendering.h"
#include "line_skeleton_path.h"
#include <QApplication>
#include <QImage>
#include <QPainter>
#include <QTextOption>
#include <algorithm>
#include <cmath>
#include <cstdlib>

namespace markshot::shot {
QPainterPath smoothedStrokePath(const QVector<QPointF> &points);
}

class ShotWindow {
public:
    using Annotation = markshot::shot::types::Annotation;
    using Tool = markshot::shot::types::Tool;
    using HighlighterStyle = markshot::shot::types::HighlighterStyle;
    QImage m_frozenFrame;
    QRectF m_frozenImageRect;

    qreal annotationSizeScale(bool widgetCoordinates) const;
    QPointF imageToWidget(QPointF point) const;
    QRectF imageRectToWidget(QRectF rect) const;
    void drawAnnotation(QPainter &painter, const Annotation &annotation, bool widgetCoordinates) const;
    void drawNumber(QPainter &painter, QPointF tip, QPointF bubble, const QString &label, QColor color, qreal width, bool widgetCoordinates) const;

    /// 【样式对照】【旋转范围】按annotation返回本采集器是否支持旋转；采集场景只包含零角度
    bool annotationSupportsRotation(const Annotation &annotation) const { return !qFuzzyIsNull(annotation.rotationDegrees); }
    /// 【样式对照】【未采集轴心】参数为annotation与widgetCoordinates，无返回值；非零旋转进入此分支立即退出
    QPointF annotationRotationCenter(const Annotation &, bool) const { std::abort(); }
    /// 【样式对照】【未采集矩形】参数为painter、annotation与widgetCoordinates，无返回值；此工具不参与当前采集
    void drawRectangle(QPainter &, const Annotation &, bool) const { std::abort(); }
    /// 【样式对照】【未采集标记】参数为painter、annotation与widgetCoordinates，无返回值；此工具不参与当前采集
    void drawMarker(QPainter &, const Annotation &, bool) const { std::abort(); }
    /// 【样式对照】【未采集滤镜】参数为painter、annotation与widgetCoordinates，无返回值；此工具不参与当前采集
    void drawMosaic(QPainter &, const Annotation &, bool) const { std::abort(); }
    /// 【样式对照】【未采集放大镜】参数为painter、annotation与widgetCoordinates，无返回值；此工具不参与当前采集
    void drawMagnifier(QPainter &, const Annotation &, bool) const { std::abort(); }
    /// 【样式对照】【未采集文字】参数为annotation与widgetCoordinates，无返回值；此工具不参与当前采集
    QRectF textContentRect(const Annotation &, bool) const { std::abort(); }
    /// 【样式对照】【阿拉伯标签】按number返回阿拉伯数字字符串，style只接受原版阿拉伯样式
    QString numberLabelText(int number, markshot::shot::types::NumberStyle style) const {
        if (style != markshot::shot::types::NumberStyle::Arabic) std::abort();
        return QString::number(number);
    }
};

class ShotWindowRulerPainter {
public:
    /// 【样式对照】【未采集尺子】按window构造仅供链接的分支保护，无返回值
    explicit ShotWindowRulerPainter(const ShotWindow &) {}
    /// 【样式对照】【未采集正文】参数为painter、annotation与widgetCoordinates，无返回值；此工具不参与当前采集
    void drawBody(QPainter &, const ShotWindow::Annotation &, bool) const { std::abort(); }
};
