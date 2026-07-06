#pragma once

#include "markshot/code_scan_provider_plugin.h"

#include <QObject>

class SampleCodeScanPlugin final : public QObject, public markshot::plugin::CodeScanProviderPlugin {
    Q_OBJECT
    Q_PLUGIN_METADATA(IID MARK_SHOT_CODE_SCAN_PROVIDER_PLUGIN_IID FILE "metadata.json")
    Q_INTERFACES(markshot::plugin::CodeScanProviderPlugin)

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
     * 检查扫码 provider 当前是否可用。
     * @param error 不可用时输出原因。
     * @return 可用时返回 true。
     */
    bool isAvailable(QString *error) const override;

    /**
     * 扫描输入图像中的二维码和条形码。
     * @param image 输入图像。
     * @param results 输出扫码结果。
     * @param error 失败时输出错误信息。
     * @return 扫描流程成功时返回 true。
     */
    bool scan(const QImage &image,
              QVector<markshot::plugin::CodeScanResult> *results,
              QString *error) override;
};
