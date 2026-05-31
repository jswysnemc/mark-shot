#pragma once

#include "scroll/stitcher.h"

#include <QImage>
#include <QRect>
#include <QString>
#include <QVector>
#include <QWidget>

#include <cstdint>

class QLabel;
class QPushButton;
class QScreen;
class QTimer;

namespace markshot::scroll {

// Fullscreen, click-through layer-shell overlay that drives a scrolling capture
// session. It periodically captures the selected screen region, stitches each
// frame into a growing tall image, and paints two things on top of the live
// desktop: a blinking border drawn just outside the captured region (so grim
// never records it), and a preview panel docked to the region's right side that
// shows the full stitched image plus a marker for the current capture range.
// The input mask keeps only the border edges and the preview panel interactive,
// so the user can keep scrolling the page underneath.
class ScrollSessionWindow final : public QWidget {
    Q_OBJECT

public:
    ScrollSessionWindow(QRect globalGeometry,
                         QString outputName,
                         StitchAlgorithm algorithm,
                         QScreen *screen = nullptr,
                         QWidget *parent = nullptr);

    // Configures this widget as a fullscreen layer-shell overlay on the given
    // screen. Must be called before show(). Returns false when layer-shell is
    // unavailable, in which case the caller should fall back to a plain window.
    bool configureLayerShell(QScreen *screen);

protected:
    void paintEvent(QPaintEvent *event) override;
    void closeEvent(QCloseEvent *event) override;
    void resizeEvent(QResizeEvent *event) override;
    void showEvent(QShowEvent *event) override;

private:
    void captureTick();
    void togglePause();
    void annotateResult();
    void saveResult();
    void copyResult();
    void buildControlBar();
    void layoutOverlay();
    void updateInputMask();
    QImage currentResult() const;

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
    QTimer *m_blinkTimer = nullptr;
    bool m_blinkOn = true;
    bool m_paused = false;
    bool m_layerShell = false;
    QVector<std::uint8_t> m_lastSignature;
    QString m_statusText;
    int m_lastAppend = 0;       // pixels added by the most recent frame

    QWidget *m_controlBar = nullptr;
    QPushButton *m_pauseButton = nullptr;
    QPushButton *m_annotateButton = nullptr;
    QPushButton *m_saveButton = nullptr;
    QPushButton *m_copyButton = nullptr;
    QPushButton *m_cancelButton = nullptr;
};

}  // namespace markshot::scroll
