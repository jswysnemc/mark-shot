#include "clipboard_paste/clipboard_reader.h"

#include <QClipboard>
#include <QElapsedTimer>
#include <QGuiApplication>
#include <QProcess>
#include <QStandardPaths>

namespace markshot::clipboard {
namespace {

/// @brief 有限时限内读取 Wayland 剪贴板，禁止外部进程无限占用内存
/// @param program 已解析的 wl-paste 路径
/// @param arguments 命令参数，不通过 shell 拼接
/// @param limit 最大输出字节数
/// @return 命令成功时的输出，失败或超限时为空
QByteArray readWayland(const QString &program, const QStringList &arguments, qsizetype limit)
{
    QProcess process;
    process.setProcessChannelMode(QProcess::ForwardedErrorChannel);
    process.start(program, arguments, QIODevice::ReadOnly);
    QElapsedTimer timer;
    timer.start();
    QByteArray output;
    while (process.state() != QProcess::NotRunning && timer.elapsed() < 1500) {
        process.waitForReadyRead(50);
        output += process.readAllStandardOutput();
        if (output.size() > limit) {
            process.kill();
            process.waitForFinished(200);
            return {};
        }
    }
    if (process.state() != QProcess::NotRunning) {
        process.kill();
        process.waitForFinished(200);
        return {};
    }
    output += process.readAllStandardOutput();
    return process.exitStatus() == QProcess::NormalExit && process.exitCode() == 0 && output.size() <= limit
        ? output : QByteArray();
}

}

std::unique_ptr<QMimeData> readContent()
{
    auto result = std::make_unique<QMimeData>();
    // 1. 【剪贴板】【读取】先复制 Qt 已收到的数据，不保留剪贴板所有者的对象指针
    const QMimeData *source = QGuiApplication::clipboard()->mimeData();
    if (source) {
        if (source->hasImage()) {
            result->setImageData(source->imageData());
        }
        for (const QString &format : {QStringLiteral("image/png"), QStringLiteral("image/jpeg"),
                                      QStringLiteral("image/bmp")}) {
            if (source->hasFormat(format)) {
                result->setData(format, source->data(format));
            }
        }
        if (source->hasText()) {
            result->setText(source->text());
        }
    }
    if (!result->formats().isEmpty()
        || !QGuiApplication::platformName().startsWith(QStringLiteral("wayland"))) {
        return result;
    }
    // 2. 【剪贴板】【Wayland读取】独立快捷键进程未获得键盘焦点时使用 data-control 工具
    const QString program = QStandardPaths::findExecutable(QStringLiteral("wl-paste"));
    if (program.isEmpty()) {
        return result;
    }
    const QList<QByteArray> types = readWayland(program, {QStringLiteral("--list-types")}, 16384).split('\n');
    for (const QString &format : {QStringLiteral("image/png"), QStringLiteral("image/jpeg"),
                                  QStringLiteral("image/bmp"), QStringLiteral("text/plain;charset=utf-8"),
                                  QStringLiteral("text/plain"), QStringLiteral("UTF8_STRING")}) {
        if (!types.contains(format.toUtf8())) {
            continue;
        }
        const QByteArray bytes = readWayland(program,
            {QStringLiteral("--no-newline"), QStringLiteral("--type"), format}, 64 * 1024 * 1024);
        if (bytes.isEmpty()) {
            continue;
        }
        if (format.startsWith(QStringLiteral("image/"))) {
            result->setData(format, bytes);
        } else {
            result->setText(QString::fromUtf8(bytes));
        }
        break;
    }
    return result;
}

}
