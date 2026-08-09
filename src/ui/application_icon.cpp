#include "ui/application_icon.h"

#include "shot_window_marker_shapes.h"
#include "ui/icons.h"

#include <QCoreApplication>
#include <QFileIconProvider>
#include <QFileInfo>
#include <QPixmap>
#include <QStringList>

namespace markshot::ui {
namespace {

/// @brief 图标主题与资源中使用的应用图标名。
constexpr char kApplicationIconName[] = "mark-shot";

/// @brief 内置 PNG 图标覆盖的尺寸，与 data/icons/hicolor 下的目录一一对应。
constexpr int kBundledIconSizes[] = {16, 22, 24, 32, 48, 64, 128, 256};

/**
 * 判断图标是否真的能绘制出像素。
 *
 * QIcon 非空只说明底层引擎存在，不代表能渲染：图标主题里只有 SVG 而运行环境
 * 缺少 Qt SVG 图像插件（Linux 上由独立的 qt6-svg / libqt6svg6 包提供）时，
 * 图标引擎能找到条目却渲染不出任何像素，托盘因此显示为空白。
 *
 * @param icon 待检查的图标。
 * @return 能渲染出非空像素返回 true，否则返回 false。
 */
bool iconRendersPixels(const QIcon &icon)
{
    if (icon.isNull()) {
        return false;
    }
    // 22 与 32 是托盘常用尺寸，任一能渲染即认为图标可用
    for (const int size : {22, 32}) {
        const QPixmap pixmap = icon.pixmap(size, size);
        if (!pixmap.isNull() && pixmap.width() > 0) {
            return true;
        }
    }
    return false;
}

/**
 * 从内置资源中的多尺寸 PNG 组装图标。
 *
 * PNG 由 qt6-base 自带的图像插件解码，不依赖 SVG 插件，因此在任何精简
 * 运行环境下都能绘制出来。
 *
 * @return 组装出的图标；资源缺失时返回空图标。
 */
QIcon bundledPngIcon()
{
    QIcon icon;
    for (const int size : kBundledIconSizes) {
        const QPixmap pixmap(QStringLiteral(":/icons/mark-shot-%1.png").arg(size));
        if (!pixmap.isNull()) {
            icon.addPixmap(pixmap);
        }
    }
    return icon;
}

/**
 * 解析 Linux 下的应用图标。
 *
 * 依次尝试：图标主题（保留主题名，托盘宿主可自行按名查找）、内置 PNG、
 * 内置 SVG、内置 ICO，最后回退到代码绘制的字形，确保托盘不会出现空图标。
 *
 * @return 应用图标。
 */
QIcon resolveLinuxApplicationIcon()
{
    // 1. 优先使用图标主题：只有这条路径能让 QIcon::name() 非空，
    //    托盘走 StatusNotifierItem 协议时宿主据此按名渲染
    const QIcon themed = QIcon::fromTheme(QLatin1String(kApplicationIconName));
    if (iconRendersPixels(themed)) {
        return themed;
    }

    // 2. 主题不可用或渲染失败时用内置 PNG，不依赖 SVG 插件
    if (const QIcon png = bundledPngIcon(); iconRendersPixels(png)) {
        return png;
    }

    // 3. 内置 SVG 与 ICO 作为次级回退
    if (const QIcon svg(QStringLiteral(":/icons/mark-shot.svg")); iconRendersPixels(svg)) {
        return svg;
    }
    if (const QIcon ico(QStringLiteral(":/icons/mark-shot.ico")); iconRendersPixels(ico)) {
        return ico;
    }

    // 4. 全部失败时用代码绘制的字形兜底
    return makeToolIcon(ShotWindow::Action::ToolSelect);
}

}  // namespace

QIcon applicationIcon()
{
#if defined(Q_OS_WIN)
    const QString applicationPath = QCoreApplication::applicationFilePath();
    if (!applicationPath.isEmpty()) {
        QFileIconProvider provider;
        const QIcon executableIcon = provider.icon(QFileInfo(applicationPath));
        if (!executableIcon.isNull()) {
            return executableIcon;
        }
    }
    const QIcon icon(QStringLiteral(":/icons/mark-shot.ico"));
    return icon.isNull() ? makeToolIcon(ShotWindow::Action::ToolSelect) : icon;
#else
    return resolveLinuxApplicationIcon();
#endif
}

QString applicationIconThemeName()
{
#if defined(Q_OS_WIN)
    return {};
#else
    const QIcon themed = QIcon::fromTheme(QLatin1String(kApplicationIconName));
    if (iconRendersPixels(themed)) {
        return QString::fromLatin1(kApplicationIconName);
    }
    return {};
#endif
}

}  // namespace markshot::ui
