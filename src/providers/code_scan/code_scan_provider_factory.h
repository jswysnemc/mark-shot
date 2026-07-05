#pragma once

#include <QObject>
#include <QString>

namespace markshot::providers {

class ProviderTask;

struct CodeScanTaskRequest {
    QString imagePath;
    QString provider = QStringLiteral("auto");
    QString commandLine;
    QString helperProgram;
};

/**
 * 按优先链创建扫码任务。
 *
 * 优先链：自定义命令 > 显式 provider > auto（插件 > 内置 zxing-cpp >
 * helper 兜底）。
 * @param request 扫码请求。
 * @param parent 任务父对象。
 * @return 任务实例，调用方负责 start。
 */
ProviderTask *createCodeScanTask(const CodeScanTaskRequest &request, QObject *parent = nullptr);

/**
 * 解析当前配置实际会使用的扫码 provider 名称。
 * @param request 扫码请求（imagePath 可为空）。
 * @return provider 展示名，供设置页显示。
 */
QString resolvedCodeScanProviderName(const CodeScanTaskRequest &request);

}  // namespace markshot::providers
