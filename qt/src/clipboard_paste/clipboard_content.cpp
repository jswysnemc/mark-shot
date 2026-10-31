#include "clipboard_paste/clipboard_content.h"

#include "ui/i18n.h"

#include <QAbstractTextDocumentLayout>
#include <QMimeData>
#include <QPainter>
#include <QPixmap>
#include <QTextDocument>
#include <QTextOption>

#include <QtMath>

namespace markshot::clipboard {
namespace {

constexpr int kMaximumTextLength = 20000;
constexpr int kMaximumTextHeight = 4096;
constexpr qint64 kMaximumImageBytes = 64 * 1024 * 1024;

/// @brief 返回空图像并记录贴图失败原因
/// @param error 错误输出位置
/// @param message 可显示的错误信息
/// @return 空图像
QImage fail(QString *error, const QString &message)
{
    if (error) {
        *error = message;
    }
    return {};
}

}

QImage imageFromContent(const QMimeData &mime, const QFont &font, const QPalette &palette, QString *error)
{
    if (error) {
        error->clear();
    }
    // 1. 【剪贴板】【内容转换】优先保留原图像素、透明度和缩放信息
    const QVariant imageData = mime.imageData();
    QImage image = qvariant_cast<QImage>(imageData);
    if (image.isNull() && imageData.canConvert<QPixmap>()) {
        image = qvariant_cast<QPixmap>(imageData).toImage();
    }
    if (image.isNull()) {
        for (const QString &format : {QStringLiteral("image/png"), QStringLiteral("image/jpeg"),
                                      QStringLiteral("image/bmp")}) {
            if (!mime.hasFormat(format)) {
                continue;
            }
            const QByteArray bytes = mime.data(format);
            if (bytes.size() > kMaximumImageBytes) {
                return fail(error, MS_TR("Clipboard content is too large to pin."));
            }
            image = QImage::fromData(bytes);
            if (!image.isNull()) {
                break;
            }
        }
    }
    if (!image.isNull()) {
        return image.sizeInBytes() <= kMaximumImageBytes
            ? image : fail(error, MS_TR("Clipboard content is too large to pin."));
    }

    // 2. 【剪贴板】【文字排版】只处理纯文字，保留换行并限制内存与画布高度
    const QString text = mime.text();
    if (text.trimmed().isEmpty()) {
        return fail(error, MS_TR("Clipboard has no image or text to pin."));
    }
    if (text.size() > kMaximumTextLength) {
        return fail(error, MS_TR("Clipboard content is too large to pin."));
    }
    QTextDocument document;
    document.setDefaultFont(font);
    document.setDocumentMargin(12);
    QTextOption option;
    option.setWrapMode(QTextOption::WrapAtWordBoundaryOrAnywhere);
    document.setDefaultTextOption(option);
    document.setPlainText(text);
    document.setTextWidth(520);
    document.setTextWidth(qBound(120.0, document.idealWidth(), 520.0));
    const QSize size(qCeil(document.size().width()), qCeil(document.size().height()));
    if (size.height() > kMaximumTextHeight) {
        return fail(error, MS_TR("Clipboard content is too large to pin."));
    }
    image = QImage(size, QImage::Format_ARGB32_Premultiplied);
    image.fill(palette.color(QPalette::Base));
    QPainter painter(&image);
    QAbstractTextDocumentLayout::PaintContext context;
    context.palette = palette;
    document.documentLayout()->draw(&painter, context);
    return image;
}

}
