#pragma once

#include "scroll/stitcher.h"

#include <QImage>
#include <QRect>
#include <QString>
#include <QVector>
#include <QWidget>

#include <cstdint>

class QLabel;
class QKeyEvent;
class QPushButton;
class QScreen;
class QSlider;
class QTimer;

namespace markshot::scroll {

// Fullscreen, click-through layer-shell overlay that drives a scrolling capture
// session. It periodically captures the selected screen region, stitches each
// frame into a growing tall image, and paints two things on top of the live
// desktop: a border drawn just outside the captured region (so grim
// never records it), and a preview panel docked to the region's right side that
// shows the full stitched image plus a marker for the current capture range.
// The input mask keeps only the border edges and the preview panel interactive,
// so the user can keep scrolling the page underneath.
class ScrollSessionWindow final : public QWidget {
    Q_OBJECT

public:
    ScrollSessionWindow(QRect globalGeometry,
                         QString outputName,
                         QScreen *screen = nullptr,
                         QWidget *parent = nullptr);

    // Configures this widget as a fullscreen layer-shell overlay on the given
    // screen. Must be called before show(). Returns false when layer-shell is
    // unavailable, in which case the caller should fall back to a plain window.
    bool configureLayerShell(QScreen *screen);

protected:
    void paintEvent(QPaintEvent *event) override;
    void closeEvent(QCloseEvent *event) override;
    void keyPressEvent(QKeyEvent *event) override;
    void resizeEvent(QResizeEvent *event) override;
    void showEvent(QShowEvent *event) override;

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
    QImage currentResult() const;

    // Geometry shared by the scrubber and the painter so their notions of "how
    // much of the long image is visible" and "how far the slider can travel"
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
    // The image preview rectangle, between the status row and the scrubber.
    QRect imageAreaRect() const;
    // Derives the preview layout from the current result, axis, and panel rects.
    PreviewLayout computePreviewLayout() const;
    // Re-derives the scrubber range, then either follows the current captured
    // frame or shifts the manual view when content was prepended.
    void syncScrubber(const StitchResult &outcome);

    // Maps the captured region (global compositor coordinates) into this
    // overlay's local coordinate space.
    QRect regionLocalRect() const;
    // The preview panel rectangle in local coordinates.
    QRect previewPanelRect() const;

    QRect m_geometry;          // captured region, global coordinates
    QPoint m_screenOrigin;     // this overlay's top-left in global coordinates
    QString m_outputName;
    Stitcher m_stitcher;
    QTimer *m_timer = nullptr;
    bool m_paused = false;
    bool m_layerShell = false;
    QVector<std::uint8_t> m_lastSignature;
    QString m_statusText;
    int m_lastAppend = 0;       // pixels added by the most recent frame

    // Non-destructive scrubber: the whole long image is always kept; the slider
    // only moves which window of it the detail view shows. While following, it
    // tracks the current captured frame; dragging it away stops following until
    // the user releases it back at the end.
    bool m_following = true;
    int m_scrubPos = 0;         // top/left of the viewed window, stitched pixels
    int m_capturePos = 0;       // current screen selection top/left in stitched pixels
    int m_captureLen = 0;       // current screen selection extent along the scroll axis

    QWidget *m_controlBar = nullptr;
    QPushButton *m_axisButton = nullptr;
    QPushButton *m_pauseButton = nullptr;
    QPushButton *m_annotateButton = nullptr;
    QPushButton *m_saveButton = nullptr;
    QPushButton *m_copyButton = nullptr;
    QPushButton *m_cancelButton = nullptr;
    QSlider *m_scrubber = nullptr;
};

}  // namespace markshot::scroll
