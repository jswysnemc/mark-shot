#pragma once

#include "shortcuts/global_shortcut_backend.h"

#include <QObject>
#include <QString>

#include <memory>

namespace markshot::shortcuts {

/**
 * 全局快捷键管理器，按运行环境选择合适的后端。
 *
 * 选择顺序：X11 会话优先使用原生抓键（Portal 的 GlobalShortcuts 接口是为
 * Wayland 设计的，X11 桌面普遍不实现），其余环境使用 xdg-desktop-portal。
 * 首选后端注册失败时自动回退到另一后端。
 */
class GlobalShortcutManager final : public QObject {
    Q_OBJECT

public:
    /// @brief 创建全局快捷键管理器。
    /// @param parent Qt 父对象。
    explicit GlobalShortcutManager(QObject *parent = nullptr);
    ~GlobalShortcutManager() override;

    /**
     * 判断当前环境是否存在任何可用的全局快捷键后端。
     * @return 有可用后端返回 true，否则返回 false。
     */
    static bool isAvailable();

    /**
     * 注册一组全局快捷键。
     * @param shortcuts 要注册的快捷键列表。
     * @return 注册成功返回 true，全部后端都失败返回 false。
     */
    bool registerShortcuts(const QList<Shortcut> &shortcuts);

    /**
     * 注销全部快捷键并释放后端资源。
     * @return 无返回值。
     */
    void unregisterShortcuts();

    /**
     * 返回最近一次注册失败的错误信息。
     * @return 错误文本；没有错误时为空字符串。
     */
    QString errorString() const;

    /**
     * 返回当前生效的后端名称。
     * @return 后端标识；尚未成功注册时为空字符串。
     */
    QString activeBackendName() const;

private:
    /**
     * 创建指定名称的后端实例。
     * @param name 后端标识，"x11" 或 "portal"。
     * @return 后端实例；该后端在当前环境不可用时返回 nullptr。
     */
    std::unique_ptr<GlobalShortcutBackend> createBackend(const QString &name);

    std::unique_ptr<GlobalShortcutBackend> m_backend;
    QString m_errorString;
};

}  // namespace markshot::shortcuts
