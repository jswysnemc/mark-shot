#pragma once

#include "markshot/code_scan_provider_plugin.h"

#include <QObject>

namespace markshot::code_scan_zxing {

/**
 * zxing-cpp 扫码 provider 插件。
 *
 * 插件在工作线程接收截图图像，直接调用 zxing-cpp 识别二维码与条形码。
 */
class ZxingCodeScanPlugin final : public QObject, public markshot::plugin::CodeScanProviderPlugin {
    Q_OBJECT
    Q_PLUGIN_METADATA(IID MARK_SHOT_CODE_SCAN_PROVIDER_PLUGIN_IID FILE "metadata.json")
    Q_INTERFACES(markshot::plugin::CodeScanProviderPlugin)

public:
    /**
     * 读取插件 provider 标识。
     * @return provider 标识，用于配置项选择插件。
     */
    QString providerId() const override;

    /**
     * 读取插件展示名称。
     * @return 面向用户显示的 provider 名称。
     */
    QString displayName() const override;

    /**
     * 检查 zxing-cpp 扫码插件是否可用。
     * @param error 输出错误信息。
     * @return 插件可用时返回 true。
     */
    bool isAvailable(QString *error) const override;

    /**
     * 识别图像中的二维码或条形码。
     * @param image 待识别图像。
     * @param results 输出扫码结果。
     * @param error 输出错误信息。
     * @return 识别流程成功时返回 true。
     */
    bool scan(const QImage &image, QVector<markshot::plugin::CodeScanResult> *results, QString *error) override;
};

}  // namespace markshot::code_scan_zxing
