#pragma once

#include <QKeySequence>
#include <QList>
#include <QString>

#include <functional>

namespace markshot::shortcuts {

/// @brief 一条全局快捷键的定义与触发回调。
struct Shortcut {
    /// @brief 应用内唯一标识，用于把激活事件路由回对应回调。
    QString id;
    /// @brief 展示给用户或桌面环境设置界面的描述文本。
    QString description;
    /// @brief 期望绑定的按键组合。
    QKeySequence sequence;
    /// @brief 快捷键触发时执行的回调。
    std::function<void()> callback;
    /// @brief 进程外触发时执行的命令行。桌面环境侧注册的后端（GNOME 的
    /// gsettings 自定义快捷键等）无法回调进程内函数，转而执行该命令并依赖
    /// 单实例转发把动作路由回运行中的实例。为空时此类后端跳过该快捷键。
    QString commandLine;
};

/**
 * 全局快捷键后端的抽象接口。
 *
 * 不同显示服务器需要完全不同的注册方式：Wayland 下只能经由
 * xdg-desktop-portal 的 GlobalShortcuts 接口，X11 下则由应用自己向 X 服务器
 * 抓取按键。该接口把两种实现统一起来，供上层按运行环境选用。
 */
class GlobalShortcutBackend {
public:
    virtual ~GlobalShortcutBackend() = default;

    /**
     * 注册一组全局快捷键。
     * @param shortcuts 要注册的快捷键列表。
     * @return 至少成功注册一条时返回 true，全部失败返回 false。
     */
    virtual bool registerShortcuts(const QList<Shortcut> &shortcuts) = 0;

    /**
     * 注销当前已注册的全部快捷键并释放相关资源。
     * @return 无返回值。
     */
    virtual void unregisterShortcuts() = 0;

    /**
     * 返回最近一次操作的错误信息。
     * @return 错误文本；没有错误时为空字符串。
     */
    virtual QString errorString() const = 0;

    /**
     * 返回后端名称，用于日志排查。
     * @return 后端标识，例如 "x11" 或 "portal"。
     */
    virtual QString backendName() const = 0;
};

}  // namespace markshot::shortcuts
