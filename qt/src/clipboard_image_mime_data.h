#pragma once

#include <QByteArray>
#include <QMimeData>
#include <QStringList>

namespace markshot {

/**
 * 【剪贴板】【多格式图片】以 PNG 字节为唯一存储，同时提供 image/png 与 image/bmp。
 *
 * wl-copy 与 xclip 只能声明一种 MIME 类型，VirtualBox 共享剪贴板等只识别 BMP 的程序
 * 拿不到图片。本类由常驻进程交给 QClipboard 发布：PNG 原样返回，BMP 在被请求时才
 * 解码转换，不长期持有整幅像素缓冲。
 */
class ClipboardImageMimeData final : public QMimeData {
    Q_OBJECT

public:
    /**
     * 创建多格式图片剪贴板数据。
     * @param png 图片的 PNG 编码字节。
     */
    explicit ClipboardImageMimeData(QByteArray png);

    /**
     * 读取原始 PNG 字节，用于进程退出前转交给持久剪贴板进程。
     * @return PNG 编码字节。
     */
    QByteArray png() const;

    /**
     * 列出对外声明的 MIME 类型，PNG 在前以便多数程序优先选择无损格式。
     * @return MIME 类型列表。
     */
    QStringList formats() const override;

protected:
    /**
     * 按请求的 MIME 类型生成数据。
     * @param mimeType 请求的 MIME 类型。
     * @param preferredType 调用方期望的值类型。
     * @return PNG 或 BMP 字节，不支持的类型返回空值。
     */
    QVariant retrieveData(const QString &mimeType, QMetaType preferredType) const override;

private:
    QByteArray m_png;
};

/**
 * 【剪贴板】【多格式图片】把 PNG 转为 24 位 BMP。
 *
 * 截图本身不透明，去掉 Alpha 通道写出兼容性最好的 24 位 BMP。
 * @param png PNG 编码字节。
 * @return BMP 字节，解码或编码失败时返回空。
 */
QByteArray bmpFromPng(const QByteArray &png);

}  // namespace markshot
