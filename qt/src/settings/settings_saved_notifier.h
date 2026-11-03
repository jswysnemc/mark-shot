#pragma once

#include <QObject>

namespace markshot::settings {

/**
 * 【设置】【保存通知】进程内广播设置已写入磁盘，供托盘全局快捷键等常驻组件重新读取配置。
 */
class SettingsSavedNotifier final : public QObject {
    Q_OBJECT

public:
    /**
     * 返回进程内唯一的通知对象。
     * @return 通知对象引用，生命周期与进程一致。
     */
    static SettingsSavedNotifier &instance();

    /**
     * 广播设置已成功保存。
     * @return 无返回值。
     */
    void notifySaved();

signals:
    /**
     * 设置窗口写入配置文件成功后发出。
     */
    void saved();

private:
    /**
     * 创建通知对象，仅供 instance 使用。
     * @param parent Qt 父对象。
     */
    explicit SettingsSavedNotifier(QObject *parent = nullptr);
};

}  // namespace markshot::settings
