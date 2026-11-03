#pragma once

#include "selection_aspect/selection_hud_colors.h"

#include <QColor>
#include <QFont>
#include <QRectF>

class QPainter;

namespace markshot::selection_aspect::hud_style {

// HUD 内容高度
inline constexpr int kHudHeight = 26;
// HUD 圆角半径
inline constexpr qreal kHudRadius = 7.0;
// HUD 左右内边距
inline constexpr int kHudPadding = 6;
// 图标绘制框边长
inline constexpr int kGlyphSize = 14;

/**
 * HUD 与比例弹层使用的字体，跟随界面字体族并保持紧凑字号。
 * @return 字体。
 */
QFont hudFont();

/**
 * 绘制 HUD 或弹层的圆角半透明背景与细边框。
 * @param painter 已开启抗锯齿的画笔。
 * @param rect 背景矩形。
 * @param colors 当前截图会话的 HUD 颜色。
 * @return 无返回值。
 */
void paintPanelBackground(QPainter &painter, const QRectF &rect, const HudColors &colors);

/**
 * 绘制锁形图标。
 * @param painter 画笔。
 * @param box 图标绘制框。
 * @param locked true 绘制闭合锁，false 绘制打开的锁。
 * @param color 线条颜色。
 * @return 无返回值。
 */
void paintLockGlyph(QPainter &painter, const QRectF &box, bool locked, const QColor &color);

/**
 * 绘制比例示意图：按宽高比缩放的矩形，自由比例绘制虚线方框。
 * @param painter 画笔。
 * @param box 图标绘制框。
 * @param ratio 宽高比，小于等于 0 表示自由比例。
 * @param color 线条颜色。
 * @return 无返回值。
 */
void paintRatioGlyph(QPainter &painter, const QRectF &box, qreal ratio, const QColor &color);

/**
 * 绘制向下的下拉箭头。
 * @param painter 画笔。
 * @param box 图标绘制框。
 * @param color 线条颜色。
 * @return 无返回值。
 */
void paintChevronGlyph(QPainter &painter, const QRectF &box, const QColor &color);

/**
 * 绘制选中标记。
 * @param painter 画笔。
 * @param box 图标绘制框。
 * @param color 线条颜色。
 * @return 无返回值。
 */
void paintCheckGlyph(QPainter &painter, const QRectF &box, const QColor &color);

}  // namespace markshot::selection_aspect::hud_style
