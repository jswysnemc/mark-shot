#include "shot_window_module.h"

using namespace markshot::shot;

/**
 * 计算文字标注在图像或窗口坐标中的内容边界。
 * @param annotation 需要测量的文字标注。
 * @param widgetCoordinates 是否返回窗口坐标。
 * @return 包含文字内容与背景内边距的边界矩形。
 */
QRectF ShotWindow::textContentRect(const Annotation &annotation, bool widgetCoordinates) const
{
    const qreal scale = annotationSizeScale(widgetCoordinates);
    const QRectF baseRect = annotation.rect.isEmpty()
        ? QRectF(annotation.points.value(0), QSizeF(360.0, 140.0))
        : annotation.rect.normalized();
    const QPointF topLeft = widgetCoordinates ? imageToWidget(baseRect.topLeft()) : baseRect.topLeft();
    const qreal wrapWidth =
        std::max<qreal>(16.0, baseRect.width() * scale - kTextBackgroundPaddingX * 2.0 * scale);

    QFont font = markshot::theme::textFont(qRound((19.0 + annotation.width) * scale),
                                           annotation.fontWeight,
                                           annotation.fontFamily);
    font.setItalic(annotation.textItalic);
    QTextOption option;
    option.setWrapMode(QTextOption::WrapAtWordBoundaryOrAnywhere);
    option.setAlignment(Qt::AlignLeft | Qt::AlignTop);

    QTextDocument document;
    document.setDocumentMargin(0.0);
    document.setDefaultFont(font);
    document.setDefaultTextOption(option);
    document.setPlainText(annotation.text);
    document.setTextWidth(wrapWidth);

    qreal textWidth = 0.0;
    qreal textHeight = 0.0;
    for (QTextBlock block = document.begin(); block.isValid(); block = block.next()) {
        const QTextLayout *layout = block.layout();
        if (!layout) {
            continue;
        }
        for (int i = 0; i < layout->lineCount(); ++i) {
            const QTextLine line = layout->lineAt(i);
            textWidth = std::max(textWidth, line.naturalTextWidth());
            textHeight = std::max(textHeight, layout->position().y() + line.y() + line.height());
        }
    }
    if (textWidth <= 0.0 || textHeight <= 0.0) {
        const QSizeF documentSize = document.size();
        textWidth = documentSize.width();
        textHeight = documentSize.height();
    }

    const qreal rectWidth =
        std::max<qreal>(1.0, std::ceil(textWidth + kTextBackgroundPaddingX * 2.0 * scale) + 5.0);
    const qreal rectHeight =
        std::max<qreal>(1.0, std::ceil(textHeight + kTextBackgroundPaddingY * 2.0 * scale));
    return QRectF(topLeft, QSizeF(rectWidth, rectHeight));
}
