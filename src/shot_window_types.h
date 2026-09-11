#pragma once

#include "ui/theme.h"

#include <QColor>
#include <QFont>
#include <QPointF>
#include <QRectF>
#include <QString>
#include <QVector>

#include <optional>

namespace markshot::shot::types {

/// @brief 工具栏与快捷键使用的动作编号，已有编号保持稳定
enum class Action {
    ToolMove,
    ToolSelect,
    ToolPen,
    ToolLine,
    ToolHighlighter,
    ToolRectangle,
    ToolEllipse,
    ToolArrow,
    ToolText,
    ToolNumber,
    ToolMosaic,
    ToolMagnifier,
    ToolLaser,
    ToolMarker,
    ToggleCaptureScope,
    ToggleToolbarLayout,
    Clear,
    Undo,
    Redo,
    OpenWith,
    Extensions,
    Pin,
    ScrollCapture,
    OcrCopy,
    Copy,
    Save,
    Upload,
    Settings,
    Cancel,
};

/// @brief 截图标注工具类型
enum class Tool {
    Move,
    Select,
    Pen,
    Line,
    Highlighter,
    Rectangle,
    Ellipse,
    Arrow,
    Text,
    Number,
    Mosaic,
    Magnifier,
    Laser,
    Marker,
};

/// @brief 打开方式使用的桌面应用信息
struct DesktopApp {
    QString name;
    QString desktopPath;
    QString exec;
    QString icon;
};

/// @brief 外部扩展命令及图像参数配置
struct ExtensionCommand {
    QString name;
    QString command;
    QString workingDirectory;
    QString description;
    bool saveImage = false;
    bool closeOnStart = true;
};

/// @brief 截图窗口的选区和编辑模式
enum class Mode {
    Selecting,
    Editing,
};

/// @brief 选区前可使用的辅助工具
enum class StartupTool {
    None,
    ColorPicker,
    Ruler,
    CodeScanner,
    GifRecorder,
    VideoRecorder,
};

/// @brief 箭头绘制样式
enum class ArrowStyle {
    Fletched,
    Kde,
    BidirectionalFletched,
    BidirectionalKde,
};

/// @brief 荧光笔的自由线条或直线样式
enum class HighlighterStyle {
    Freehand,
    StraightLine,
};

/// @brief 放大镜的透镜形状
enum class MagnifierShape {
    Circle,
    Rectangle,
};

/// @brief 矩形标注的描边、高亮或反色样式
enum class RectangleStyle {
    Stroke,
    Highlight,
    Invert,
};

/// @brief 图章形状，已发布编号不得重排
enum class MarkerShape {
    Triangle = 0,
    Star,
    Check,
    Cross,
    Diamond,
    Heart,
    Hexagon,
    Circle,
    Square,
    Pentagon,
    Plus,
    ArrowUp,
    Spade,
    Club,
    Lightning,
    Ban,
    Octagon,
    Crescent,
    Pin,
    Flag,
    Bookmark,
    Shield,
    Exclamation,
    SpeechBubble,
};

/// @brief 序号标记的文字样式
enum class NumberStyle {
    Arabic,
    UpperAlpha,
    LowerAlpha,
    UpperRoman,
    LowerRoman,
    HeavenlyStem,
    Chinese,
};

/// @brief 选区与标注的拖动控制点类型
enum class SelectionDrag {
    None,
    Move,
    Rotate,
    LineControl,
    LineStart,
    LineEnd,
    MagnifierSource,
    MagnifierLens,
    NumberTip,
    NumberBubble,
    Left,
    Right,
    Top,
    Bottom,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    MagnifierSourceLeft,
    MagnifierSourceRight,
    MagnifierSourceTop,
    MagnifierSourceBottom,
    MagnifierSourceTopLeft,
    MagnifierSourceTopRight,
    MagnifierSourceBottomLeft,
    MagnifierSourceBottomRight,
};

/// @brief 以图像像素坐标保存的标注数据
struct Annotation {
    int id = 0;
    Tool tool = Tool::Pen;
    QRectF rect;
    QVector<QPointF> points;
    QString text;
    int number = 0;
    QColor color = QColor(255, 77, 77);
    QColor backgroundColor = QColor(0, 0, 0, 0);
    qreal width = 4.0;
    bool filled = false;
    qreal cornerRadius = 0.0;
    ArrowStyle arrowStyle = ArrowStyle::Fletched;
    HighlighterStyle highlighterStyle = HighlighterStyle::StraightLine;
    qreal rotationDegrees = 0.0;
    qreal magnifierScale = 2.75;
    MagnifierShape magnifierShape = MagnifierShape::Circle;
    NumberStyle numberStyle = NumberStyle::Arabic;
    QString fontFamily = markshot::theme::textFontFamily();
    QFont::Weight fontWeight = QFont::DemiBold;
    bool textItalic = false;
    RectangleStyle rectangleStyle = RectangleStyle::Stroke;
    MarkerShape markerShape = MarkerShape::Triangle;
};

/// @brief 标注撤销与重做使用的数据快照
struct HistorySnapshot {
    QVector<Annotation> annotations;
    std::optional<int> selectedAnnotationId;
    QVector<int> selectedAnnotationIds;
    int nextNumber = 1;
    int nextAnnotationId = 1;
};

/// @brief 按过期时间消退的临时激光笔轨迹
struct LaserStroke {
    QVector<QPointF> points;
    QColor color;
    qreal width = 10.0;
    qint64 expiresAt = 0;
};

}
