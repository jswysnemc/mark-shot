#include "text_annotation_layout.h"

#include "shot_window_internal.h"

#include <QTextBlock>
#include <QTextDocument>
#include <QTextLayout>
#include <QTextOption>

#include <algorithm>
#include <cmath>

namespace markshot::shot {

namespace {

/// 单行文字收框时使用的足够大框宽，保证测量时不触发换行
constexpr qreal kUnwrappedTextBoxWidth = 100000.0;
/// 换行宽度下限，过窄的框不再继续压缩
constexpr qreal kMinTextWrapWidth = 16.0;
/// 自然宽度与可用宽度的比较容差，吸收浮点误差和收框取整
constexpr qreal kTextWrapTolerance = 0.5;

/**
 * 按文字标注样式填充测量用文档。
 * @param document 待填充的文档。
 * @param annotation 文字标注。
 * @param scale 字号缩放比例。
 * @return 无返回值。
 */
void prepareTextDocument(QTextDocument &document, const types::Annotation &annotation, qreal scale)
{
    QFont font = markshot::theme::textFont(qRound((19.0 + annotation.width) * scale),
                                           annotation.fontWeight,
                                           annotation.fontFamily);
    font.setItalic(annotation.textItalic);
    QTextOption option;
    option.setWrapMode(QTextOption::WrapAtWordBoundaryOrAnywhere);
    option.setAlignment(Qt::AlignLeft | Qt::AlignTop);
    document.setDocumentMargin(0.0);
    document.setDefaultFont(font);
    document.setDefaultTextOption(option);
    document.setPlainText(annotation.text);
    document.setTextWidth(-1);
}

/**
 * 计算图像坐标中扣除左右内边距后的换行宽度。
 * @param annotation 文字标注。
 * @return 文字可用宽度。
 */
qreal textWrapWidth(const types::Annotation &annotation)
{
    return textAnnotationBaseRect(annotation).width() - kTextBackgroundPaddingX * 2.0;
}

}  // namespace

QRectF textAnnotationBaseRect(const types::Annotation &annotation)
{
    return annotation.rect.isEmpty()
        ? QRectF(annotation.points.value(0), QSizeF(360.0, 140.0))
        : annotation.rect.normalized();
}

bool textAnnotationWraps(const types::Annotation &annotation)
{
    // 1. 统一在图像坐标按导出字号测量，框宽扣掉左右内边距才是文字可用宽度
    const qreal wrapWidth = textWrapWidth(annotation);
    if (wrapWidth < kMinTextWrapWidth) {
        return false;
    }
    QTextDocument document;
    prepareTextDocument(document, annotation, 1.0);
    return document.idealWidth() > wrapWidth + kTextWrapTolerance;
}

QSizeF textAnnotationBoxSize(const types::Annotation &annotation, qreal scale)
{
    QTextDocument document;
    prepareTextDocument(document, annotation, scale);

    // 1. 换行与否按图像坐标决定，屏幕只按比例换算换行宽度
    if (textAnnotationWraps(annotation)) {
        document.setTextWidth(std::max(kMinTextWrapWidth * scale, textWrapWidth(annotation) * scale));
    }

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

    // 2. 宽度只保留左右对称内边距，避免选区右侧多出一段空白
    const qreal boxWidth = std::max<qreal>(1.0, std::ceil(textWidth + kTextBackgroundPaddingX * 2.0 * scale));
    const qreal boxHeight = std::max<qreal>(1.0, std::ceil(textHeight + kTextBackgroundPaddingY * 2.0 * scale));
    return QSizeF(boxWidth, boxHeight);
}

void fitTextAnnotationRect(types::Annotation &annotation, std::optional<qreal> boxWidth)
{
    // 1. 先放入目标框宽，再按实际排版结果收紧宽高
    const QRectF base = textAnnotationBaseRect(annotation);
    annotation.rect = QRectF(base.topLeft(),
                             QSizeF(boxWidth.value_or(kUnwrappedTextBoxWidth), std::max<qreal>(1.0, base.height())));
    annotation.rect.setSize(textAnnotationBoxSize(annotation, 1.0));
}

std::optional<qreal> wrappedEditorBoxWidth(QTextDocument *document, qreal scale)
{
    if (!document || scale <= 0.0 || document->textWidth() <= 0.0) {
        return std::nullopt;
    }

    // 1. 读取尺寸以完成布局，任一段落排成多行即说明编辑器发生了换行
    if (document->size().isEmpty()) {
        return std::nullopt;
    }
    for (QTextBlock block = document->begin(); block.isValid(); block = block.next()) {
        const QTextLayout *layout = block.layout();
        if (layout && layout->lineCount() > 1) {
            // 2. 编辑器文字宽度不含背景内边距，换算回图像坐标后补上左右内边距
            return document->textWidth() / scale + kTextBackgroundPaddingX * 2.0;
        }
    }
    return std::nullopt;
}

}
