/** 【外框对照】【原版采集】调用原版配置和图像导出函数，保存输入、输出 PNG 及实际物理尺寸 */
#include "export_image_effect.h"

#include <QCoreApplication>
#include <QDir>
#include <QFile>
#include <QImage>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <iostream>

/** 【外框对照】【原始像素】按 width、height 和 alpha 创建含透明度与边缘颜色的原图，返回预乘图像 */
static QImage source(int width, int height, bool alpha)
{
    QImage image(width, height, QImage::Format_ARGB32_Premultiplied);
    for (int y = 0; y < height; ++y) {
        for (int x = 0; x < width; ++x) {
            const int opacity = alpha ? (31 + x * 17 + y * 29) % 256 : 255;
            image.setPixelColor(x, y, QColor((x * 41 + y) % 256,
                (y * 59 + x) % 256, (x * 13 + y * 7) % 256, opacity));
        }
    }
    return image;
}

/** 【外框对照】【生产结果】按 argc、argv 中的目录采集实际配置与输出，返回进程状态，不改写参考项目 */
int main(int argc, char **argv)
{
    QCoreApplication app(argc, argv);
    if (argc != 2) {
        return 2;
    }
    QDir directory(QString::fromLocal8Bit(argv[1]));
    if (!directory.mkpath(QStringLiteral("."))) {
        return 3;
    }
    const QJsonArray values{
        QJsonValue(), false, true, 0, -2, " ON ", "unknown",
        QJsonObject{{"enabled", true}, {"padding", 7}, {"cornerRadius", 0}, {"shadowRadius", 0}},
        QJsonObject{{"enabled", true}, {"padding", 8}, {"cornerRadius", 4}, {"shadowRadius", 6}, {"shadowOffsetY", 3}, {"shadowOpacity", 0.45}},
        QJsonObject{{"show", "yes"}, {"margin", -12}, {"radius", "-7"}, {"shadowBlur", 999}, {"shadowYOffset", "-4"}, {"shadowAlpha", 2.5}},
        QJsonObject{{"enabled", QJsonValue()}, {"show", true}, {"padding", QJsonValue()}, {"margin", 5}, {"cornerRadius", "NaN"}, {"shadowOpacity", "inf"}, {"shadowRadius", 4.5}},
        QJsonObject{{"enabled", true}, {"padding", 24}, {"cornerRadius", 6}, {"shadowRadius", 18}, {"shadowOffsetY", 10}, {"shadowOpacity", 0.55}},
        QJsonObject{{"enabled", true}, {"padding", 0}, {"cornerRadius", 8}, {"shadowRadius", 10}, {"shadowOffsetY", 128}},
        QJsonObject{{"enabled", true}, {"padding", 256}, {"cornerRadius", 128}, {"shadowRadius", 128}, {"shadowOffsetY", 128}, {"shadowOpacity", 1}},
        QJsonObject{{"visible", true}, {"padding", "12"}, {"cornerRadius", "8.25"}, {"shadowRadius", "10"}, {"shadowOffsetY", "5"}, {"shadowOpacity", "0.4"}},
    };
    QJsonArray cases;
    int index = 0;
    // 1. 【外框对照】【配置矩阵】覆盖布尔、对象、错误类型、别名、范围和透明原图，输出来自原版函数
    for (const QJsonValue &value : values) {
        for (bool alpha : {false, true}) {
            QJsonObject root{{"export", QJsonObject{{"imageFrame", value}}}};
            const auto config = markshot::exportImageEffectConfigFromRoot(root);
            QImage input = source(36, 24, alpha);
            input.setDevicePixelRatio(alpha ? 1.25 : 2.0);
            const QImage output = markshot::applyExportImageEffect(input, config);
            const QString name = QStringLiteral("case-%1").arg(index++);
            if (output.isNull() || !input.save(directory.filePath(name + "-source.png"))
                || !output.save(directory.filePath(name + "-expected.png"))) {
                return 4;
            }
            cases.append(QJsonObject{{"name", name}, {"frame", value},
                {"size", QJsonArray{output.width(), output.height()}},
                {"dpr", output.devicePixelRatio()}});
        }
    }
    // 2. 【外框对照】【小图边界】验证极小图像的圆角内缩条件与阴影，不以常规尺寸推断边界
    for (QSize size : {QSize(1, 1), QSize(2, 3), QSize(3, 10), QSize(4, 4), QSize(5, 5), QSize(80, 48)}) {
        const QJsonValue value = values[8];
        QImage input = source(size.width(), size.height(), true);
        const auto config = markshot::exportImageEffectConfigFromRoot(
            QJsonObject{{"export", QJsonObject{{"imageFrame", value}}}});
        const QImage output = markshot::applyExportImageEffect(input, config);
        const QString name = QStringLiteral("case-%1").arg(index++);
        if (!input.save(directory.filePath(name + "-source.png"))
            || !output.save(directory.filePath(name + "-expected.png"))) {
            return 5;
        }
        cases.append(QJsonObject{{"name", name}, {"frame", value},
            {"size", QJsonArray{output.width(), output.height()}},
            {"dpr", output.devicePixelRatio()}});
    }
    QFile file(directory.filePath("cases.json"));
    if (!file.open(QIODevice::WriteOnly)) {
        return 6;
    }
    file.write(QJsonDocument(cases).toJson());
    // 3. 【外框对照】【通道舍入】采集全部合法预乘通道，PNG 保存使用本机 Qt 实际转换，独立于外框几何
    QImage channels(256, 256, QImage::Format_ARGB32_Premultiplied);
    for (int alpha = 0; alpha < 256; ++alpha) {
        for (int channel = 0; channel < 256; ++channel) {
            const int color = channel <= alpha ? channel : 0;
            channels.setPixel(channel, alpha, qRgba(color, color, color, alpha));
        }
    }
    if (!channels.save(directory.filePath("channel-rounding.png"))) {
        return 7;
    }
    std::cout << cases.size() << "\n";
    return 0;
}
