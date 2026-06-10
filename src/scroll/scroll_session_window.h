#pragma once

#include "scroll/stitcher.h"

#include <QImage>
#include <QPoint>
#include <QRect>
#include <QRegion>
#include <QSize>
#include <QString>
#include <QStringList>
#include <QVector>
#include <QWidget>

#include <cstdint>

class QLabel;
class QKeyEvent;
class QMouseEvent;
class QPainter;
class QPushButton;
class QScreen;
class QTimer;
class QWheelEvent;

namespace markshot::scroll {

struct ScrollSessionUiConfig {
    bool frameEnabled = true;
    bool hidePreviewDuringCapture = false;
    int frameGap = 5;
    int previewGap = 5;
};

// Fullscreen, click-through layer-shell overlay that drives a scrolling capture
// session. It periodically captures the selected screen region, stitches each
// frame into a growing tall image, and paints an outer frame plus a preview
// panel docked outside the region. The frame is separated from the capture
// region so screenshot backends do not record it while the user scrolls.
// The input mask keeps the preview panel interactive, so the user can keep
// scrolling the page underneath.
class ScrollSessionWindow final : public QWidget {
    /// @brief Qt meta-object declaration for this class.
    Q_OBJECT

public:
    ScrollSessionWindow(QRect globalGeometry,
                         QString outputName,
                         QScreen *screen = nullptr,
                         ScrollSessionUiConfig uiConfig = {},
                         QWidget *parent = nullptr);

    bool layerShellActive() const;

    // Configures this widget as a fullscreen layer-shell overlay on the given
    // screen. Must be called before show(). Returns false when layer-shell is
    // unavailable, in which case the caller should fall back to a plain window.
    bool configureLayerShell(QScreen *screen);

protected:
    bool eventFilter(QObject *watched, QEvent *event) override;
    void paintEvent(QPaintEvent *event) override;
    void closeEvent(QCloseEvent *event) override;
    void keyPressEvent(QKeyEvent *event) override;
    void mouseMoveEvent(QMouseEvent *event) override;
    void mousePressEvent(QMouseEvent *event) override;
    void mouseReleaseEvent(QMouseEvent *event) override;
    void resizeEvent(QResizeEvent *event) override;
    void showEvent(QShowEvent *event) override;
    void wheelEvent(QWheelEvent *event) override;

private slots:
    void handleGnomePreviewAction(const QString &sessionId, const QString &action);

private:
    void captureTick();
    void togglePause();
    void toggleAxis();
    void annotateResult();
    void saveResult();
    void copyResult();
    void buildControlBar();
    void layoutOverlay();
    void updateInputMask();
    void refreshControlLabels();
    void dumpDebugFrame(const QImage &frame, const char *tag);
    void setPreviewPanelVisible(bool visible);
    void updatePreviewPanelVisibility();
    void syncPreviewWindowVisibility();
    void scheduleScrollIdlePause();
    void cancelScrollIdlePause();
    void handleScrollIdleTimeout();
    void resumeAutoPausedCapture();
    bool shouldHidePreviewWhileCapturing() const;
    bool shouldAvoidPreviewOverlapForCapture() const;
    bool floatingDragHandleActive() const;
    QRect floatingDragHandleLocalRect() const;
    QRect floatingDragHandleGlobalRect() const;
    void layoutFloatingDragHandle();
    void drawFloatingDragHandle(QPainter &painter) const;
    void armAxisDrag(const QPoint &globalPos);
    bool updateAxisDrag(const QPoint &globalPos);
    bool finishAxisDrag();
    QImage currentResult() const;
    QRect captureBoundsGlobal() const;
    QRect floatingPanelGlobalRect() const;
    void updatePanelWindowGeometry();
    void setRegionGeometry(QRect geometry);
    QRegion overlayPaintRegion() const;
    bool gnomeShellPreviewActive() const;
    QSize gnomePreviewImageSize() const;
    QImage renderGnomePreviewImage(const QSize &size) const;
    void updateGnomeShellPreview(bool force = false);
    void hideGnomeShellPreview();
    void cleanupGnomePreviewFiles(int keepLatest = 0);

    // Geometry shared by preview interaction and painting so their notions of
    // "how much of the long image is visible" and "how far the view can travel"
    // never diverge (an off-by-one there makes the view jump when content is
    // prepended). All lengths are in stitched-image pixels along the scroll axis.
    struct PreviewLayout {
        bool valid = false;
        bool overflow = false;  // image exceeds the detail rect along the axis
        QRect detailRect;       // scrubbable window view
        QRect globalRect;       // whole-image thumbnail (empty unless overflow)
        qreal detailScale = 1.0;// source px -> preview px in the detail view
        int longLen = 0;        // long image extent along the scroll axis
        int viewportLen = 0;    // source px visible in the detail view
        int maxScrub = 0;       // longLen - viewportLen, clamped to >= 0
    };
    // The image preview rectangle, between the status row and the control bar.
    QRect imageAreaRect() const;
    // Derives the preview layout from the current result, axis, and panel rects.
    PreviewLayout computePreviewLayout() const;
    PreviewLayout computePreviewLayout(const QRect &area) const;
    void drawPreviewContent(QPainter &painter, const QRect &area) const;
    // Re-derives the preview range, then either follows the current captured
    // frame or shifts the manual view when content was prepended.
    void syncPreviewScroll(const StitchResult &outcome);
    QRect overviewTargetRect(const PreviewLayout &layout, const QImage &result) const;
    QRect overviewViewportRect(const QRect &target, const PreviewLayout &layout) const;
    int scrubPosFromOverviewPoint(const QPoint &point,
                                  const QRect &target,
                                  const PreviewLayout &layout,
                                  int markerOffsetPx) const;
    void setScrubPosition(int pos, bool followAtEnd);
    bool beginOverviewDrag(const QPoint &point);
    void updateOverviewDrag(const QPoint &point);

    // Maps the captured region (global compositor coordinates) into this
    // overlay's local coordinate space.
    QRect regionLocalRect() const;
    QRect frameOuterLocalRect() const;
    QRect previewAnchorLocalRect() const;
    QRect previewAnchorGlobalRect() const;
    QRegion framePaintRegion() const;
    bool previewPanelFitsAvailableSpace() const;
    // The preview panel rectangle in local coordinates.
    QRect previewPanelRect() const;

    QRect m_geometry;          // captured region, global coordinates
    QPoint m_screenOrigin;     // this overlay's top-left in global coordinates
    QString m_outputName;
    ScrollSessionUiConfig m_uiConfig;
    qint64 m_sessionId = 0;
    Stitcher m_stitcher;
    QTimer *m_timer = nullptr;
    QTimer *m_scrollIdleTimer = nullptr;
    bool m_paused = false;
    bool m_layerShell = false;
    bool m_panelOnlyWindow = false;
    QVector<std::uint8_t> m_lastSignature;
    QString m_statusText;
    int m_lastAppend = 0;       // pixels added by the most recent frame

    // Non-destructive preview position: the whole long image is always kept;
    // dragging the overview frame or using the wheel only moves which window of
    // it the detail view shows. While following, it tracks the current captured
    // frame until the user moves away from the live edge.
    bool m_following = true;
    int m_scrubPos = 0;         // top/left of the viewed window, stitched pixels
    int m_capturePos = 0;       // current screen selection top/left in stitched pixels
    int m_captureLen = 0;       // current screen selection extent along the scroll axis
    int m_debugFrameDumpCount = 0;

    // Axis/floating button drag-to-translate state. Dragging the handle moves
    // the entire capture region along the current scroll axis, allowing the user
    // to reach content beyond the normal scroll range.
    bool m_axisDragging = false;
    bool m_axisDragArmed = false;   // press recorded, waiting for threshold
    QPoint m_axisDragStartGlobal;
    QRect m_axisDragStartGeometry;
    QRegion m_transientPaintMask;
    bool m_restoreMaskAfterPaint = false;
    bool m_panelTransparentForCapture = false;
    bool m_previewPanelVisible = true;
    bool m_autoPausedForPreview = false;
    bool m_overviewDragging = false;
    int m_overviewDragOffsetPx = 0;
    bool m_gnomeShellPreview = false;
    bool m_gnomeShellOverlay = false;
    bool m_gnomePreviewVisible = false;
    QString m_gnomePreviewSessionId;
    QStringList m_gnomePreviewFiles;
    qint64 m_lastGnomePreviewUpdateMs = 0;
    int m_gnomePreviewSerial = 0;
    bool m_gnomePreviewImageDirty = true;

    QWidget *m_controlBar = nullptr;
    QPushButton *m_axisButton = nullptr;
    QPushButton *m_floatingAxisButton = nullptr;
    QPushButton *m_pauseButton = nullptr;
    QPushButton *m_annotateButton = nullptr;
    QPushButton *m_saveButton = nullptr;
    QPushButton *m_copyButton = nullptr;
    QPushButton *m_cancelButton = nullptr;
};

}  // namespace markshot::scroll
