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
 * 仅当图标同时满足两个条件时返回名称：本进程能按主题渲染出像素，且图标
 * 安装在托盘宿主（gnome-shell、plasmashell 等独立进程）也能搜索到的标准
 * XDG 图标目录。宿主不可见时返回空字符串，调用方应回退为传输像素数据，
 * 避免 StatusNotifierItem 宿主按名查找失败显示空白（issue #78）。
 *
 * @return 托盘宿主可按名解析的名称；否则返回空字符串。
 */
QString applicationIconThemeName();

/**
 * 返回不携带主题名、只包含像素数据的托盘图标。
 *
 * Qt 的 StatusNotifierItem 实现只要 QIcon::name() 非空就只向宿主传图标名
 * 而不传像素。当主题名对宿主不可解析时，托盘必须使用本函数返回的纯像素
 * 图标，保证宿主始终有位图可渲染。
 *
 * @return 纯像素托盘图标；任何情况下都保证可以绘制出非空像素。
 */
QIcon applicationTrayPixmapIcon();

}  // namespace markshot::ui
