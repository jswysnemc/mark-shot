#pragma once

#include "recording/recording_status.h"

#include <QByteArray>
#include <QJsonObject>
#include <QLocalServer>
#include <QString>

#include <functional>
#include <memory>
#include <optional>

namespace markshot::ipc {

struct SingleInstanceCommand {
    bool capture = false;
    bool fullscreen = false;
    bool allOutputs = false;
    bool recordingStatus = false;
    bool stopRecording = false;
};

struct SingleInstanceResponse {
    bool handled = false;
    bool stopped = false;
    QString message;
    markshot::recording::RecordingStatus recording;
};

using SingleInstanceHandler = std::function<SingleInstanceResponse(const SingleInstanceCommand &)>;

/**
 * 返回单实例 IPC 服务名称。
 * @return 服务名称。
 */
QString singleInstanceServerName();

/**
 * 发送单实例命令并读取响应。
 * @param command 命令内容。
 * @param response 输出响应。
 * @param error 输出错误信息。
 * @return 收到响应时返回 true。
 */
bool sendSingleInstanceCommand(const SingleInstanceCommand &command,
                               SingleInstanceResponse *response,
                               QString *error);

/**
 * 创建单实例 IPC 服务。
 * @param error 输出错误信息。
 * @return 服务实例。
 */
std::unique_ptr<QLocalServer> listenForSingleInstanceCommands(QString *error);

/**
 * 给单实例 IPC 服务安装命令处理器。
 * @param server IPC 服务。
 * @param context 生命周期上下文。
 * @param handler 命令处理器。
 * @return 无返回值。
 */
void installSingleInstanceCommandHandler(QLocalServer *server,
                                         QObject *context,
                                         SingleInstanceHandler handler);

/**
 * 将响应转换为 JSON 对象。
 * @param response IPC 响应。
 * @return JSON 对象。
 */
QJsonObject responseToJsonObject(const SingleInstanceResponse &response);

}  // namespace markshot::ipc
