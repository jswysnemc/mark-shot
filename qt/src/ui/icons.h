#pragma once

#include "shot_window.h"

#include <QColor>
#include <QIcon>
#include <QString>

namespace markshot::ui {

// Human-readable label for a toolbar action (used in tooltips and the
// runtime tool-name display).
QString actionName(ShotWindow::Action action);

/**
 * 创建工具栏动作图标，使用深色截图面板默认配色
 * @param action 工具栏动作
 * @return 由统一 SVG 资源按需渲染的图标
 */
QIcon makeToolIcon(ShotWindow::Action action);

/**
 * 按主题颜色创建工具栏动作图标
 * @param action 工具栏动作
 * @param ink 常态颜色，无效时使用默认面板配色
 * @param checkedInk 选中态颜色，无效时沿用常态颜色
 * @return 由统一 SVG 资源按需渲染的图标
 */
QIcon makeToolIcon(ShotWindow::Action action, QColor ink, QColor checkedInk = QColor());

enum class PropertyIcon {
    StrokeWidth,
    Opacity,
    Color,
    TextBackground,
    CornerRadius,
    MagnifierScale,
    MagnifierShape,
    Font,
    EditText,
    ResetNumber,
};

/**
 * 创建属性面板控件图标
 * @param icon 属性图标类型
 * @param ink 图标颜色，无效时使用默认面板配色
 * @return 由统一 SVG 资源按需渲染的图标
 */
QIcon makePropertyIcon(PropertyIcon icon, QColor ink = QColor());

/**
 * 创建填充开关图标，图形本身表达填充状态
 * @param filled 是否为填充状态
 * @param ink 图标颜色，无效时使用默认面板配色
 * @return 由统一 SVG 资源按需渲染的图标
 */
QIcon makeFillIcon(bool filled, QColor ink = QColor());

// 形状标记工具/弹层图标。
QIcon makeMarkerShapeIcon(ShotWindow::MarkerShape shape, QColor ink = QColor(), bool filled = true);

}  // namespace markshot::ui
