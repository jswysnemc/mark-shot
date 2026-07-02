#pragma once

#include "recording/recording_options.h"
#include "recording/recording_status.h"

#include <QObject>
#include <QPointer>

namespace markshot::recording {

class RecordingController;

class RecordingSessionManager final : public QObject {
    Q_OBJECT

public:
    /**
     * 获取全局录制会话管理器。
     * @return 录制会话管理器实例。
     */
    static RecordingSessionManager &instance();

    /**
     * 启动录制会话。
     * @param options 录制配置。
     * @param parent 父对象。
     * @param error 输出错误信息。
     * @return 启动成功时返回 true。
     */
    bool start(const RecordingOptions &options, QObject *parent, QString *error);

    /**
     * 请求停止当前录制。
     * @param error 输出错误信息。
     * @return 已提交停止请求时返回 true。
     */
    bool stop(QString *error);

    /**
     * 读取当前录制状态。
     * @return 录制状态。
     */
    RecordingStatus status() const;

signals:
    void statusChanged();
    void recordingFinished(bool ok, QString outputPath, QString message);

private:
    explicit RecordingSessionManager(QObject *parent = nullptr);

    QPointer<RecordingController> m_controller;
};

}  // namespace markshot::recording
