#pragma once

#include <QImage>
#include <QPointF>
#include <QString>
#include <QVector>
#include <QtPlugin>

namespace markshot::plugin {

struct CodeScanResult {
    QString format;
    QString text;
    QVector<QPointF> points;
};

/**
 * 二维码/条形码扫描 provider 插件接口。
 *
 * scan 在工作线程调用，实现必须线程安全且不得访问 GUI。
 */
class CodeScanProviderPlugin {
public:
    virtual ~CodeScanProviderPlugin() = default;

    /**
     * 读取 provider 唯一标识。
     * @return 稳定的小写短标识。
     */
    virtual QString providerId() const = 0;

    /**
     * 读取展示名称。
     * @return 用于设置页展示的名称。
     */
    virtual QString displayName() const = 0;

    /**
     * 判断 provider 当前是否可用。
     * @param error 输出不可用原因。
     * @return 可用时返回 true。
     */
    virtual bool isAvailable(QString *error) const = 0;

    /**
     * 扫描图像中的条码。
     * @param image 输入图像。
     * @param results 输出识别结果，points 为图像像素坐标。
     * @param error 输出错误信息。
     * @return 扫描成功时返回 true（无条码也算成功，results 为空）。
     */
    virtual bool scan(const QImage &image, QVector<CodeScanResult> *results, QString *error) = 0;
};

}  // namespace markshot::plugin

#define MARK_SHOT_CODE_SCAN_PROVIDER_PLUGIN_IID "dev.mark-shot.CodeScanProviderPlugin/1.0"

Q_DECLARE_INTERFACE(markshot::plugin::CodeScanProviderPlugin, MARK_SHOT_CODE_SCAN_PROVIDER_PLUGIN_IID)
