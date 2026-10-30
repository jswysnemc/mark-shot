#include "shot_window_ruler_tool.h"

#include "shot_window_module.h"

#include "ui/compact_combo_box.h"

using namespace markshot::shot;
using markshot::shot::types::Mode;
using markshot::shot::types::Tool;

void ShotWindowRulerTool::activate()
{
    ShotWindow &window = m_window;
    // 1. 再次选中已激活的尺子工具时切换测量方式，并用提示告知当前方式
    if (active()) {
        const RulerMode next = window.m_ruler.mode == RulerMode::Length ? RulerMode::Angle : RulerMode::Length;
        setMode(next);
        window.showToast(next == RulerMode::Angle ? MS_TR("Measure angle") : MS_TR("Measure length"), 1200);
        return;
    }
    window.setTool(Tool::Ruler);
}

void ShotWindowRulerTool::setMode(RulerMode mode)
{
    ShotWindow &window = m_window;
    // 1. 未完成的草稿按旧方式画到一半，换方式时撤去；已画好的尺子保持原样
    cancelDraft();
    if (window.m_ruler.mode == mode) {
        return;
    }
    window.m_ruler.mode = mode;
    window.updateAnnotationPropertyPanel();
    window.update();
}

void ShotWindowRulerTool::enterFromSelection()
{
    ShotWindow &window = m_window;
    // 1. 【截图】【尺子】退出选区阶段的工具状态，以整张截图作为标注范围
    window.leaveStartupTool();
    window.m_controls.m_selectionBeforeFullscreenAnnotation.reset();
    window.enterFullscreenAnnotation(true);

    // 2. 【截图】【尺子】选中尺子工具，并记住入口，Esc 回到选区阶段
    window.setTool(Tool::Ruler);
    window.m_ruler.enteredFromSelection = true;
    window.m_ruler.hoverValid = false;
    window.updateCursor();
}

bool ShotWindowRulerTool::leaveToSelection()
{
    ShotWindow &window = m_window;
    // 1. 先撤去未完成的草稿，再按一次 Esc 才离开
    if (cancelDraft()) {
        return true;
    }
    if (!window.m_ruler.enteredFromSelection || window.m_mode != Mode::Editing || !window.m_fullscreenAnnotation) {
        return false;
    }

    // 2. 【截图】【尺子】丢弃这次测量，与原来退出尺子工具时清空尺子一致
    window.commitTextEditor();
    window.m_annotations.clear();
    window.m_annotationHistory.clear();
    window.m_laserStrokes.clear();
    window.setSelectedAnnotations({});
    window.m_nextNumber = 1;
    window.m_nextAnnotationId = 1;

    // 3. 【截图】【尺子】回到选区阶段，工具恢复为默认工具，属性栏随之隐藏
    window.leaveFullscreenAnnotation();
    window.setTool(window.defaultEditingTool());
    window.updateCursor();
    return true;
}

void ShotWindowRulerTool::initializePanel(QHBoxLayout *layout)
{
    ShotWindow &window = m_window;
    auto *combo = new markshot::ui::CompactComboBox(window.m_controls.m_annotationPropertyPanel);
    combo->setObjectName(QStringLiteral("annotationRulerMode"));
    combo->setCursor(Qt::ArrowCursor);
    combo->setFocusPolicy(Qt::NoFocus);
    combo->addItem(MS_TR("Length"), static_cast<int>(RulerMode::Length));
    combo->addItem(MS_TR("Angle"), static_cast<int>(RulerMode::Angle));
    combo->setToolTip(MS_TR("Measure mode"));
    combo->setAccessibleName(MS_TR("Measure mode"));
    QObject::connect(combo, QOverload<int>::of(&QComboBox::activated), &window, [&window, combo](int index) {
        if (index >= 0) {
            ShotWindowRulerTool(window).setMode(static_cast<RulerMode>(combo->itemData(index).toInt()));
        }
    });
    layout->addWidget(combo);
    window.m_controls.m_propertyRulerModeCombo = combo;
}

void ShotWindowRulerTool::syncPanel(bool visible)
{
    QComboBox *combo = m_window.m_controls.m_propertyRulerModeCombo;
    if (!combo) {
        return;
    }
    combo->setVisible(visible);
    if (visible) {
        const QSignalBlocker blocker(combo);
        combo->setCurrentIndex(std::max(0, combo->findData(static_cast<int>(m_window.m_ruler.mode))));
    }
}
