#pragma once

#include <QHash>
#include <QKeySequence>
#include <QList>
#include <QObject>
#include <QSet>
#include <QString>
#include <QVariantMap>

#include <functional>

#include <QDBusObjectPath>

namespace markshot {

class GlobalShortcutPortal final : public QObject {
    Q_OBJECT

public:
    using Callback = std::function<void()>;

    struct Shortcut {
        QString id;
        QString description;
        QKeySequence sequence;
        Callback callback;
    };

    /// @brief 创建 Portal 全局快捷键后端。
    /// @param parent Qt 父对象。
    explicit GlobalShortcutPortal(QObject *parent = nullptr);
    ~GlobalShortcutPortal() override;

    /// @brief 判断当前会话是否提供 xdg-desktop-portal GlobalShortcuts 接口。
    /// @return 接口可用时返回 true，否则返回 false。
    static bool isAvailable();

    /// @brief 注册一组全局快捷键，并保存激活后的回调。
    /// @param shortcuts 要注册的快捷键列表。
    /// @return 注册成功返回 true，否则返回 false。
    bool registerShortcuts(const QList<Shortcut> &shortcuts);

    /// @brief 关闭当前 Portal 快捷键会话并清理回调。
    void unregisterShortcuts();

    /// @brief 返回最近一次注册或注销操作的错误信息。
    /// @return 错误文本；没有错误时为空字符串。
    QString errorString() const;

private slots:
    /// @brief 处理 Portal 发送的快捷键激活信号。
    /// @param sessionHandle 触发快捷键的 Portal 会话。
    /// @param shortcutId 应用注册时提供的快捷键标识。
    /// @param timestamp Portal 提供的触发时间戳。
    /// @param options Portal 附带的扩展参数。
    void handleActivated(QDBusObjectPath sessionHandle,
                         QString shortcutId,
                         qulonglong timestamp,
                         QVariantMap options);

private:
    /// @brief 创建 Portal 全局快捷键会话。
    /// @return 创建成功返回 true，否则返回 false。
    bool createSession();

    /// @brief 将快捷键动作绑定到当前 Portal 会话。
    /// @param shortcuts 要绑定的快捷键列表。
    /// @param boundShortcutIds Portal 明确返回的已绑定快捷键标识；无法解析返回值时保持为空。
    /// @return 绑定成功返回 true，否则返回 false。
    bool bindShortcuts(const QList<Shortcut> &shortcuts, QSet<QString> *boundShortcutIds);

    /// @brief 连接 Portal 快捷键激活信号。
    /// @return 连接成功返回 true，否则返回 false。
    bool connectActivationSignal();

    QDBusObjectPath m_sessionHandle;
    QHash<QString, Callback> m_callbacks;
    QString m_errorString;
    bool m_activationSignalConnected = false;
};

}  // namespace markshot
