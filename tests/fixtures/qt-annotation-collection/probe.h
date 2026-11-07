// 【标注多选对照】【输入类型】只提供生产函数需要的字段，群组之外的分支均断言不可执行
#pragma once
#include <QApplication>
#include <QFont>
#include <QImage>
#include <QJsonArray>
#include <QJsonObject>
#include <QLineF>
#include <QRectF>
#include <QTextDocument>
#include <QTextLayout>
#include <QTextBlock>
#include <QTextOption>
#include <QTextEdit>
#include <QVector>
#include <QtMath>
#include <algorithm>
#include <cassert>
#include <cmath>
#include <optional>
#include "constants.inc"

namespace markshot::theme {
QFont textFont(int, QFont::Weight, QString);
/** 【标注多选对照】【未执行样式】多选样本不编辑文字输入框，执行此接口即失败，color、background与size为生产样式参数 */
inline QString textEditorStyleSheet(QColor color, QColor background, int size) { Q_UNUSED(color); Q_UNUSED(background); Q_UNUSED(size); assert(false); return {}; }
}
namespace markshot::shot {
constexpr qreal kMinTextWrapWidth = 16.0, kTextWrapTolerance = 0.5, kUnwrappedTextBoxWidth = 100000.0;
namespace types {
enum class Tool { Move, Select, Laser, Rectangle, Ellipse, Marker, Mosaic, Magnifier, Ruler, Text, Pen, Highlighter, Line, Arrow, Number };
enum class RulerMode { Length, Angle };
enum class MagnifierShape { Rectangle, Circle };
enum class HighlighterStyle { Freehand, StraightLine };
struct Annotation {
    int id = 1;
    Tool tool = Tool::Rectangle;
    QRectF rect;
    QVector<QPointF> points;
    qreal width = 3.0, rotationDegrees = 0.0, magnifierScale = 2.75;
    QString text = QStringLiteral("Rotation"), fontFamily = QStringLiteral("Noto Sans");
    QFont::Weight fontWeight = QFont::DemiBold;
    bool textItalic = false;
    QColor color = QColor(30, 40, 50), backgroundColor = Qt::transparent;
    RulerMode rulerMode = RulerMode::Length;
    MagnifierShape magnifierShape = MagnifierShape::Rectangle;
    HighlighterStyle highlighterStyle = HighlighterStyle::Freehand;
};
}
using Annotation = types::Annotation;
using Tool = types::Tool;
using HighlighterStyle = types::HighlighterStyle;
using MagnifierShape = types::MagnifierShape;
using RulerMode = types::RulerMode;
qreal normalizedRotationDegrees(qreal);
qreal clampedMagnifierScale(qreal);
QRectF textAnnotationBaseRect(const Annotation &);
bool textAnnotationWraps(const Annotation &);
QSizeF textAnnotationBoxSize(const Annotation &, qreal);
void fitTextAnnotationRect(Annotation &, std::optional<qreal> = std::nullopt);
}
namespace markshot::ruler {
using Annotation = markshot::shot::Annotation;
using RulerMode = markshot::shot::RulerMode;
QRectF rulerBounds(const Annotation &);
void rotateAnglePoints(Annotation &, QPointF, qreal);
}
namespace markshot::annotation {
constexpr qreal kMinimumRotationRadius = 0.1, kRotationSnapStep = 45.0;
std::optional<qreal> rotationDragDegrees(QPointF, QPointF, QPointF);
qreal snappedRotationDegrees(qreal);
}

class ShotWindow {
public:
    using Annotation = markshot::shot::Annotation;
    using Tool = markshot::shot::Tool;
    using MagnifierShape = markshot::shot::MagnifierShape;
    enum class SelectionDrag { None, Move, Left, Right, Top, Bottom, TopLeft, TopRight, BottomLeft, BottomRight, MagnifierSource, MagnifierLens, MagnifierSourceLeft, MagnifierSourceRight, MagnifierSourceTop, MagnifierSourceBottom, MagnifierSourceTopLeft, MagnifierSourceTopRight, MagnifierSourceBottomLeft, MagnifierSourceBottomRight, Rotate, LineControl, LineStart, LineEnd, NumberTip, NumberBubble, AngleVertex, AngleFirstArm, AngleSecondArm };
    QImage m_frozenFrame = QImage(640, 480, QImage::Format_ARGB32_Premultiplied);
    QRectF m_frozenImageRect = QRectF(0, 0, 640, 480);
    QVector<Annotation> m_annotations, m_annotationsBeforeDrag;
    QVector<int> m_selectedAnnotationIds;
    std::optional<int> m_selectedAnnotationId;
    std::optional<Annotation> m_draft;
    SelectionDrag m_annotationDrag = SelectionDrag::None;
    Annotation m_annotationBeforeDrag;
    QRectF m_annotationBoundsBeforeDrag;
    QPointF m_dragStart;
    bool m_annotationHistoryCaptured = false, m_imageSelected = false, m_imagePanning = false;
    int m_lineSkeletonDragPointIndex = -1, history = 0;
    QTextEdit *m_textEditor = nullptr;
    std::optional<int> m_editingTextAnnotationId;
    Tool m_tool = Tool::Select;
    qreal m_strokeWidth = 3.0, m_highlighterWidth = 12.0, m_textSize = 1.0, m_numberWidth = 3.0, m_mosaicBlockSize = 14.0;
    QColor m_currentColor = QColor(220, 40, 50), m_textBackgroundColor = Qt::transparent;
    QFont::Weight m_textWeight = QFont::DemiBold;
    bool m_textItalic = false;
    struct { QWidget *m_colorPalette = nullptr; bool m_propertyColorEditingTextBackground = false, m_propertyColorEditHistoryCaptured = false; } m_controls;
    struct { qreal width = 3.0; } m_ruler;

    /** 【标注多选对照】【对象读取】按id读取实际持久数组中的对象，无额外选择过滤 */
    Annotation *annotationById(int id) { for (auto &a : m_annotations) if (a.id == id) return &a; return nullptr; }
    /** 【标注多选对照】【只读对象】按id返回实际持久数组中的对象 */
    const Annotation *annotationById(int id) const { for (const auto &a : m_annotations) if (a.id == id) return &a; return nullptr; }
    /** 【标注多选对照】【历史捕获】无参数，记录生产群组更新调用捕获历史的实际次数 */
    void pushHistorySnapshot() { ++history; }
    /** 【标注多选对照】【历史前置】本探针没有挂起的滚轮历史，无参数、无返回值 */
    void commitAnnotationWidthWheelHistory() {}
    /** 【标注多选对照】【界面省略】无原生窗口绘制，不影响被验证的模型状态，无返回值 */
    void update() {}
    void updateAnnotationPropertyPanel() {}
    void updateColorPalettePreview() {}
    void persistAnnotationState() {}
    void applySnowTextEditorChromeIfEnabled() { assert(false); }
    /** 【标注多选对照】【未执行分支】多选不会进入专用单项放大镜更新，执行即失败 */
    bool isMagnifierResizeOrMoveDrag() const { assert(false); return false; }
    void updateMagnifierDrag(QPointF) { assert(false); }
    bool updateLineAnchorDrag(QPointF);
    bool annotationSupportsLineAnchors(const Annotation &) const;
    bool annotationSupportsLineControl(const Annotation &) const;
    QPointF annotationLineControlPoint(const Annotation &) const;
    int lineAnchorPointIndexAt(const Annotation &, QPointF) const { assert(false); return -1; }
    /** 【标注多选对照】【矩形映射】widget坐标与image坐标相同，原图与窗口均为640乘480 */
    QPointF imageToWidget(QPointF point) const { return point; }
    QPointF widgetToImage(QPointF point) const { return point; }

    QPointF clampImagePoint(QPointF) const;
    qreal annotationSizeScale(bool) const;
    bool annotationSupportsRotation(const Annotation &) const;
    QRectF annotationUnrotatedBounds(const Annotation &) const;
    QRectF annotationBounds(const Annotation &) const;
    QPointF annotationRotationCenter(const Annotation &, bool) const;
    QPointF rotatedPoint(QPointF, QPointF, qreal) const;
    QRectF rotatedRectBounds(QRectF, qreal, QPointF) const;
    QRectF textContentRect(const Annotation &, bool) const;
    QVector<int> selectedAnnotationIds() const;
    void setSelectedAnnotations(QVector<int>);
    QRectF selectedAnnotationsBounds() const;
    QVector<int> annotationsInRect(QRectF) const;
    QRectF resizedBounds(QRectF, SelectionDrag, QPointF, bool) const;
    void moveAnnotation(Annotation &, QPointF) const;
    void transformAnnotation(Annotation &, QRectF, QRectF) const;
    void updateAnnotationDrag(QPointF, bool, bool);
    bool setSelectedAnnotationWidth(int, bool);
    void setCurrentColor(QColor);
    void applyPropertyColor(QColor);
    /** 【标注多选对照】【无选择分支】宽度样本始终有实际选择，无选择读取默认值则失败 */
    qreal currentToolWidth() const { assert(false); return 0.0; }
    QPointF clampedMagnifierCircleCenter(QPointF, qreal) const;
    QRectF magnifierCircleRect(QPointF, qreal) const;
    QRectF clampedMagnifierRect(QRectF) const;
    QRectF magnifierSourceRect(const Annotation &) const;
};

class ShotWindowRulerTool {
    ShotWindow &m_window;
public:
    /** 【标注多选对照】【尺子关联】按window关联生产尺子群组旋转，无额外字段 */
    explicit ShotWindowRulerTool(ShotWindow &window) : m_window(window) {}
    /** 【标注多选对照】【群组分支】专用updateDrag仅适用单项，样本强制有多项并继续生产群组分支 */
    bool updateDrag(QPointF, bool, bool) const { assert(m_window.selectedAnnotationIds().size() > 1); return false; }
    bool rotateInGroup(ShotWindow::Annotation &, const ShotWindow::Annotation &, QPointF, qreal) const;
};
