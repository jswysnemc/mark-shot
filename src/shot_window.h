#pragma once

#include <QColor>
#include <QElapsedTimer>
#include <QImage>
#include <QKeySequence>
#include <QPointF>
#include <QRect>
#include <QRectF>
#include <QStringList>
#include <QVector>
#include <QWidget>

#include "display_capture/display_capture_target.h"
#include "toolbar_appearance_config.h"
#include "ui/theme.h"

#include <array>
#include <optional>

class QPainter;
class QShowEvent;
class QByteArray;
class QBoxLayout;
class QComboBox;
class QCloseEvent;
class QKeyEvent;
class QLabel;
class QListWidget;
class QPushButton;
class QScrollBar;
class QScreen;
class QSlider;
class QTextEdit;
class QTimer;
class QWheelEvent;

namespace markshot::ui {
class ColorPicker;
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

public:
    // Toolbar and shortcut commands. The enum is intentionally dense because
    // m_actionShortcuts indexes it by the underlying integer value.
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

    // Editing tools available after a region is selected. These values are also
    // used as stable config names and shortcut-table indexes.
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
    };

    // Desktop file entry used by the Open With panel.
    struct DesktopApp {
        QString name;
        QString desktopPath;
        QString exec;
        QString icon;
    };

    // User-configured external command. Placeholders can receive the current
    // selection geometry or a temporary PNG rendered from the selection.
    struct ExtensionCommand {
        QString name;
        QString command;
        QString workingDirectory;
        QString description;
        bool saveImage = false;
        bool closeOnStart = true;
    };

    explicit ShotWindow(QImage frozenFrame,
                        QString outputName,
                        QRect sourceGeometry = {},
                        QVector<QRect> windowGeometries = {},
                        bool windowDetectionEnabled = true,
                        QWidget *parent = nullptr);
    static std::optional<Tool> toolFromName(QString name);
    static QStringList supportedToolNames();
    bool configureLayerShell(QScreen *screen);
    void updateLayerShellForIme();
    void startFullscreenAnnotation();
    void setImageNavigationEnabled(bool enabled);
    void setDefaultTool(Tool tool);
    void setDefaultTools(Tool tool, Tool fullscreenTool);
    void setDefaultColor(QColor color);
    void showDisplayCaptureTargets(QVector<markshot::display_capture::Target> targets);

signals:
    void selectionActivated(ShotWindow *window);
    void displayCaptureSnapshotRequested(ShotWindow *window);
    void displayCaptureEditRequested(ShotWindow *window, markshot::display_capture::Target target);
    void sessionCancelRequested();

protected:
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
    // High-level interaction mode: first pick a capture region, then edit the
    // selected image area and its annotations.
    enum class Mode {
        Selecting,
        Editing,
    };

    // Startup-only tools run before regular selection/editing gestures.
    enum class StartupTool {
        None,
        ColorPicker,
        Ruler,
        CodeScanner,
    };

public:
    // Arrow renderer variants. The default uses a filled tapered shaft; KDE uses
    // a constant-width open arrow to match Spectacle-style annotations.
    enum class ArrowStyle {
        Fletched,
        Kde,
        BidirectionalFletched,
        BidirectionalKde,
    };

    // Highlighter can either follow the freehand stroke path or constrain to a
    // single editable line.
    enum class HighlighterStyle {
        Freehand,
        StraightLine,
    };

    // Magnifier lens shape. Circle keeps the classic round loupe; Rectangle
    // draws a box lens that can have independent width/height when resized.
    enum class MagnifierShape {
        Circle,
        Rectangle,
    };

    // 矩形工具的视觉风格。Stroke 即默认描边/填充样式;Highlight 类似荧光笔,
    // 在矩形区域用半透明色 Multiply 混合;Invert 对矩形覆盖区域做像素反色。
    enum class RectangleStyle {
        Stroke,
        Highlight,
        Invert,
    };

    // Number badge display styles. Existing annotations keep their own style so
    // changing the active tool default does not rewrite old markers.
    enum class NumberStyle {
        Arabic,
        UpperAlpha,
        LowerAlpha,
        UpperRoman,
        LowerRoman,
        HeavenlyStem,
        Chinese,
    };

private:

    // Active drag target for selected annotations. Line and magnifier controls
    // live beside the usual resize/move handles.
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
        // 放大镜小框(source 取景框)的 8 向 resize 把手,与 lens 大框的把手区分开,
        // 以便交互层独立处理小框/大框的尺寸调整。
        MagnifierSourceLeft,
        MagnifierSourceRight,
        MagnifierSourceTop,
        MagnifierSourceBottom,
        MagnifierSourceTopLeft,
        MagnifierSourceTopRight,
        MagnifierSourceBottomLeft,
        MagnifierSourceBottomRight,
    };

    // Canonical annotation record. Geometry is stored in image pixels, not
    // widget coordinates; painting maps it through imageToWidget() only when
    // rendering the live window. The meaning of rect and points depends on tool:
    // shapes use rect, strokes/lines/arrows use points, and magnifier uses both.
    struct Annotation {
        int id = 0;            // Stable id used for selection and history.
        Tool tool = Tool::Pen;
        QRectF rect;           // Image-space bounds for area-based tools.
        QVector<QPointF> points; // Image-space control points for path tools.
        QString text;
        int number = 0;
        QColor color = QColor(255, 77, 77);
        QColor backgroundColor = QColor(0, 0, 0, 0);
        qreal width = 4.0;     // Tool-specific size: stroke width, font scale, or mosaic block size.
        bool filled = false;
        qreal cornerRadius = 0.0;
        ArrowStyle arrowStyle = ArrowStyle::Fletched;
        HighlighterStyle highlighterStyle = HighlighterStyle::StraightLine;
        qreal rotationDegrees = 0.0;
        qreal magnifierScale = 2.75;
        MagnifierShape magnifierShape = MagnifierShape::Circle;
        NumberStyle numberStyle = NumberStyle::Arabic;
        QString fontFamily = markshot::theme::textFontFamily();
        RectangleStyle rectangleStyle = RectangleStyle::Stroke;
    };

    // Undo/redo captures only the annotation graph and id counters. The frozen
    // image and selected capture region remain immutable for a ShotWindow.
    struct HistorySnapshot {
        QVector<Annotation> annotations;
        std::optional<int> selectedAnnotationId;
        QVector<int> selectedAnnotationIds;
        int nextNumber = 1;
        int nextAnnotationId = 1;
    };

    // Transient laser strokes are painted on top of annotations and expire by
    // timer, so they are intentionally excluded from HistorySnapshot.
    struct LaserStroke {
        QVector<QPointF> points;
        QColor color;
        qreal width = 10.0;
        qint64 expiresAt = 0;
    };

    void initializeToolbar();
    void initializeImageScrollBars();
    void initializeActionToolbar();
    void initializeShortcuts();
    void initializeTransientPanels();
    void initializeTextEditor();
    void initializeLaserTimer();
    void initializeWindowDetection(QVector<QRect> windowGeometries, bool enabled);
    QPushButton *addToolbarButton(Action action, const QString &shortcutText, QWidget *parentToolbar = nullptr);
    QVector<DesktopApp> imageDesktopApps() const;
    QVector<ExtensionCommand> extensionCommands(QString *errorMessage = nullptr) const;
    QImage renderedSelection() const;
    QPointF clampImagePoint(QPointF point) const;
    QImage mosaicImage(QRect sourceRect, int blockSize) const;
    QString currentToolName() const;
    QPointF widgetToImage(QPointF point) const;
    QPointF imageToWidget(QPointF point) const;
    QRectF normalizedSelection() const;
    QString slurpSelectionGeometry() const;
    QRect selectionGlobalRect() const;
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
    void copySelection();
    void redoAnnotation();
    void drawAnnotation(QPainter &painter, const Annotation &annotation, bool widgetCoordinates) const;
    void drawArrow(QPainter &painter,
                   const QVector<QPointF> &points,
                   qreal width,
                   ArrowStyle style) const;
    void drawMosaic(QPainter &painter, QRectF imageRect, qreal blockSize, bool widgetCoordinates) const;
    void drawRectangle(QPainter &painter, const Annotation &annotation, bool widgetCoordinates) const;
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
    QString saveSelectionToTempFile() const;
    void setCurrentColor(QColor color);
    void saveSelectionAs();
    void saveSelection();
    void revealSelectionInfo();
    void openSettingsAfterClosingCapture();
    void setTool(Tool tool);
    void toggleCaptureScope();
    void toggleToolbarLayout();
    void applyToolbarLayout();
    void enterFullscreenAnnotation(bool resetAnnotations);
    void leaveFullscreenAnnotation();
    void toggleColorPalette(QPoint position);
    void toggleOpenWithPanel();
    void toggleExtensionPanel();
    void hideAnnotationPropertyPanels();
    void hideTransientPanels();
    void updateCursor();
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
    bool setSelectedAnnotationWidth(int width);
    bool setSelectedAnnotationWidth(int width, bool captureHistory);
    void setSelectedAnnotationOpacity(int opacity);
    void setSelectedAnnotationFilled(bool filled);
    void setSelectedAnnotationCornerRadius(int radius);
    void setSelectedAnnotationArrowStyle(ArrowStyle style);
    void setSelectedRectangleStyle(RectangleStyle style);
    void setSelectedHighlighterStyle(HighlighterStyle style);
    void setSelectedNumberStyle(NumberStyle style);
    void resetNumberSequence();
    void setSelectedMagnifierScale(int scaleValue);
    void setSelectedMagnifierShape(MagnifierShape shape);
    void toggleMagnifierShape();
    void setSelectedTextFontFamily(const QString &fontFamily);
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
    void setStartupTool(StartupTool tool);
    void leaveStartupTool();
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
    void drawStartupColorLoupe(QPainter &painter, QPointF imagePoint) const;
    void drawStartupRuler(QPainter &painter) const;
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
    std::array<QKeySequence, static_cast<int>(Action::Cancel) + 1> m_actionShortcuts;
    std::array<QKeySequence, static_cast<int>(Tool::Laser) + 1> m_toolShortcuts;
    QKeySequence m_startupColorPickerShortcut;
    QKeySequence m_startupRulerShortcut;
    QKeySequence m_startupCodeScannerShortcut;
    QKeySequence m_startupDisplayCaptureShortcut;
    bool m_dragging = false;
    bool m_annotationHistoryCaptured = false;
    bool m_annotationSelectionBoxActive = false;
    bool m_fullscreenAnnotation = false;
    bool m_toolbarDragging = false;
    bool m_toolbarUserPlaced = false;
    bool m_committingText = false;
    bool m_showSelectionInfo = false;
    bool m_showWheelPreview = false;
    QElapsedTimer m_selectionInfoTimer;
    QElapsedTimer m_ctrlTapTimer;
    QPointF m_wheelPreviewPosition;
    QElapsedTimer m_wheelPreviewTimer;
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
    qreal m_textSize = 3.0;
    qreal m_mosaicBlockSize = 14.0;
    qreal m_magnifierScale = 2.75;
    MagnifierShape m_magnifierShape = MagnifierShape::Circle;
    bool m_shapeFilled = false;
    std::array<bool, static_cast<int>(Tool::Laser) + 1> m_autoSelectAfterDrawByTool = {};
    qreal m_rectangleCornerRadius = 0.0;
    RectangleStyle m_rectangleStyle = RectangleStyle::Stroke;
    ArrowStyle m_arrowStyle = ArrowStyle::Fletched;
    HighlighterStyle m_highlighterStyle = HighlighterStyle::StraightLine;
    NumberStyle m_numberStyle = NumberStyle::Arabic;
    QString m_textFontFamily = markshot::theme::textFontFamily();
    QColor m_textBackgroundColor = QColor(0, 0, 0, 0);
    int m_nextNumber = 1;
    int m_nextAnnotationId = 1;
    std::optional<int> m_selectedAnnotationId;
    QVector<int> m_selectedAnnotationIds;
    QVector<Annotation> m_annotations;
    std::optional<Annotation> m_draft;
    QVector<LaserStroke> m_laserStrokes;
    std::optional<LaserStroke> m_laserDraft;

    // Persistent widgets owned by the annotation window. Their geometry is
    // recomputed after selection changes, zoom/pan changes, and panel toggles.
    QWidget *m_toolbar = nullptr;
    QBoxLayout *m_toolbarLayout = nullptr;
    QScrollBar *m_horizontalImageScrollBar = nullptr;
    QScrollBar *m_verticalImageScrollBar = nullptr;
    QWidget *m_actionToolbar = nullptr;
    QWidget *m_annotationPropertyPanel = nullptr;
    QLabel *m_annotationPropertyTitle = nullptr;
    QLabel *m_propertyWidthLabel = nullptr;
    QSlider *m_propertyWidthSlider = nullptr;
    QLabel *m_propertyOpacityLabel = nullptr;
    QSlider *m_propertyOpacitySlider = nullptr;
    QPushButton *m_propertyColorButton = nullptr;
    QPushButton *m_propertyTextBackgroundButton = nullptr;
    QPushButton *m_propertyFillButton = nullptr;
    QLabel *m_propertyRadiusGlyphLabel = nullptr;
    QLabel *m_propertyRadiusLabel = nullptr;
    QSlider *m_propertyRadiusSlider = nullptr;
    QLabel *m_propertyArrowStyleLabel = nullptr;
    QComboBox *m_propertyArrowStyleCombo = nullptr;
    QComboBox *m_propertyHighlighterStyleCombo = nullptr;
    QComboBox *m_propertyNumberStyleCombo = nullptr;
    QComboBox *m_propertyRectangleStyleCombo = nullptr;
    QPushButton *m_propertyResetNumberButton = nullptr;
    QLabel *m_propertyMagnifierScaleGlyphLabel = nullptr;
    QLabel *m_propertyMagnifierScaleLabel = nullptr;
    QSlider *m_propertyMagnifierScaleSlider = nullptr;
    QPushButton *m_propertyMagnifierShapeButton = nullptr;
    QPushButton *m_propertyFontButton = nullptr;
    QWidget *m_propertyFontPanel = nullptr;
    QListWidget *m_propertyFontList = nullptr;
    QPushButton *m_propertyEditTextButton = nullptr;
    QWidget *m_propertyColorDialogPanel = nullptr;
    markshot::ui::ColorPicker *m_propertyColorPicker = nullptr;
    bool m_propertyColorEditHistoryCaptured = false;
    bool m_propertyColorEditingTextBackground = false;
    QWidget *m_openWithPanel = nullptr;
    QWidget *m_extensionPanel = nullptr;
    QWidget *m_startupColorPanel = nullptr;
    markshot::display_capture::DisplayCapturePicker *m_displayCapturePicker = nullptr;
    QVector<markshot::display_capture::Target> m_displayCaptureTargets;
    QWidget *m_colorPalette = nullptr;
    QWidget *m_colorPalettePreview = nullptr;
    QPoint m_colorPaletteAnchor;
    QPoint m_toolbarDragStart;
    QRect m_toolbarBeforeDrag;
    std::optional<QRectF> m_selectionBeforeFullscreenAnnotation;
    QVector<QPushButton *> m_fullscreenActionButtons;
    bool m_toolbarVerticalLayout = false;

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
    std::optional<HistorySnapshot> m_annotationWidthWheelHistorySnapshot;
    int m_annotationWidthWheelHistoryContext = 0;
    QTextEdit *m_textEditor = nullptr;
    QPointF m_textEditorImagePoint;
    std::optional<int> m_editingTextAnnotationId;
    QVector<HistorySnapshot> m_undoStack;
    QVector<HistorySnapshot> m_redoStack;
    QVector<QRect> m_windowRects;
    std::optional<QRect> m_hoveredWindowRect;
    QPointF m_selectionClickStart;
};
