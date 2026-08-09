#pragma once

#include <QIcon>
#include <QString>

namespace markshot::ui {

/**
 * 解析应用图标，供窗口图标与系统托盘使用。
 *
 * Linux 下优先返回带主题名的图标：托盘走 StatusNotifierItem 协议时，
 * Qt 只有在 QIcon::name() 非空的情况下才会把图标名传给托盘宿主，
 * 宿主据此自行按主题查找并渲染；名字为空时宿主只能依赖像素数据，
 * 部分实现（GNOME 的 AppIndicator 扩展等）会因此显示空白。
 *
 * @return 应用图标；任何情况下都保证可以绘制出非空像素。
 */
QIcon applicationIcon();

/**
 * 返回应用图标对应的主题图标名。
 *
 * @return 图标主题中可解析到的名称；主题查找失败时返回空字符串。
 */
QString applicationIconThemeName();

}  // namespace markshot::ui
