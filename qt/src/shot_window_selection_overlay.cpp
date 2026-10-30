#include "shot_window_module.h"

#include "selection_loupe.h"

using namespace markshot::shot;

void ShotWindow::drawSelectionAdjustmentOverlay(QPainter &painter) const
{
    const bool adjusting = canAdjustSelection()
        && ((m_dragging && m_selectionDrag != SelectionDrag::None) || m_selectionKeyboardAdjusting);
    if ((m_mode != Mode::Selecting && !adjusting)
        || !m_startupHoverValid || m_frozenFrame.isNull()
        || m_startupTool == StartupTool::ColorPicker) {
        return;
    }

    // 1. 【截图】【选区放大镜】使用当前图像坐标生成预览和像素坐标说明
    const QPointF widgetPoint = imageToWidget(m_startupHoverImagePoint);
    if (m_selectionLoupeEnabled) {
        QVector<QRect> obstacles;
        for (const QWidget *panel : {m_controls.m_toolbar, m_controls.m_actionToolbar, m_controls.m_openWithPanel, m_controls.m_extensionPanel}) {
            if (panel && panel->isVisible()) {
                obstacles.append(panel->geometry());
            }
        }
        const SelectionLoupeLayout layout = selectionLoupeLayout(widgetPoint, size(),
            m_startupColorLoupeSize, m_startupHoverImagePoint, m_frozenFrame.size(), obstacles);
        const QString caption = QStringLiteral("%1, %2")
            .arg(qRound(m_startupHoverImagePoint.x())).arg(qRound(m_startupHoverImagePoint.y()));
        drawSelectionLoupe(painter, m_frozenFrame, layout, caption);
    }
}

/**
 * 生成启动阶段快捷键提示项。
 * @return 当前选区状态下需要显示的提示项列表。
 */
QVector<markshot::startup_hint::ShortcutHintItem> ShotWindow::startupShortcutHintItems() const
{
    using markshot::startup_hint::InputIcon;

    if (m_mode != Mode::Selecting || hasUsableSelection()) {
        return {};
    }

    if (m_startupTool == StartupTool::CodeScanner) {
        return {
            {MS_TR("Drag"), MS_TR("Select code region"), InputIcon::Mouse},
            {MS_TR("Right/Esc"), MS_TR("Return to selection"), InputIcon::Mouse},
        };
    }

    if (recordingModeForStartupTool(m_startupTool).has_value()) {
        return {
            {MS_TR("Drag"), MS_TR("Select recording region"), InputIcon::Mouse},
            {MS_TR("Right/Esc"), MS_TR("Return to selection"), InputIcon::Mouse},
        };
    }

    if (m_startupTool != StartupTool::None) {
        return {};
    }

    auto shortcutTextOr = [](const QKeySequence &sequence, const QString &fallback) {
        const QString text = sequence.toString(QKeySequence::NativeText);
        return text.isEmpty() ? fallback : text;
    };

    QVector<markshot::startup_hint::ShortcutHintItem> items = {
        {MS_TR("Drag"), MS_TR("Select screenshot region"), InputIcon::Mouse},
    };
    if (m_selectionLoupeEnabled) {
        items.append({MS_TR("Arrow keys"), MS_TR("Nudge cursor"), InputIcon::Keyboard});
    }
    items.append({shortcutTextOr(m_startupColorPickerShortcut, QStringLiteral("C")), MS_TR("Pick color"), InputIcon::Keyboard});
    items.append({shortcutTextOr(m_startupRulerShortcut, QStringLiteral("R")), MS_TR("Measure size"), InputIcon::Keyboard});
    items.append({shortcutTextOr(m_startupCodeScannerShortcut, QStringLiteral("Q")), MS_TR("Scan QR or barcode"), InputIcon::Keyboard});
    items.append({shortcutTextOr(m_startupDisplayCaptureShortcut, QStringLiteral("D")), MS_TR("Quick display capture"), InputIcon::Keyboard});
    if (!activeRecordingAvailable()) {
        items.append({shortcutTextOr(m_startupGifRecorderShortcut, QStringLiteral("G")),
                      MS_TR("Record GIF"),
                      InputIcon::Keyboard});
        items.append({shortcutTextOr(m_startupVideoRecorderShortcut, QStringLiteral("V")),
                      MS_TR("Record video"),
                      InputIcon::Keyboard});
    }
    items.append({MS_TR("Middle"), MS_TR("Toggle fullscreen annotation"), InputIcon::Wheel});
    items.append({QStringLiteral("Ctrl+H"), MS_TR("Screenshot History"), InputIcon::Keyboard});
    items.append({MS_TR("Right/Esc"), MS_TR("Cancel"), InputIcon::Mouse});
    return items;
}

/**
 * 根据鼠标位置更新启动提示面板停靠位置。
 * @param pointer 当前鼠标位置。
 * @return 停靠位置发生变化时返回 true。
 */
bool ShotWindow::updateStartupShortcutHintAnchor(QPointF pointer)
{
    const QVector<markshot::startup_hint::ShortcutHintItem> items = startupShortcutHintItems();
    if (items.isEmpty()) {
        m_startupHintAnchor = markshot::startup_hint::PanelAnchor::BottomLeft;
        return false;
    }

    const markshot::startup_hint::PanelAnchor nextAnchor =
        markshot::startup_hint::preferredAnchor(pointer, items, size());
    if (nextAnchor == m_startupHintAnchor) {
        return false;
    }

    m_startupHintAnchor = nextAnchor;
    return true;
}

/**
 * 绘制启动阶段快捷键提示面板。
 * @param painter 当前绘图对象。
 */
void ShotWindow::drawStartupShortcutHint(QPainter &painter) const
{
    const QVector<markshot::startup_hint::ShortcutHintItem> items = startupShortcutHintItems();
    if (items.isEmpty()) {
        return;
    }

    const markshot::startup_hint::PanelLayout layout =
        markshot::startup_hint::layoutPanel(items, size(), m_startupHintAnchor);
    markshot::startup_hint::drawPanel(painter, items, layout);
}

/// @brief 绘制当前启动辅助工具
/// @param painter 当前绘制器
/// @return 无返回值
void ShotWindow::drawStartupToolOverlay(QPainter &painter)
{
    if (m_startupTool == StartupTool::ColorPicker) {
        drawStartupColorLoupe(painter, m_startupHoverImagePoint);
    }
}
