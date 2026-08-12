#include "ui/application_icon.h"

#include "shot_window_marker_shapes.h"
#include "ui/icons.h"

#include <QCoreApplication>
#include <QDir>
#include <QDirIterator>
#include <QFile>
#include <QFileIconProvider>
#include <QFileInfo>
#include <QPixmap>
#include <QStandardPaths>
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

#if !defined(Q_OS_WIN)
/**
 * 判断应用图标是否安装在托盘宿主可见的标准图标目录中。
 *
 * StatusNotifierItem 宿主（gnome-shell 的 AppIndicator 扩展、plasmashell）
 * 运行在独立进程中，按 IconName 查找图标时只搜索它自己的标准 XDG 图标
 * 路径。本进程 QIcon 能解析主题图标（例如 Nix wrapper 注入的私有
 * XDG_DATA_DIRS、开发目录直接运行）并不代表宿主也能解析；那种情况下
 * 必须回退为直接传像素，否则托盘显示空白（issue #78）。
 *
 * @return 宿主大概率可按名解析时返回 true。
 */
bool themeIconVisibleToTrayHost()
{
    // 刻意不使用本进程的 XDG_DATA_DIRS：包装器（Nix wrapper 等）注入的私有
    // 路径恰恰是宿主看不到的。只认三个所有会话进程都会搜索的标准根目录。
    const QStringList dataRoots = {
        QStandardPaths::writableLocation(QStandardPaths::GenericDataLocation),
        QStringLiteral("/usr/local/share"),
        QStringLiteral("/usr/share"),
    };
    const QStringList iconFileNames = {
        QStringLiteral("%1.svg").arg(QLatin1String(kApplicationIconName)),
        QStringLiteral("%1.png").arg(QLatin1String(kApplicationIconName)),
    };
    for (const QString &root : dataRoots) {
        if (root.isEmpty()) {
            continue;
        }
        const QDir hicolor(root + QStringLiteral("/icons/hicolor"));
        if (!hicolor.exists()) {
            continue;
        }
        QDirIterator it(hicolor.absolutePath(), iconFileNames, QDir::Files,
                        QDirIterator::Subdirectories);
        if (it.hasNext()) {
            return true;
        }
    }
    for (const QString &fileName : iconFileNames) {
        if (QFile::exists(QStringLiteral("/usr/share/pixmaps/") + fileName)) {
            return true;
        }
    }
    return false;
}
#endif

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
    if (iconRendersPixels(themed) && themeIconVisibleToTrayHost()) {
        return QString::fromLatin1(kApplicationIconName);
    }
    return {};
#endif
}

QIcon applicationTrayPixmapIcon()
{
#if defined(Q_OS_WIN)
    return applicationIcon();
#else
    // 跳过主题查找：主题图标携带 QIcon::name()，Qt 的 StatusNotifierItem
    // 实现遇到非空名字只传名字不传像素，宿主解析不了名字就显示空白。
    if (const QIcon png = bundledPngIcon(); iconRendersPixels(png)) {
        return png;
    }
    if (const QIcon svg(QStringLiteral(":/icons/mark-shot.svg")); iconRendersPixels(svg)) {
        return svg;
    }
    if (const QIcon ico(QStringLiteral(":/icons/mark-shot.ico")); iconRendersPixels(ico)) {
        return ico;
    }
    return makeToolIcon(ShotWindow::Action::ToolSelect);
#endif
}

}  // namespace markshot::ui
