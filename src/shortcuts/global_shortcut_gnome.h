#pragma once

#include "shortcuts/global_shortcut_backend.h"

#include <QString>
#include <QStringList>

namespace markshot::shortcuts {

/**
 * GNOME 桌面的全局快捷键后端。
 *
 * GNOME Wayland 的 xdg-desktop-portal（xdg-desktop-portal-gnome）尚未实现
 * GlobalShortcuts 接口，X11 抓键在 Wayland 会话中也不可用（issue #76）。该
 * 后端改用 GNOME 自身的快捷键机制：通过 gsettings 向
 * org.gnome.settings-daemon.plugins.media-keys 写入自定义快捷键，绑定到
 * mark-shot 的命令行入口（--capture 等）。按键由 GNOME 触发新进程，再经
 * 单实例转发把动作路由回正在运行的托盘实例。
 *
 * 注销时会从 gsettings 中移除本应用写入的全部条目，不触碰用户自己的
 * 自定义快捷键。
 */
class GnomeGlobalShortcutBackend final : public GlobalShortcutBackend {
public:
    ~GnomeGlobalShortcutBackend() override;

    /**
     * 判断该后端在当前环境是否可用。
     *
     * @return 当前桌面为 GNOME 且 gsettings 命令存在时返回 true。
     */
    static bool isAvailable();

    bool registerShortcuts(const QList<Shortcut> &shortcuts) override;
    void unregisterShortcuts() override;
    QString errorString() const override;
    QString backendName() const override;

private:
    /// @brief 本次注册写入 gsettings 的 keybinding 路径，注销时逐一清理。
    QStringList m_registeredPaths;
    QString m_errorString;
};

}  // namespace markshot::shortcuts
