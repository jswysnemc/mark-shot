#pragma once

#include <QColor>
#include <QElapsedTimer>
#include <QFont>
#include <QImage>
#include <QKeySequence>
#include <QPointer>
#include <QPoint>
#include <QPointF>
#include <QRegion>
#include <QRect>
#include <QRectF>
#include <QStringList>
#include <QVector>
#include <QWidget>

#include "capture_double_click_action.h"
#include "editor_history.h"
#include "display_capture/display_capture_target.h"
#include "recording/recording_options.h"
#include "shot_window_qt_fwd.h"
#include "shot_window_controls.h"
#include "shot_window_types.h"
#include "startup_shortcut_hint.h"
#include "toolbar_appearance_config.h"
#include "toolbar_dock_layout.h"
#include "region_detection/region_detector.h"
#include "ui/theme.h"
#include "window_detection.h"

#include <QFutureWatcher>

#include <array>
#include <memory>
#include <optional>

namespace markshot::ui {
class ColorPicker;
}

namespace markshot::selection_aspect {
class SelectionSizePanel;
}

namespace markshot::display_capture {
class DisplayCapturePicker;
}

// Main capture and annotation surface. It owns the frozen screenshot, region
// selection, annotation model, tool panels, shortcuts, and export actions. The
// implementation is split into focused translation units, but this class keeps
// the single interactive state machine so pointer/keyboard gestures can update
// selection, annotations, and UI chrome consistently.
class ShotWindow final : public QWidget {
    Q_OBJECT
    friend class SelectionHoverRepaintTest;
    friend class RegionDetectionToggleTest;
    friend class CanvasImageMemoryTest;
    friend class SelectionHistoryRestoreTest;
    friend class TextAnnotationEditingTest;
    friend class ToolbarDockWindowTest;

public:
    using Action = markshot::shot::types::Action;
    using Tool = markshot::shot::types::Tool;
    using DesktopApp = markshot::shot::types::DesktopApp;
    using ExtensionCommand = markshot::shot::types::ExtensionCommand;
    using ArrowStyle = markshot::shot::types::ArrowStyle;
    using ArrowLineStyle = markshot::shot::types::ArrowLineStyle;
    using HighlighterStyle = markshot::shot::types::HighlighterStyle;
    using MagnifierShape = markshot::shot::types::MagnifierShape;
    using RectangleStyle = markshot::shot::types::RectangleStyle;
    using MosaicStyle = markshot::shot::types::MosaicStyle;
    using MarkerShape = markshot::shot::types::MarkerShape;
    using NumberStyle = markshot::shot::types::NumberStyle;

    explicit ShotWindow(QImage frozenFrame,
                        QString outputName,
                        QRect sourceGeometry = {},
                        QVector<markshot::WindowInfo> windowInfos = {},
                        bool windowDetectionEnabled = true,
                        QWidget *parent = nullptr);
    static std::optional<Tool> toolFromName(QString name);
    static QStringList supportedToolNames();
    /**
     * 【截图会话】【目标屏幕】记录本窗口对应的截图屏幕。
     * @param screen 截图屏幕，缺失时后续功能回退到窗口当前屏幕。
     * @return 无返回值。
     */
    void setCaptureScreen(QScreen *screen);
    /// @brief 按截图尺寸配置指定屏幕上的图层窗口
    /// @param screen 目标屏幕，可为空以使用默认屏幕
    /// @return 图层窗口配置成功时返回 true
    bool configureLayerShell(QScreen *screen);
    /// @brief 根据文字编辑状态调整图层并重新发布输入法光标位置
    /// @return 无返回值
    void updateLayerShellForIme();
    void startFullscreenAnnotation();
    void setImageNavigationEnabled(bool enabled);
    void setDefaultTool(Tool tool);
    void setDefaultTools(Tool tool, Tool fullscreenTool);
    void setDefaultColor(QColor color);
    void showDisplayCaptureTargets(QVector<markshot::display_capture::Target> targets);
    /**
     * 使用已确认录制配置进入区域录制选区状态。
     * @param options 已确认的录制配置。
     * @return 无返回值。
     */
    void beginRegionRecordingSelection(markshot::recording::RecordingOptions options);
    markshot::shot::types::StartupTool startupTool() const;
    QScreen *captureScreen() const;
    void applyRoutedStartupTool(markshot::shot::types::StartupTool tool);
    /** 【截图会话】【工具转移】判断当前工具能否跟随指针跨屏，拖动或录制时返回 false，无参数。 */
    bool startupToolCanFollowPointer() const;
    /** 【截图会话】【工具转移】把当前工具及待选区录制配置转给 target 覆盖层，返回是否已转移。 */
    bool transferStartupToolTo(ShotWindow *target);
    void translateSelection();

signals:
    void selectionActivated(ShotWindow *window);
    void displayCaptureSnapshotRequested(ShotWindow *window);
    void displayCaptureEditRequested(ShotWindow *window, markshot::display_capture::Target target);
    void sessionCancelRequested();
    void peerOverlaysSuspendRequested(ShotWindow *window, bool suspended);
    void startupToolRouteRequested(ShotWindow *window);
    void startupToolClaimRequested(ShotWindow *window);

protected:
    /// @brief 在实际输入处理后同步光标，并处理抓取中断
    /// @param event 窗口收到的 Qt 事件
    /// @return Qt 是否已处理当前事件
    bool event(QEvent *event) override;
    bool eventFilter(QObject *watched, QEvent *event) override;
    void keyPressEvent(QKeyEvent *event) override;
    void mouseMoveEvent(QMouseEvent *event) override;
    void mousePressEvent(QMouseEvent *event) override;
    void mouseReleaseEvent(QMouseEvent *event) override;
    void mouseDoubleClickEvent(QMouseEvent *event) override;
    void paintEvent(QPaintEvent *event) override;
    void resizeEvent(QResizeEvent *event) override;
    void showEvent(QShowEvent *event) override;
    void wheelEvent(QWheelEvent *event) override;
    void closeEvent(QCloseEvent *event) override;

private:
    using Mode = markshot::shot::types::Mode;
    using StartupTool = markshot::shot::types::StartupTool;
    using WheelPreview = markshot::shot::types::WheelPreview;
    using SelectionDrag = markshot::shot::types::SelectionDrag;
    using Annotation = markshot::shot::types::Annotation;
    using HistorySnapshot = markshot::shot::types::HistorySnapshot;
    using LaserStroke = markshot::shot::types::LaserStroke;

    void scheduleInitialSelectionRepaint(const QRegion &region);
    void flushInitialSelectionRepaint();
    QRegion initialSelectionDirtyRegion(const QRectF &previousSelection,
                                        bool previousSelectionUsable) const;
    QPointF selectingPointerImagePoint(QPointF widgetPos);
    void attachSelectionPointer();
    void detachSelectionPointerIfWarpFailed(QPoint widgetPoint);
    /// @brief 判断当前是否允许调整已完成的截图选区
    /// @return 移动工具处于可调整状态时返回 true
    bool canAdjustSelection() const;
    /// @brief 处理已完成选区的方向键调整
    /// @param event 当前键盘事件
    /// @return 已处理时返回 true
    bool handleSelectionAdjustmentKey(QKeyEvent *event);
    /// @brief 根据当前拖动点更新选区
    /// @param imagePoint 图像坐标中的指针位置
    /// @return 无返回值
    void updateSelectionDrag(QPointF imagePoint);
    /// @brief 更新选区调整后的附属控件和画面
    /// @return 无返回值
    void refreshAdjustedSelection();
    /// @brief 绘制选区创建及再次调整时的放大镜
    /// @param painter 当前绘制器
    /// @return 无返回值
    void drawSelectionAdjustmentOverlay(QPainter &painter) const;

    void initializeToolbar();
    void initializeImageScrollBars();
    void initializeActionToolbar();
    void initializePropertyFontPanel();
    void initializePropertyStyleCombos(QHBoxLayout *propertyLayout);
    void initializeShortcuts();
    void initializeTransientPanels();
    void initializeTextEditor();
    void initializeLaserTimer();
    void initializeWindowDetection(QVector<markshot::WindowInfo> windowInfos, bool enabled);
    void initializeRegionDetection();
    void applyRegionDetector(std::shared_ptr<markshot::region_detection::RegionDetector> detector);
    std::optional<QRect> hoveredRegionAt(QPoint imagePoint);
    QRegion applyHoveredRect(const std::optional<QRect> &rect);
    QRegion refreshHoveredRegion(QPoint imagePoint);
    bool cycleHoveredRegion(QPoint imagePoint, int direction);
    QPushButton *addToolbarButton(Action action, const QString &shortcutText, QWidget *parentToolbar = nullptr);
    QVector<DesktopApp> imageDesktopApps() const;
    QVector<ExtensionCommand> extensionCommands(QString *errorMessage = nullptr) const;
    QImage renderedSelection() const;
    QImage exportSelectionImage() const;
    QPointF clampImagePoint(QPointF point) const;
    QImage mosaicImage(QRect sourceRect, int blockSize) const;
    QString currentToolName() const;
    QPointF widgetToImage(QPointF point) const;
    QPointF imageToWidget(QPointF point) const;
    QRectF normalizedSelection() const;
    // 选区内双击手势：判定是否可触发、执行配置动作、回滚第一击留下的草稿（issue #85）。
    bool canRunDoubleClickAction(QPointF imagePoint) const;
    bool runConfiguredDoubleClickAction();
    void discardDraftForDoubleClick();
    QString slurpSelectionGeometry() const;
    QRect selectionGlobalRect() const;
    // 把当前选区写入持久化历史，供后续会话通过 , 和 . 键恢复（issue #79）。
    void recordSelectionHistory();
    // 沿历史浏览选区：step 为 +1 时回到更早的记录，-1 时前进到更新的记录。
    void applySelectionHistoryStep(int step);
    bool isSelectionHistoryKey(const QKeyEvent *event) const;
    void enterSelectionEditing(QPointF widgetPoint);
    QRectF imageRectToWidget(QRectF rect) const;
    QPointF clampedMagnifierCircleCenter(QPointF center, qreal diameter) const;
    QRectF magnifierCircleRect(QPointF center, qreal diameter) const;
    QRectF clampedMagnifierRect(QRectF rect) const;
    QRectF magnifierSourceRect(const Annotation &annotation) const;
    QPainterPath magnifierLensPath(const Annotation &annotation) const;
    QPainterPath magnifierSourcePath(const Annotation &annotation) const;
    QRectF textContentRect(const Annotation &annotation, bool widgetCoordinates) const;
    QString defaultSavePath() const;
    Tool defaultEditingTool() const;
    bool hasUsableSelection() const;
    bool imageNavigationAvailable() const;
    bool wheelZoomsImage() const;
    qreal annotationSizeScale(bool widgetCoordinates) const;
    qreal currentToolWidth() const;
    qreal currentToolPreviewSize() const;
    SelectionDrag selectionDragAt(QPointF imagePoint) const;
    QRectF constrainedRect(QPointF start, QPointF end) const;
    Annotation *annotationById(int id);
    const Annotation *annotationById(int id) const;
    bool annotationSupportsRotation(const Annotation &annotation) const;
    bool annotationSupportsLineControl(const Annotation &annotation) const;
    bool annotationSupportsLineAnchors(const Annotation &annotation) const;
    QPointF annotationLineControlPoint(const Annotation &annotation) const;
    int lineAnchorPointIndexAt(const Annotation &annotation, QPointF imagePoint) const;
    SelectionDrag lineAnchorDragAt(const Annotation &annotation, QPointF imagePoint) const;
    QString numberLabelText(int number, NumberStyle style) const;
    SelectionDrag numberDragAt(const Annotation &annotation, QPointF imagePoint) const;
    QPointF rotatedPoint(QPointF point, QPointF center, qreal degrees) const;
    QRectF rotatedRectBounds(QRectF rect, qreal degrees) const;
    QRectF rotatedRectBounds(QRectF rect, qreal degrees, QPointF pivot) const;
    QRectF annotationUnrotatedBounds(const Annotation &annotation) const;
    QPointF annotationRotationCenter(const Annotation &annotation, bool widgetCoordinates) const;
    QPointF annotationRotationHandlePoint(const Annotation &annotation, bool widgetCoordinates) const;
    QPointF selectionRotationHandlePoint(QRectF imageBounds, bool widgetCoordinates) const;
    QRectF annotationBounds(const Annotation &annotation) const;
    QVector<int> selectedAnnotationIds() const;
    void setSelectedAnnotations(QVector<int> annotationIds);
    QRectF selectedAnnotationsBounds() const;
    QVector<int> annotationsInRect(QRectF imageRect) const;
    SelectionDrag annotationBoundsDragAt(QPointF imagePoint, QRectF bounds) const;
    SelectionDrag selectedAnnotationsDragAt(QPointF imagePoint) const;
    SelectionDrag magnifierDragAt(const Annotation &annotation, QPointF imagePoint) const;
    bool isMagnifierResizeOrMoveDrag() const;
    void updateMagnifierDrag(QPointF imagePoint);
    static bool isMagnifierLensCornerHandle(SelectionDrag drag);
    static bool isMagnifierSourceCornerHandle(SelectionDrag drag);
    static SelectionDrag magnifierSourceHandleToGenericHandle(SelectionDrag handle);
    static QRectF magnifierResizeRectWithHandle(const QRectF &before,
                                                SelectionDrag handle,
                                                QPointF point,
                                                qreal minSize,
                                                bool keepSquare);
    void loadAnnotationStateFromDisk();
    void persistAnnotationState();
    void flushAnnotationStateNow();
    QRectF resizedBounds(QRectF start, SelectionDrag drag, QPointF imagePoint, bool keepAspectRatio) const;
    QVector<QPointF> selectionHandlePoints(QRectF rect) const;
    QRectF selectedAnnotationDeleteButtonRect() const;
    SelectionDrag annotationDragAt(QPointF imagePoint, int annotationId) const;
    std::optional<int> annotationAt(QPointF imagePoint) const;
    void drawSelectedAnnotationFrame(QPainter &painter) const;
    void drawLineAnchorHandles(QPainter &painter,
                               const Annotation &annotation,
                               QPointF center,
                               qreal angle,
                               bool rotateHandles) const;
    void moveAnnotation(Annotation &annotation, QPointF delta) const;
    void transformAnnotation(Annotation &annotation, QRectF oldBounds, QRectF newBounds) const;
    void beginAnnotationDrag(int annotationId, SelectionDrag drag, QPointF imagePoint);
    void updateAnnotationDrag(QPointF imagePoint, bool keepAspectRatio);
    bool updateLineAnchorDrag(QPointF imagePoint);
    bool insertLineSkeletonPointAt(int annotationId, QPointF imagePoint);
    bool removeSelectedLineSkeletonPoint();
    void beginAnnotationSelectionBox(QPointF imagePoint);
    void updateAnnotationSelectionBox(QPointF imagePoint);
    void commitAnnotationSelectionBox();
    HistorySnapshot currentHistorySnapshot() const;
    void restoreHistorySnapshot(const HistorySnapshot &snapshot);
    void appendHistorySnapshot(const HistorySnapshot &snapshot);
    void pushHistorySnapshot();
    void queueAnnotationWidthWheelHistory(int context, const HistorySnapshot &snapshot);
    void commitAnnotationWidthWheelHistory();
    void undoAnnotationEdit();
    void beginSelection(QPointF imagePoint);
    void commitDraft();
    void commitTextEditor();
    void showTextEditorContextMenu(const QPoint &globalPosition);
    void copySelection();
    void redoAnnotation();
    void drawAnnotation(QPainter &painter, const Annotation &annotation, bool widgetCoordinates) const;
    void drawMosaic(QPainter &painter, const Annotation &annotation, bool widgetCoordinates) const;
    /// @brief 按滤镜效果处理冻结帧中的一块区域
    /// @param sourceRect 图像坐标中的区域
    /// @param style 滤镜效果
    /// @param strength 工具强度，与马赛克块大小共用
    /// @return 处理后的区域图像，区域无效时返回空图
    QImage filteredRegion(QRect sourceRect, MosaicStyle style, qreal strength) const;
    /// @brief 统一绘制所有聚光灯矩形的压暗遮罩
    /// @param painter 绘制器
    /// @param area 需要压暗的范围，坐标与 widgetCoordinates 一致
    /// @param widgetCoordinates true 表示窗口坐标，false 表示图像坐标
    /// @return 无返回值
    void drawSpotlightOverlay(QPainter &painter, QRectF area, bool widgetCoordinates) const;
    void drawRectangle(QPainter &painter, const Annotation &annotation, bool widgetCoordinates) const;
    void drawMarker(QPainter &painter, const Annotation &annotation, bool widgetCoordinates) const;
    void drawMagnifier(QPainter &painter, const Annotation &annotation, bool widgetCoordinates) const;
    void drawNumber(QPainter &painter,
                    QPointF tipPoint,
                    QPointF bubblePoint,
                    const QString &label,
                    QColor color,
                    qreal width,
                    bool widgetCoordinates) const;
    void drawWheelPreview(QPainter &painter);
    void drawLaserStroke(QPainter &painter, const LaserStroke &stroke, bool widgetCoordinates, qreal opacity) const;
    void beginTextAnnotation(QPointF imagePoint);
    void beginEditingSelectedTextAnnotation();
    void beginLaserStroke(QPointF imagePoint);
    void updateLaserStroke(QPointF imagePoint);
    void commitLaserStroke();
    void cleanupLaserStrokes();
    void openSelectionWithDesktop(const DesktopApp &app);
    void runExtensionCommand(const ExtensionCommand &command);
    void pinSelection();
    void startScrollCapture();
    void ocrCopySelection();
    void scanCodeSelection();
    void uploadSelection();
    void showToast(const QString &text, int durationMs = 2000);
    QString saveSelectionToTempFile(bool applyExportEffect = false) const;
    void setCurrentColor(QColor color);
    void saveSelectionAs();
    void saveSelection();
    void revealSelectionInfo();
    void openSettingsAfterClosingCapture();
    void setTool(Tool tool);
    void toggleCaptureScope();
    void toggleToolbarLayout();
    void applyToolbarLayout();
    void restoreConfiguredToolbarLayout();
    std::optional<markshot::ToolbarDockEdge> toolbarDockEdge() const;
    void enterFullscreenAnnotation(bool resetAnnotations);
    void leaveFullscreenAnnotation();
    void toggleColorPalette(QPoint position);
    void toggleOpenWithPanel();
    void toggleExtensionPanel();
    void hideAnnotationPropertyPanels();
    void hideTransientPanels();
    /// @brief 按当前系统指针位置同步截图和标注光标
    /// @return 无返回值
    void updateCursor();
    /// @brief 根据当前事件位置及交互状态同步光标
    /// @param widgetPoint 指针在窗口内的位置
    /// @return 无返回值
    void updatePointerCursor(QPointF widgetPoint);
    /// @brief 在动态工具栏完成布局后异步同步释放位置的光标
    /// @param widgetPoint 最近一次真实指针事件在窗口内的位置
    /// @return 无返回值
    void scheduleUiCursorRefresh(QPointF widgetPoint);
    /// @brief 结束中断的指针操作，撤去未提交草稿并恢复抓取控件状态
    /// @return 无返回值
    void cancelPointerInteraction();
    /// @brief 判断软件指针当前是否确实会绘制到画面
    /// @return 当前存在有效软件指针时返回 true
    bool selectionPointerVisible() const;
    bool propertyComboPopupVisible() const;
    /// @brief 判断事件位置是否位于由子控件处理的界面区域
    /// @param widgetPoint 指针在窗口内的位置
    /// @return 子控件负责该位置时返回 true
    bool mouseOverUiWidget(QPointF widgetPoint) const;
    /// @brief 显示指定预览并启动独立的自动结束定时器
    /// @param widgetPoint 预览热点的窗口内位置
    /// @param preview 工具尺寸预览或图像缩放提示
    /// @return 无返回值
    void startWheelPreview(QPointF widgetPoint, WheelPreview preview);
    /// @brief 判断预览是否处于有效时间和可交互的画布区域
    /// @return 当前可以显示预览时返回 true
    bool wheelPreviewVisible() const;
    /// @brief 清除预览并立即恢复当前位置的光标
    /// @return 无返回值
    void clearWheelPreview();
    void updateColorPaletteGeometry(QPoint anchor);
    void updateColorPalettePreview();
    void updateOpenWithPanel();
    void updateOpenWithPanelGeometry();
    void updateExtensionPanel();
    void updateExtensionPanelGeometry();
    void updateAnnotationPropertyPanel();
    void updateAnnotationPropertyPanelGeometry();
    void updatePropertyColorDialogGeometry();
    void updatePropertyFontPanelGeometry();
    void updateShapeMarkerPopupGeometry();
    void toggleShapeMarkerPopup();
    void hideShapeMarkerPopup();
    void refreshShapeMarkerToolbarButton();
    bool setSelectedAnnotationWidth(int width);
    bool setSelectedAnnotationWidth(int width, bool captureHistory);
    void setSelectedAnnotationOpacity(int opacity);
    void setSelectedAnnotationFilled(bool filled);
    void setSelectedAnnotationCornerRadius(int radius);
    void setSelectedAnnotationArrowStyle(ArrowStyle style);
    /// @brief 设置当前箭头或箭头工具的线型
    /// @param style 实线、虚线或点线
    /// @return 无返回值
    void setSelectedArrowLineStyle(ArrowLineStyle style);
    /// @brief 同步箭头属性控件的可见性及当前值
    /// @param annotation 单个选中标注，未选中时为空
    /// @param visible 是否显示箭头属性
    /// @return 无返回值
    void updateArrowPropertyControls(const Annotation *annotation, bool visible);
    void setSelectedRectangleStyle(RectangleStyle style);
    /// @brief 切换选中马赛克标注或马赛克工具默认的滤镜效果
    /// @param style 滤镜效果
    /// @return 无返回值
    void setSelectedMosaicStyle(MosaicStyle style);
    /// @brief 计算拖出新选区时的矩形，锁定比例或按住 Shift 时保持比例
    /// @param anchor 拖动起点（图像坐标）
    /// @param pointer 当前指针（图像坐标）
    /// @param squareModifier 是否按住 Shift，未锁定比例时强制正方形
    /// @return 新选区
    QRectF creationSelectionRect(QPointF anchor, QPointF pointer, bool squareModifier) const;
    /// @brief 对手柄调整后的选区套用锁定比例
    /// @param before 调整前选区
    /// @param handle 拖动的边、角或整个选区
    /// @param adjusted 未约束的调整结果
    /// @return 满足锁定比例的选区；未锁定时原样返回
    QRectF aspectLockedSelection(QRectF before, SelectionDrag handle, QRectF adjusted) const;
    /// @brief 获取选区尺寸 HUD，首次调用时创建并连接信号
    /// @return 尺寸 HUD
    markshot::selection_aspect::SelectionSizePanel *ensureSelectionSizePanel();
    /// @brief 让选区尺寸 HUD 的宽度字段进入编辑，供 Ctrl+R 调用
    /// @return 无返回值
    void focusSelectionSizeInput();
    /// @brief 同步选区尺寸 HUD 的显隐、数值与位置：跟随选区左上角并避让屏幕边缘、文本输入框和工具栏
    /// @return 无返回值
    void updateSelectionSizePanelGeometry();
    /// @brief 切换选区锁定比例，并立即按新比例调整已有选区
    /// @param ratio 宽高比，0 表示自由比例
    /// @return 无返回值
    void setSelectionAspectRatio(qreal ratio);
    /// @brief 把选区设为精确尺寸
    /// @param size 目标尺寸（图像像素）
    /// @return 无返回值
    void applySelectionSize(QSize size);
    void setSelectedMarkerShape(MarkerShape shape);
    // 设置形状标记的填充模式：filled 为 false 时以描边（空心）绘制。
    void setSelectedMarkerFill(bool filled);
    void setSelectedHighlighterStyle(HighlighterStyle style);
    void setSelectedNumberStyle(NumberStyle style);
    void resetNumberSequence();
    void setSelectedMagnifierScale(int scaleValue);
    void setSelectedMagnifierShape(MagnifierShape shape);
    void toggleMagnifierShape();
    void setSelectedTextFontFamily(const QString &fontFamily);
    void setSelectedTextFontSize(int pointSize);
    void setSelectedTextBold(bool bold);
    void setSelectedTextItalic(bool italic);
    void applyPropertyColor(QColor color);
    void deleteSelectedAnnotation();
    void openSelectedAnnotationColorPalette();
    void openSelectedTextBackgroundColorPalette();
    void toggleSelectedTextFontPanel();
    void clearAnnotations();
    void updateTextEditorGeometry();
    void updateFrozenImageRect();
    void zoomImageAt(qreal factor, QPointF widgetAnchor);
    void resetImageZoom();
    void panImageTo(QPointF widgetPosition);
    void refreshViewGeometry();
    void updateImageScrollBars();
    void setImageCenterFromScrollBars();
    void updateMinimumImageWindowSize();
    void updateActionToolbarGeometry();
    void updateToolbarGeometry();
    void updateToolbarState();
    void setFullscreenActionButtonsVisible(bool visible);
    QRect clampedToolbarGeometry(QRect toolbarGeometry) const;
    QRect sidePanelGeometry(QSize panelSize) const;
    void setStartupTool(StartupTool tool);
    void leaveStartupTool();
    void beginStartupRecording(markshot::recording::RecordingMode mode);
    bool handleStartupRecordingSelection();
    void startRecording(markshot::recording::RecordingOptions options);
    std::optional<markshot::recording::RecordingMode> recordingModeForStartupTool(StartupTool tool) const;
    void toggleDisplayCapturePicker();
    void hideDisplayCapturePicker();
    bool displayCapturePickerVisible() const;
    bool displayCapturePickerContains(QPoint point) const;
    void ensureDisplayCapturePicker();
    void copyDisplayCaptureTarget(int index);
    void editDisplayCaptureTarget(int index);
    void saveDisplayCaptureTarget(int index);
    void updateDisplayCapturePickerGeometry();
    QColor sampledImageColor(QPointF imagePoint) const;
    void showStartupColorDialog(QColor color, QPoint anchor);
    void drawStartupToolOverlay(QPainter &painter);
    void drawStartupShortcutHint(QPainter &painter) const;

    /// 绘制录制中状态提示组件。@param painter 当前绘图对象。@return 无返回值。
    void drawActiveRecordingStatus(QPainter &painter);
    /// 初始化录制中状态提示刷新逻辑。@return 无返回值。
    void initializeActiveRecordingOverlay();
    /// 判断当前是否存在活动录制。@return 存在活动录制时返回 true。
    bool activeRecordingAvailable() const;
    /// 从冻结帧提示组件请求停止录制。@return 停止请求已提交时返回 true。
    bool stopActiveRecordingFromOverlay();
    /// 计算录制中提示组件的停止按钮区域。@return 停止按钮矩形。
    QRectF activeRecordingStopButtonRect() const;
    /// 判断录制中提示组件当前是否可见。@return 面板可见时返回 true。
    bool activeRecordingOverlayVisible() const;
    /// 根据指针位置更新停止按钮悬停状态。@param pointer 指针位置。@return 指针位于停止按钮上时返回 true。
    bool updateActiveRecordingStopHover(QPointF pointer);
    void drawStartupColorLoupe(QPainter &painter, QPointF imagePoint) const;
    void drawStartupRuler(QPainter &painter) const;
    QVector<markshot::startup_hint::ShortcutHintItem> startupShortcutHintItems() const;
    bool updateStartupShortcutHintAnchor(QPointF pointer);
    QKeySequence shortcutForAction(Action action) const;
    QKeySequence shortcutForTool(Tool tool) const;
    QString shortcutText(Action action, const QString &fallback = {}) const;
    QString shortcutText(Tool tool) const;
    bool eventMatchesShortcut(const QKeyEvent *event, Action action) const;
    bool eventMatchesShortcut(const QKeyEvent *event, Tool tool) const;
    bool eventMatchesStartupShortcut(const QKeyEvent *event, StartupTool tool) const;
    bool eventMatchesDisplayCaptureShortcut(const QKeyEvent *event) const;
    bool handleConfiguredActionShortcut(QKeyEvent *event);
    bool handleConfiguredToolShortcut(QKeyEvent *event);

    // Captured source image and image navigation state.
    QImage m_frozenFrame;
    QString m_outputName;
    QRect m_sourceGeometry;
    QPointer<QScreen> m_captureScreen;
    QRectF m_frozenImageRect;
    bool m_imageNavigationEnabled = false;
    qreal m_imageZoom = 1.0;
    QPointF m_imageCenter;
    bool m_imageCenterInitialized = false;
    bool m_imageSelected = false;
    bool m_imagePanning = false;
    bool m_syncingImageScrollBars = false;
    StartupTool m_startupTool = StartupTool::None;
    bool m_startupHoverValid = false;
    bool m_startupRulerDragging = false;
    bool m_startupRulerHasMeasure = false;
    qreal m_startupColorLoupeSize = 112.0;
    QPointF m_startupHoverImagePoint;
    bool m_selectionLoupeEnabled = false;
    bool m_selectionPointerDetached = false;
    bool m_selectionKeyboardAdjusting = false;
    QPoint m_selectionPointerHardwareAnchor;
    QPointF m_startupRulerStart;
    QPointF m_startupRulerEnd;
    QImage m_sharpViewportCache;
    QRectF m_sharpViewportCacheSourceRect;
    QSize m_sharpViewportCacheTargetSize;
    qreal m_sharpViewportCacheDpr = 0.0;
    QPointF m_imagePanStartWidget;
    QPointF m_imagePanStartCenter;

    // Selection and annotation interaction state. All geometry here is stored in
    // image coordinates so export and live painting share the same data model.
    QRectF m_selection;
    // 持久化选区历史的会话内缓存与浏览位置（issue #79）。历史存全局逻辑
    // 坐标，应用到当前帧时经 imageRectFromGeometry 换算回图像坐标。
    QVector<QRect> m_selectionHistory;
    int m_selectionHistoryIndex = -1;
    bool m_selectionHistoryLoaded = false;
    QPointF m_selectionStart;
    QRectF m_selectionBeforeDrag;
    QPointF m_dragStart;
    Annotation m_annotationBeforeDrag;
    QVector<Annotation> m_annotationsBeforeDrag;
    QRectF m_annotationBoundsBeforeDrag;
    QRectF m_annotationSelectionBox;
    SelectionDrag m_selectionDrag = SelectionDrag::None;
    SelectionDrag m_annotationDrag = SelectionDrag::None;
    int m_lineSkeletonDragPointIndex = -1;
    Mode m_mode = Mode::Selecting;
    Tool m_tool = Tool::Pen;
    Tool m_defaultTool = Tool::Pen;
    Tool m_fullscreenDefaultTool = Tool::Pen;
    // 选区内双击手势执行的动作，会话启动时从应用配置读入一次（issue #85）。
    markshot::CaptureDoubleClickAction m_doubleClickAction =
        markshot::CaptureDoubleClickAction::Copy;
    std::array<QKeySequence, static_cast<int>(Action::Translate) + 1> m_actionShortcuts;
    std::array<QKeySequence, static_cast<int>(Tool::Marker) + 1> m_toolShortcuts;
    QKeySequence m_startupColorPickerShortcut;
    QKeySequence m_startupRulerShortcut;
    QKeySequence m_startupCodeScannerShortcut;
    QKeySequence m_startupDisplayCaptureShortcut;
    QKeySequence m_startupGifRecorderShortcut;
    QKeySequence m_startupVideoRecorderShortcut;
    std::optional<markshot::recording::RecordingOptions> m_pendingRecordingOptions;
    bool m_recordingConfigDialogOpen = false;
    QTimer *m_activeRecordingOverlayTimer = nullptr;
    // 录制中提示面板的“停止录制”按钮悬停状态（issue: 手绘按钮无原生 hover）。
    bool m_activeRecordingStopHovered = false;
    markshot::startup_hint::PanelAnchor m_startupHintAnchor = markshot::startup_hint::PanelAnchor::BottomLeft;
    bool m_dragging = false;
    bool m_operationBusy = false;
    // Pointer input can arrive at the native refresh rate (200 Hz on some
    // Wayland displays). Keep selection geometry current for precision, but
    // coalesce expensive raster repaint requests to a bounded cadence.
    QTimer *m_initialSelectionRepaintTimer = nullptr;
    QRegion m_pendingInitialSelectionRepaint;
    bool m_annotationHistoryCaptured = false;
    bool m_annotationSelectionBoxActive = false;
    bool m_fullscreenAnnotation = false;
    bool m_toolbarDragging = false;
    bool m_toolbarUserPlaced = false;
    bool m_actionToolbarUserPlaced = false;
    bool m_committingText = false;
    bool m_showSelectionInfo = false;
    WheelPreview m_wheelPreview = WheelPreview::None;
    QElapsedTimer m_selectionInfoTimer;
    QElapsedTimer m_ctrlTapTimer;
    QPointF m_wheelPreviewPosition;
    QElapsedTimer m_wheelPreviewTimer;
    QTimer *m_wheelPreviewHideTimer = nullptr;
    quint64 m_pointerInteractionSerial = 0;
    qreal m_annotationWidthWheelRemainder = 0.0;
    int m_annotationWidthWheelContext = 0;
    QElapsedTimer m_laserClock;

    // Current tool defaults and styling. These seed new Annotation records and
    // are updated by tool panels, shortcuts, and config/CLI defaults.
    markshot::ToolbarAppearanceConfig m_toolbarAppearance;
    QColor m_currentColor = QColor(255, 77, 77);
    qreal m_strokeWidth = 3.0;
    qreal m_highlighterWidth = 6.0;
    qreal m_numberWidth = 3.0;
    // 默认文本字号为 20pt，渲染字号等于 19 加 width
    qreal m_textSize = 1.0;
    qreal m_mosaicBlockSize = 14.0;
    qreal m_magnifierScale = 2.75;
    MagnifierShape m_magnifierShape = MagnifierShape::Circle;
    bool m_shapeFilled = false;
    std::array<bool, static_cast<int>(Tool::Marker) + 1> m_autoSelectAfterDrawByTool = {};
    qreal m_rectangleCornerRadius = 0.0;
    RectangleStyle m_rectangleStyle = RectangleStyle::Stroke;
    MosaicStyle m_mosaicStyle = MosaicStyle::Pixelate;
    // 选区锁定比例（宽 / 高），0 表示自由比例，仅在本次截图会话内有效
    qreal m_selectionAspectRatio = 0.0;
    // 选区左上角尺寸 HUD，实际类型为 SelectionSizePanel
    QWidget *m_selectionSizePanel = nullptr;
    MarkerShape m_markerShape = MarkerShape::Triangle;
    // 形状标记默认填充；false 时新标记以描边（空心）绘制。
    bool m_markerFilled = true;
    ArrowStyle m_arrowStyle = ArrowStyle::Fletched;
    ArrowLineStyle m_arrowLineStyle = ArrowLineStyle::Solid;
    HighlighterStyle m_highlighterStyle = HighlighterStyle::StraightLine;
    NumberStyle m_numberStyle = NumberStyle::Arabic;
    QString m_textFontFamily = markshot::theme::textFontFamily();
    QFont::Weight m_textWeight = QFont::DemiBold;
    bool m_textItalic = false;
    QColor m_textBackgroundColor = QColor(0, 0, 0, 0);
    int m_nextNumber = 1;
    int m_nextAnnotationId = 1;
    std::optional<int> m_selectedAnnotationId;
    QVector<int> m_selectedAnnotationIds;
    QVector<Annotation> m_annotations;
    std::optional<Annotation> m_draft;
    QVector<LaserStroke> m_laserStrokes;
    std::optional<LaserStroke> m_laserDraft;

    ShotWindowControls m_controls;

    // Editor, history, and window-detection auxiliaries.
    QTimer *m_laserTimer = nullptr;
    // 标注工具默认值持久化的写盘节流:写盘走 QSaveFile 同步 IO,在拖动滑块
    // 等高频改动场景里直接写会阻塞主线程绘制,因此每次改动都重启
    // single-shot 定时器,把连续改动合并成一次写入。
    // m_annotationStateDirty 表示自上次写盘以来还有挂起的改动。
    QTimer *m_annotationStateTimer = nullptr;
    bool m_annotationStateDirty = false;
    // 选中标注通过滚轮连续改粗细时,先保存第一次改动前的快照,停止滚轮后
    // 再合并成一条撤销历史,避免每个滚轮 tick 都进入历史栈。
    QTimer *m_annotationWidthWheelHistoryTimer = nullptr;
    QTextEdit *m_textEditor = nullptr;
    QPointF m_textEditorImagePoint;
    std::optional<int> m_editingTextAnnotationId;
    markshot::rust::EditorHistory m_annotationHistory;
    QVector<markshot::WindowInfo> m_windowInfos;
    std::optional<QRect> m_hoveredWindowRect;
    std::unique_ptr<markshot::region_detection::RegionDetector> m_regionDetector;
    markshot::region_detection::SelectionCursor m_regionCursor;
    QFutureWatcher<std::shared_ptr<markshot::region_detection::RegionDetector>> *m_regionDetectorWatcher = nullptr;
    QPointF m_selectionClickStart;

    bool m_applyingRoutedStartupTool = false;
    void routeStartupToolToCursorScreenIfEnabled();
    void claimStartupToolFromPeersIfEnabled(QEvent *event);
    void rememberRecentTextFontIfEnabled(const QString &fontFamily);
    void refreshRecentTextFontsIfEnabled();
    bool ignoreRecentFontHeaderClickIfEnabled(QListWidgetItem *item) const;
    void updateAnnotationDeleteButtonHoverIfEnabled(QPointF pointer);
    bool paintAnnotationDeleteButtonIfEnabled(QPainter &painter, const QRectF &deleteButton) const;
    void handleTextToolPress(QPointF imagePoint, bool committedEditor);
    bool textToolAcceptsPoint(QPointF imagePoint) const;
    bool handleTextEditorActionShortcut(QObject *watched, QEvent *event);
    bool handleSnowTextEditorEventIfEnabled(QObject *watched, QEvent *event);
    void connectSnowTextEditorGeometryIfEnabled();
    void applySnowTextEditorChromeIfEnabled();
    bool applySnowTextEditorGeometryIfEnabled();
    void applyEditorDisplayFontIfEnabled();
    void copyEditorFontToAnnotationIfEnabled(Annotation &annotation);
    bool selectLatestDrawnAnnotationForWheel();
    bool stopActiveRecordingAndExitSelectionIfEnabled();
    bool m_annotationDeleteButtonHovered = false;
    bool m_updatingSnowTextEditorGeometry = false;

    /// @brief 手动拖动超过点击阈值后立即去掉识别选区。
    /// @return 需要重绘的区域。
    QRegion dismissRecognizedSelectionOnManualDragIfEnabled();

    /// @brief 为图片编辑窗口绘制 1px 边框。
    /// @param painter 窗口画笔。
    /// @return 无返回值。
    void paintEditorWindowBorderIfEnabled(QPainter &painter) const;
    /// @brief 记下形状标记按钮的当前形状，供悬停换色时重绘
    /// @return 无返回值
    void rememberShapeMarkerToolbarIconIfEnabled();
};
