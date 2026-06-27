#pragma once

#include <QRectF>
#include <QSize>
#include <QString>
#include <QVector>

class QPainter;
class QPointF;

namespace markshot::startup_hint {

enum class PanelAnchor {
    BottomLeft,
    TopLeft,
};

enum class InputIcon {
    Keyboard,
    Mouse,
    Wheel,
};

struct ShortcutHintItem {
    QString key;
    QString label;
    InputIcon icon = InputIcon::Keyboard;
};

struct PanelLayout {
    QRectF panelRect;
    QVector<QRectF> keyRects;
    QVector<QRectF> labelRects;
};

/**
 * 计算快捷键提示面板布局。
 * @param items 需要展示的快捷键说明项。
 * @param viewportSize 当前截图窗口尺寸。
 * @param anchor 面板停靠位置。
 * @return 面板整体与每一行文本的绘制矩形。
 */
PanelLayout layoutPanel(const QVector<ShortcutHintItem> &items, QSize viewportSize, PanelAnchor anchor);

/**
 * 计算鼠标避让检测区域。
 * @param layout 当前提示面板布局。
 * @return 比面板更大的避让检测矩形。
 */
QRectF avoidanceRect(const PanelLayout &layout);

/**
 * 根据鼠标位置选择提示面板停靠位置。
 * @param pointer 当前鼠标位置。
 * @param items 需要展示的快捷键说明项。
 * @param viewportSize 当前截图窗口尺寸。
 * @return 鼠标接近左下提示面板时返回左上，否则返回左下。
 */
PanelAnchor preferredAnchor(QPointF pointer, const QVector<ShortcutHintItem> &items, QSize viewportSize);

/**
 * 绘制 PixPin 风格快捷键提示面板。
 * @param painter 当前绘图对象。
 * @param items 需要展示的快捷键说明项。
 * @param layout 面板布局。
 */
void drawPanel(QPainter &painter, const QVector<ShortcutHintItem> &items, const PanelLayout &layout);

}  // namespace markshot::startup_hint
