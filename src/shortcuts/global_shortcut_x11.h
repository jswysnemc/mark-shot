#pragma once

#include "shortcuts/global_shortcut_backend.h"

#include <QAbstractNativeEventFilter>
#include <QByteArray>
#include <QHash>
#include <QObject>

#include <cstdint>
#include <functional>

namespace markshot::shortcuts {

/**
 * X11 原生全局快捷键后端。
 *
 * xdg-desktop-portal 的 GlobalShortcuts 接口是为 Wayland 设计的，X11 会话下
 * 多数桌面环境（Cinnamon、Xfce、MATE 等）并不实现该接口，Portal 调用会直接
 * 报“没有该接口”。X11 上的通行做法是应用自己向 X 服务器抓取按键，该后端即
 * 通过 XCB 的 GrabKey 实现，并用原生事件过滤器接收按键事件。
 */
class X11GlobalShortcutBackend final : public QObject,
                                       public QAbstractNativeEventFilter,
                                       public GlobalShortcutBackend {
    Q_OBJECT

public:
    /// @brief 创建 X11 全局快捷键后端。
    /// @param parent Qt 父对象。
    explicit X11GlobalShortcutBackend(QObject *parent = nullptr);
    ~X11GlobalShortcutBackend() override;

    /**
     * 判断当前会话是否可用该后端。
     *
     * 要求 Qt 平台插件为 xcb，且能取到底层 xcb 连接。
     *
     * @return 可用返回 true，否则返回 false。
     */
    static bool isAvailable();

    bool registerShortcuts(const QList<Shortcut> &shortcuts) override;
    void unregisterShortcuts() override;
    QString errorString() const override;
    QString backendName() const override;

    bool nativeEventFilter(const QByteArray &eventType, void *message, qintptr *result) override;

private:
    /// @brief 已抓取的一个按键组合，注销时按此还原。
    struct GrabbedKey {
        std::uint8_t keycode = 0;
        std::uint16_t modifiers = 0;
    };

    /**
     * 向 X 服务器抓取一个按键组合。
     * @param keysym 主键对应的 X11 keysym 值。
     * @param modifiers X11 修饰键掩码。
     * @param shortcutId 快捷键标识，用于把按键事件路由回调。
     * @return 抓取成功返回 true，否则返回 false。
     */
    bool grabKey(std::uint32_t keysym, std::uint16_t modifiers, const QString &shortcutId);

    /**
     * 查询 keysym 对应的键码。
     * @param keysym X11 keysym 值。
     * @return 对应键码；查不到时返回 0。
     */
    std::uint8_t keycodeForKeysym(std::uint32_t keysym) const;

    /**
     * 安装原生事件过滤器，只在首次抓取成功时执行。
     * @return 无返回值。
     */
    void installEventFilter();

    QHash<quint32, QString> m_shortcutIdByBinding;
    QHash<QString, std::function<void()>> m_callbacks;
    QList<GrabbedKey> m_grabbedKeys;
    QString m_errorString;
    bool m_eventFilterInstalled = false;
};

}  // namespace markshot::shortcuts
