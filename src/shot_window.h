#pragma once

#include <QColor>
#include <QElapsedTimer>
#include <QImage>
#include <QPointF>
#include <QRect>
#include <QRectF>
#include <QStringList>
#include <QWidget>

#include <optional>

class QPainter;
class QBoxLayout;
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

class ShotWindow final : public QWidget {
public:
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
        Cancel,
    };

    struct DesktopApp {
        QString name;
        QString desktopPath;
        QString exec;
        QString icon;
    };

    struct ExtensionCommand {
        QString name;
        QString command;
        QString workingDirectory;
        QString description;
        bool saveImage = false;
        bool closeOnStart = true;
    };

    explicit ShotWindow(QImage frozenFrame, QString outputName, QRect sourceGeometry = {}, QWidget *parent = nullptr);
    bool configureLayerShell(QScreen *screen);
    void startFullscreenAnnotation();
    void setImageNavigationEnabled(bool enabled);

protected:
    bool eventFilter(QObject *watched, QEvent *event) override;
    void keyPressEvent(QKeyEvent *event) override;
    void mouseMoveEvent(QMouseEvent *event) override;
    void mousePressEvent(QMouseEvent *event) override;
    void mouseReleaseEvent(QMouseEvent *event) override;
    void mouseDoubleClickEvent(QMouseEvent *event) override;
    void paintEvent(QPaintEvent *event) override;
    void resizeEvent(QResizeEvent *event) override;
    void wheelEvent(QWheelEvent *event) override;

private:
    enum class Mode {
        Selecting,
        Editing,
    };

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
        Laser,
    };

    enum class SelectionDrag {
        None,
        Move,
        Left,
        Right,
        Top,
        Bottom,
        TopLeft,
        TopRight,
        BottomLeft,
        BottomRight,
    };

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
        QString fontFamily = QStringLiteral("Sans Serif");
    };

    struct HistorySnapshot {
        QVector<Annotation> annotations;
        std::optional<int> selectedAnnotationId;
        QVector<int> selectedAnnotationIds;
        int nextNumber = 1;
        int nextAnnotationId = 1;
    };

    struct LaserStroke {
        QVector<QPointF> points;
        QColor color;
        qreal width = 10.0;
        qint64 expiresAt = 0;
    };

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
    QRectF textContentRect(const Annotation &annotation, bool widgetCoordinates) const;
    QString defaultSavePath() const;
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
    QRectF annotationBounds(const Annotation &annotation) const;
    QVector<int> selectedAnnotationIds() const;
    void setSelectedAnnotations(QVector<int> annotationIds);
    QRectF selectedAnnotationsBounds() const;
    QVector<int> annotationsInRect(QRectF imageRect) const;
    SelectionDrag annotationBoundsDragAt(QPointF imagePoint, QRectF bounds) const;
    QRectF resizedBounds(QRectF start, SelectionDrag drag, QPointF imagePoint, bool keepAspectRatio) const;
    QVector<QPointF> selectionHandlePoints(QRectF rect) const;
    QRectF selectedAnnotationDeleteButtonRect() const;
    SelectionDrag annotationDragAt(QPointF imagePoint, int annotationId) const;
    std::optional<int> annotationAt(QPointF imagePoint) const;
    void drawSelectedAnnotationFrame(QPainter &painter) const;
    void moveAnnotation(Annotation &annotation, QPointF delta) const;
    void transformAnnotation(Annotation &annotation, QRectF oldBounds, QRectF newBounds) const;
    void beginAnnotationDrag(int annotationId, SelectionDrag drag, QPointF imagePoint);
    void updateAnnotationDrag(QPointF imagePoint, bool keepAspectRatio);
    void beginAnnotationSelectionBox(QPointF imagePoint);
    void updateAnnotationSelectionBox(QPointF imagePoint);
    void commitAnnotationSelectionBox();
    HistorySnapshot currentHistorySnapshot() const;
    void restoreHistorySnapshot(const HistorySnapshot &snapshot);
    void pushHistorySnapshot();
    void undoAnnotationEdit();
    void beginSelection(QPointF imagePoint);
    void commitDraft();
    void commitTextEditor();
    void copySelection();
    void redoAnnotation();
    void drawAnnotation(QPainter &painter, const Annotation &annotation, bool widgetCoordinates) const;
    void drawArrow(QPainter &painter, QPointF start, QPointF end, qreal width) const;
    void drawMosaic(QPainter &painter, QRectF imageRect, qreal blockSize, bool widgetCoordinates) const;
    void drawNumber(QPainter &painter, QPointF imagePoint, int number, QColor color, qreal width, bool widgetCoordinates) const;
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
    void showToast(const QString &text, int durationMs = 2000);
    QString saveSelectionToTempFile() const;
    void setCurrentColor(QColor color);
    void saveSelection();
    void revealSelectionInfo();
    void setTool(Tool tool);
    void toggleCaptureScope();
    void toggleToolbarLayout();
    void applyToolbarLayout();
    void enterFullscreenAnnotation(bool resetAnnotations);
    void leaveFullscreenAnnotation();
    void toggleColorPalette(QPoint position);
    void toggleOpenWithPanel();
    void toggleExtensionPanel();
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
    void adjustSelectedAnnotationWidth(qreal delta);
    void setSelectedAnnotationWidth(int width);
    void setSelectedAnnotationOpacity(int opacity);
    void setSelectedAnnotationFilled(bool filled);
    void setSelectedAnnotationCornerRadius(int radius);
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
    QImage m_sharpViewportCache;
    QRectF m_sharpViewportCacheSourceRect;
    QSize m_sharpViewportCacheTargetSize;
    qreal m_sharpViewportCacheDpr = 0.0;
    QPointF m_imagePanStartWidget;
    QPointF m_imagePanStartCenter;
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
    Mode m_mode = Mode::Selecting;
    Tool m_tool = Tool::Pen;
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
    QElapsedTimer m_laserClock;
    QColor m_currentColor = QColor(255, 77, 77);
    qreal m_penWidth = 2.0;
    qreal m_shapeWidth = 3.0;
    qreal m_numberWidth = 3.0;
    qreal m_mosaicBlockSize = 14.0;
    qreal m_laserWidth = 10.0;
    bool m_shapeFilled = false;
    qreal m_rectangleCornerRadius = 0.0;
    QString m_textFontFamily = QStringLiteral("Sans Serif");
    QColor m_textBackgroundColor = QColor(0, 0, 0, 0);
    int m_nextNumber = 1;
    int m_nextAnnotationId = 1;
    std::optional<int> m_selectedAnnotationId;
    QVector<int> m_selectedAnnotationIds;
    QVector<Annotation> m_annotations;
    std::optional<Annotation> m_draft;
    QVector<LaserStroke> m_laserStrokes;
    std::optional<LaserStroke> m_laserDraft;
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
    QLabel *m_propertyRadiusLabel = nullptr;
    QSlider *m_propertyRadiusSlider = nullptr;
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
    QWidget *m_colorPalette = nullptr;
    QWidget *m_colorPalettePreview = nullptr;
    QPoint m_colorPaletteAnchor;
    QPoint m_toolbarDragStart;
    QRect m_toolbarBeforeDrag;
    std::optional<QRectF> m_selectionBeforeFullscreenAnnotation;
    QVector<QPushButton *> m_fullscreenActionButtons;
    bool m_toolbarVerticalLayout = false;
    QTimer *m_laserTimer = nullptr;
    QTextEdit *m_textEditor = nullptr;
    QPointF m_textEditorImagePoint;
    std::optional<int> m_editingTextAnnotationId;
    QVector<HistorySnapshot> m_undoStack;
    QVector<HistorySnapshot> m_redoStack;
    QVector<QRect> m_windowRects;
    std::optional<QRect> m_hoveredWindowRect;
    QPointF m_selectionClickStart;
};
