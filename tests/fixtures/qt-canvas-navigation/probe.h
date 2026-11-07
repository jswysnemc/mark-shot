// 【画布导航对照】【宿主定义】承载完整生产几何和滚轮函数，非几何业务回调单独登记，不替代核心算法
#pragma once
#include <QApplication>
#include <QCursor>
#include <QImage>
#include <QJsonArray>
#include <QJsonObject>
#include <QScrollBar>
#include <QSignalBlocker>
#include <QTextEdit>
#include <QWheelEvent>
#include <QWidget>
#include <algorithm>
#include <cassert>
#include <cmath>
#include <optional>
#include "constants.inc"
#include "ui/canvas_scrollbar_style.h"

class ShotWindow : public QWidget {
public:
    enum class Mode { Selecting, Editing };
    enum class Tool { Move, Select, Pen, Line, Highlighter, Rectangle, Ellipse, Marker, Arrow, Magnifier, Laser, Ruler, Text, Mosaic, Number };
    enum class StartupTool { None, Ruler, ColorPicker };
    enum class WheelPreview { None, ImageZoom, ToolSize };
    struct Annotation { int id = 7; Tool tool = Tool::Rectangle; qreal width = 3; };
    struct HistorySnapshot {};
    struct Controls { QScrollBar *m_horizontalImageScrollBar = nullptr; QScrollBar *m_verticalImageScrollBar = nullptr; } m_controls;
    QImage m_frozenFrame;
    QRectF m_frozenImageRect;
    Mode m_mode = Mode::Editing;
    Tool m_tool = Tool::Select;
    StartupTool m_startupTool = StartupTool::None;
    bool m_imageNavigationEnabled = false;
    qreal m_imageZoom = 1;
    QPointF m_imageCenter;
    bool m_imageCenterInitialized = false;
    bool m_imagePanning = false;
    bool m_syncingImageScrollBars = false;
    QPointF m_imagePanStartWidget, m_imagePanStartCenter;
    bool m_dragging = false, m_toolbarDragging = false;
    QObject *m_regionDetector = nullptr;
    bool m_selectionLoupeEnabled = false;
    qreal m_startupColorLoupeSize = 112;
    QPointF m_startupHoverImagePoint;
    bool m_startupHoverValid = false;
    QTextEdit *m_textEditor = nullptr;
    std::optional<int> m_editingTextAnnotationId;
    QVector<int> ids;
    QVector<Annotation> members;
    int m_annotationWidthWheelContext = -1;
    qreal m_annotationWidthWheelRemainder = 0;
    QString wheelRoute = QStringLiteral("none");
    int widthRequested = 0, historyCaptured = 0;

    /** 【画布导航对照】【宿主构造】创建原生滚动条和文本控件，返回窗口对象，不执行完整ShotWindow初始化 */
    ShotWindow() {
        m_controls.m_horizontalImageScrollBar = new QScrollBar(Qt::Horizontal, this);
        m_controls.m_verticalImageScrollBar = new QScrollBar(Qt::Vertical, this);
        markshot::ui::setupCanvasScrollBar(m_controls.m_horizontalImageScrollBar, kImageScrollBarExtent);
        markshot::ui::setupCanvasScrollBar(m_controls.m_verticalImageScrollBar, kImageScrollBarExtent);
        m_textEditor = new QTextEdit(this);
        m_textEditor->hide();
    }
    bool imageNavigationAvailable() const;
    bool wheelZoomsImage() const;
    QPointF widgetToImage(QPointF point) const;
    QPointF imageToWidget(QPointF point) const;
    QPointF clampImagePoint(QPointF point) const;
    void updateFrozenImageRect();
    void zoomImageAt(qreal factor, QPointF widgetAnchor);
    void resetImageZoom();
    void panImageTo(QPointF widgetPosition);
    void updateImageScrollBars();
    void setImageCenterFromScrollBars();
    void wheelEvent(QWheelEvent *event) override;
    /** 【画布导航对照】【布局回调】生产布局回调只更新滚动条，不建立其他面板，无返回值 */
    void refreshViewGeometry() { updateImageScrollBars(); }
    /** 【画布导航对照】【选区分支】本宿主没有选区拖动，返回false，无参数 */
    bool canAdjustSelection() const { return false; }
    /** 【画布导航对照】【区域检测】区域检测器始终为空，本回调不可执行，返回false */
    bool cycleHoveredRegion(QPoint, int) { assert(false); return false; }
    /** 【画布导航对照】【选择坐标】按point调用完整生产投影，返回原图坐标 */
    QPointF selectingPointerImagePoint(QPointF point) const { return widgetToImage(point); }
    /** 【画布导航对照】【选择准备】使用采集输入中的既有成员，不创建草稿，无返回值 */
    void selectLatestDrawnAnnotationForWheel() {}
    /** 【画布导航对照】【编号读取】返回本样本明确提供的选中编号，无参数 */
    QVector<int> selectedAnnotationIds() const { return ids; }
    /** 【画布导航对照】【成员读取】按id返回本样本原始成员，无效编号返回空指针 */
    const Annotation *annotationById(int id) const { for (const auto &a : members) if (a.id == id) return &a; return nullptr; }
    /** 【画布导航对照】【工具宽度】返回样本固定初始宽度3，无参数 */
    qreal currentToolWidth() const { return 3; }
    /** 【画布导航对照】【历史观察】返回空历史对象，仅统计实际生产函数是否请求历史，无参数 */
    HistorySnapshot currentHistorySnapshot() { return {}; }
    /** 【画布导航对照】【宽度观察】按width记录真实生产分派结果，返回false以终止后续宽度业务 */
    bool setSelectedAnnotationWidth(int width, bool) { wheelRoute = QStringLiteral("width"); widthRequested = width; return false; }
    /** 【画布导航对照】【历史回调】不可到达的宽度提交回调，参数为上下文和历史，无返回值 */
    void queueAnnotationWidthWheelHistory(int, HistorySnapshot) { ++historyCaptured; assert(false); }
    /** 【画布导航对照】【字体回调】宽度提交始终未发生，本回调不可执行，无返回值 */
    void applyEditorDisplayFontIfEnabled() { assert(false); }
    /** 【画布导航对照】【文本回调】宽度提交始终未发生，本回调不可执行，无返回值 */
    void applySnowTextEditorGeometryIfEnabled() { assert(false); }
    /** 【画布导航对照】【缩放观察】按preview登记真实生产缩放分派，point只作为提示位置，无返回值 */
    void startWheelPreview(QPointF, WheelPreview preview) { assert(preview == WheelPreview::ImageZoom); wheelRoute = QStringLiteral("zoom"); }
};
