#pragma once

#include <QHash>
#include <QObject>
#include <QString>

namespace markshot::notifications {

/**
 * 【通知】【录制完成】发送带操作按钮的录制完成通知。
 *
 * 通知提供打开所在目录的入口，需要在通知服务回调时把动作映射回具体文件，
 * 因此由长生命周期对象持有待处理通知的编号。
 */
class RecordingSavedNotifier final : public QObject {
    Q_OBJECT

public:
    /**
     * 读取通知器单例。
     * @return 通知器实例。
     */
    static RecordingSavedNotifier &instance();

    /**
     * 发送录制完成通知。
     * @param path 录制文件路径。
     * @return 通知发送成功时返回 true。
     */
    bool notifySaved(const QString &path);

private slots:
    /**
     * 处理通知服务回调的动作。
     * @param notificationId 通知编号。
     * @param actionKey 动作标识。
     * @return 无返回值。
     */
    void handleActionInvoked(uint notificationId, const QString &actionKey);

private:
    /**
     * 创建通知器。
     * @param parent 父对象。
     */
    explicit RecordingSavedNotifier(QObject *parent = nullptr);

    /**
     * 连接通知服务的动作回调。
     * @return 连接成功时返回 true。
     */
    bool ensureActionListener();

    /**
     * 在文件管理器中定位录制文件。
     * @param path 录制文件路径。
     * @return 无返回值。
     */
    void revealInFileManager(const QString &path);

    QHash<uint, QString> m_pendingPaths;
    bool m_listenerConnected = false;
};

}  // namespace markshot::notifications
