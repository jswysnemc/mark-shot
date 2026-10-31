#pragma once

#include <QRect>
#include <QSize>
#include <QVector>

namespace markshot::selection_translate {

/**
 * 计算目标语言弹层的位置：竖向工具栏放在按钮左右两侧，横向工具栏放在按钮上下两侧，都朝窗口内侧弹出。
 * @param anchor 翻译按钮在截图窗口中的矩形。
 * @param verticalToolbar 按钮所在工具栏是否竖向排列。
 * @param popupSize 弹层尺寸。
 * @param bounds 截图窗口矩形。
 * @return 弹层在截图窗口中的矩形，四周至少留出 8px。
 */
QRect languagePopupGeometry(const QRect &anchor, bool verticalToolbar, const QSize &popupSize, const QRect &bounds);

/**
 * 计算译文浮条的位置：先在译文区域上方、下方右对齐，被挡住时在这两行里横向平移避开障碍物，
 * 仍放不下时放进区域右上角内侧，内侧同样横向避开障碍物。
 * @param region 译文区域在截图窗口中的矩形。
 * @param barSize 浮条尺寸。
 * @param bounds 截图窗口矩形。
 * @param obstacles 需要避让的工具栏和面板矩形。
 * @return 浮条在截图窗口中的矩形，四周至少留出 8px。
 */
QRect resultBarGeometry(const QRect &region, const QSize &barSize, const QRect &bounds,
                        const QVector<QRect> &obstacles);

}  // namespace markshot::selection_translate
