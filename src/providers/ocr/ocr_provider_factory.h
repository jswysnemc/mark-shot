#pragma once

#include <QObject>
#include <QString>

namespace markshot::providers {

class ProviderTask;

struct OcrTaskRequest {
    QString imagePath;
    QString backend = QStringLiteral("auto");
    QString provider = QStringLiteral("auto");
    QString commandLine;
    QString helperProgram;
};

/**
 * 按优先链创建 OCR 任务。
 *
 * 优先链：自定义命令 > 显式 provider > auto（旧 venv 用户保持 helper >
 * 插件 > 内置 tesseract > helper 兜底）。
 * @param request OCR 请求。
 * @param parent 任务父对象。
 * @return 任务实例，调用方负责 start。
 */
ProviderTask *createOcrTask(const OcrTaskRequest &request, QObject *parent = nullptr);

/**
 * 解析当前配置实际会使用的 OCR provider 名称。
 * @param request OCR 请求（imagePath 可为空）。
 * @return provider 展示名，供设置页显示。
 */
QString resolvedOcrProviderName(const OcrTaskRequest &request);

/**
 * 判断旧版 Python OCR 运行环境是否存在。
 * @return ocr-venv 已配置时返回 true。
 */
bool legacyOcrHelperConfigured();

}  // namespace markshot::providers
