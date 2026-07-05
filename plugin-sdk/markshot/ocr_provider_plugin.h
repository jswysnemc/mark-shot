#pragma once

#include <QImage>
#include <QRectF>
#include <QString>
#include <QVector>
#include <QtPlugin>

namespace markshot::plugin {

struct OcrToken {
    QString text;
    QRectF box;
    int line = 0;
    int index = 0;
    qreal confidence = 0.0;
};

/**
 * OCR 识别 provider 插件接口。
 *
 * 实现方以 Qt 插件形式编译为动态库，放入 mark-shot 插件搜索目录即可被发现。
 * recognize 在工作线程调用，实现必须线程安全且不得访问 GUI。
 */
class OcrProviderPlugin {
public:
    virtual ~OcrProviderPlugin() = default;

    /**
     * 读取 provider 唯一标识。
     * @return 稳定的小写短标识，如 "rapid-onnx"。
     */
    virtual QString providerId() const = 0;

    /**
     * 读取展示名称。
     * @return 用于设置页展示的名称。
     */
    virtual QString displayName() const = 0;

    /**
     * 判断 provider 当前是否可用（模型、依赖是否就绪）。
     * @param error 输出不可用原因。
     * @return 可用时返回 true。
     */
    virtual bool isAvailable(QString *error) const = 0;

    /**
     * 对图像执行文本识别。
     * @param image 输入图像。
     * @param tokens 输出识别 token，box 为图像像素坐标。
     * @param error 输出错误信息。
     * @return 识别成功时返回 true（无文本也算成功，tokens 为空）。
     */
    virtual bool recognize(const QImage &image, QVector<OcrToken> *tokens, QString *error) = 0;
};

}  // namespace markshot::plugin

#define MARK_SHOT_OCR_PROVIDER_PLUGIN_IID "dev.mark-shot.OcrProviderPlugin/1.0"

Q_DECLARE_INTERFACE(markshot::plugin::OcrProviderPlugin, MARK_SHOT_OCR_PROVIDER_PLUGIN_IID)
