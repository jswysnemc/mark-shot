#include "cli/recording_cli.h"

#include "ipc/single_instance_ipc.h"

#include <QJsonDocument>
#include <QTextStream>

namespace markshot::cli {
namespace {

/**
 * 输出单实例响应 JSON。
 * @param response 单实例响应。
 * @return 无返回值。
 */
void writeResponseJson(const markshot::ipc::SingleInstanceResponse &response)
{
    QTextStream stream(stdout);
    stream << QString::fromUtf8(QJsonDocument(markshot::ipc::responseToJsonObject(response))
                                    .toJson(QJsonDocument::Compact))
           << Qt::endl;
}

/**
 * 创建无运行实例时的录制状态响应。
 * @param message 响应消息。
 * @return 单实例响应。
 */
markshot::ipc::SingleInstanceResponse inactiveResponse(const QString &message)
{
    markshot::ipc::SingleInstanceResponse response;
    response.handled = true;
    response.message = message;
    return response;
}

}  // namespace

int printRecordingStatus()
{
    markshot::ipc::SingleInstanceCommand command;
    command.recordingStatus = true;

    markshot::ipc::SingleInstanceResponse response;
    QString error;
    if (!markshot::ipc::sendSingleInstanceCommand(command, &response, &error)) {
        writeResponseJson(inactiveResponse(QStringLiteral("no running instance")));
        return 0;
    }

    writeResponseJson(response);
    return 0;
}

int stopRecordingFromCommandLine()
{
    markshot::ipc::SingleInstanceCommand command;
    command.stopRecording = true;

    markshot::ipc::SingleInstanceResponse response;
    QString error;
    if (!markshot::ipc::sendSingleInstanceCommand(command, &response, &error)) {
        writeResponseJson(inactiveResponse(QStringLiteral("no active recording")));
        return 1;
    }

    writeResponseJson(response);
    return response.stopped ? 0 : 1;
}

}  // namespace markshot::cli
