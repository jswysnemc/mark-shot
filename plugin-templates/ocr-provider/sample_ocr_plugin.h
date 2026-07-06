#pragma once

#include "markshot/ocr_provider_plugin.h"

#include <QObject>

class SampleOcrPlugin final : public QObject, public markshot::plugin::OcrProviderPlugin {
    Q_OBJECT
    Q_PLUGIN_METADATA(IID MARK_SHOT_OCR_PROVIDER_PLUGIN_IID FILE "metadata.json")
    Q_INTERFACES(markshot::plugin::OcrProviderPlugin)

public:
    /**
     * 读取 provider 唯一标识。
     * @return 小写短标识，用户配置会引用该值。
     */
    QString providerId() const override;

    /**
     * 读取设置页展示名称。
     * @return 展示名称。
     */
    QString displayName() const override;

    /**
     * 检查 OCR provider 当前是否可用。
     * @param error 不可用时输出原因。
     * @return 可用时返回 true。
     */
    bool isAvailable(QString *error) const override;

    /**
     * 对输入图像执行 OCR 识别。
     * @param image 输入图像。
     * @param tokens 输出 OCR token。
     * @param error 失败时输出错误信息。
     * @return 识别流程成功时返回 true。
     */
    bool recognize(const QImage &image,
                   QVector<markshot::plugin::OcrToken> *tokens,
                   QString *error) override;
};
