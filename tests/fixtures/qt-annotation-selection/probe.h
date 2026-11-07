// 【标注选框对照】【固定场景】提供生产选框函数的矩形输入与窗口坐标，其他工具分支不作为本探针验证范围
#pragma once
#include <QImage>
#include <QJsonArray>
#include <QJsonObject>
#include <QLineF>
#include <QPainter>
#include <QPolygonF>
#include <QVector>
#include <algorithm>
#include <cmath>
#include <optional>

namespace markshot::shot::types { enum class RulerMode { Length, Angle }; }

class ShotWindow {
public:
    enum class Tool { Move, Select, Laser, Ruler, Pen, Line, Highlighter, Arrow, Rectangle, Ellipse, Marker, Mosaic, Text, Number, Magnifier };
    enum class MagnifierShape { Rectangle, Circle };
    struct Annotation {
        int id = 1;
        Tool tool = Tool::Rectangle;
        QRectF rect;
        qreal rotationDegrees = 0.0;
        QVector<QPointF> points;
        MagnifierShape magnifierShape = MagnifierShape::Rectangle;
        markshot::shot::types::RulerMode rulerMode = markshot::shot::types::RulerMode::Length;
    };
    Annotation annotation;
    bool m_imageNavigationEnabled = false;
    bool m_imageSelected = false;
    Tool m_tool = Tool::Select;
    QImage m_frozenFrame = QImage(600, 360, QImage::Format_ARGB32_Premultiplied);
    QRectF m_frozenImageRect = QRectF(0, 0, 600, 360);
    QSize windowSize = QSize(600, 360);
    qreal sceneScale = 1.0;

    /** 【标注选框对照】【窗口尺寸】返回固定测试窗口宽度，不参与边界计算 */
    int width() const { return windowSize.width(); }
    /** 【标注选框对照】【窗口尺寸】返回固定测试窗口高度，不参与边界计算 */
    int height() const { return windowSize.height(); }
    /** 【标注选框对照】【固定选择】返回当前生产绘制函数的一项选择，无参数 */
    QVector<int> selectedAnnotationIds() const { return {annotation.id}; }
    /** 【标注选框对照】【对象读取】按 id 返回本次输入标注，其他身份返回空值 */
    const Annotation *annotationById(int id) const { return id == annotation.id ? &annotation : nullptr; }
    /** 【标注选框对照】【输入边界】返回已提供的正文边界，本探针不代替生产文字排版或其他工具边界 */
    QRectF annotationUnrotatedBounds(const Annotation &value) const { return value.rect; }
    /** 【标注选框对照】【固定边界】返回本次单项输入边界，无参数 */
    QRectF selectedAnnotationsBounds() const { return annotation.rect; }
    /** 【标注选框对照】【固定边界】按 value 返回输入边界，仅用于未执行的其他工具分支 */
    QRectF annotationBounds(const Annotation &value) const { return value.rect; }
    /** 【标注选框对照】【图像映射】按 point 返回窗口逻辑坐标，使用场景比例 */
    QPointF imageToWidget(QPointF point) const { return point * sceneScale; }
    /** 【标注选框对照】【边界映射】按 rect 返回窗口逻辑矩形，使用场景比例 */
    QRectF imageRectToWidget(QRectF rect) const { return QRectF(imageToWidget(rect.topLeft()), imageToWidget(rect.bottomRight())); }
    /** 【标注选框对照】【普通矩形】矩形没有线条定点，此入口不增加绘制，无返回值 */
    void drawLineAnchorHandles(QPainter &, const Annotation &, QPointF, qreal, bool) const {}
    /** 【标注选框对照】【矩形范围】矩形场景不使用放大镜采样框，返回空矩形 */
    QRectF magnifierSourceRect(const Annotation &) const { return {}; }
    /** 【标注选框对照】【基础绘制】不加载二进制皮肤插件，返回基础删除按钮继续绘制 */
    bool paintAnnotationDeleteButtonIfEnabled(QPainter &, QRectF) const { return false; }

    qreal annotationSizeScale(bool) const;
    bool annotationSupportsRotation(const Annotation &) const;
    QPointF annotationRotationCenter(const Annotation &, bool) const;
    QPointF rotatedPoint(QPointF, QPointF, qreal) const;
    QPointF annotationRotationHandlePoint(const Annotation &, bool) const;
    QPointF selectionRotationHandlePoint(QRectF, bool) const;
    QVector<QPointF> selectionHandlePoints(QRectF) const;
    QRectF selectedAnnotationDeleteButtonRect() const;
    void drawSelectedAnnotationFrame(QPainter &) const;
};

class ShotWindowRulerPainter {
public:
    /** 【标注选框对照】【分支声明】仅使未执行的尺子分支具备合法接口，window 不参与本探针绘制 */
    explicit ShotWindowRulerPainter(const ShotWindow &) {}
    /** 【标注选框对照】【分支声明】矩形场景不执行尺子绘制，无返回值 */
    void drawSelectedFrame(QPainter &, const ShotWindow::Annotation &) const {}
    /** 【标注选框对照】【分支声明】矩形场景不执行尺子旋转间距读取，返回零 */
    qreal rotationHandleGap(const ShotWindow::Annotation &) const { return 0; }
    /** 【标注选框对照】【分支声明】矩形场景不执行角度尺边界读取，返回空矩形 */
    QRectF angleFrame(const ShotWindow::Annotation &) const { return {}; }
};
