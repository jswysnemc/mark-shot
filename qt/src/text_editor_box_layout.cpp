#include "text_editor_box_layout.h"

#include "shot_window_internal.h"

#include <QAbstractTextDocumentLayout>
#include <QTextBlock>
#include <QTextDocument>
#include <QTextLayout>
#include <QtMath>

#include <algorithm>

namespace markshot::shot {

namespace {

/// 行尾光标与输入框右边缘之间至少保留的空隙
constexpr int kCaretGap = 2;

}  // namespace

TextEditorBoxInsets textEditorBoxInsets(qreal scale, int cursorWidth)
{
    // 1. 左上与提交后的背景内边距一致，输入时与提交后文字位置相同
    TextEditorBoxInsets insets;
    insets.left = qRound(kTextBackgroundPaddingX * scale);
    insets.top = qRound(kTextBackgroundPaddingY * scale);
    // 2. 右侧内边距同时承担光标位置，缩小显示时内边距不足就按光标宽度留出
    insets.right = std::max(insets.left, std::max(1, cursorWidth) + kCaretGap);
    insets.bottom = insets.top;
    return insets;
}

QPoint newTextEditorBoxTopLeft(QPoint click, const QRect &selection, const QSize &minimumBox)
{
    // 1. 选区右侧或下方放不下最小输入框时向内回移，但不越过选区左上边界
    const int right = selection.x() + selection.width();
    const int bottom = selection.y() + selection.height();
    const int x = std::max(selection.left(), std::min(click.x(), right - minimumBox.width()));
    const int y = std::max(selection.top(), std::min(click.y(), bottom - minimumBox.height()));
    return QPoint(x, y);
}

int textEditorLineHeight(const QTextDocument &document, int fontHeight)
{
    // 1. 读取文档尺寸以完成排版，首行高度与输入框实际高度来自同一套浮点度量
    if (QAbstractTextDocumentLayout *layout = document.documentLayout()) {
        layout->documentSize();
    }
    const QTextLayout *firstLine = document.begin().layout();
    if (!firstLine || firstLine->lineCount() == 0) {
        return fontHeight;
    }
    // 2. 加上上下文档边距后向上取整，与 QTextDocument::size() 的取整方式一致
    const qreal height = firstLine->lineAt(0).height() + document.documentMargin() * 2.0;
    return std::max(fontHeight, qCeil(height));
}

}  // namespace markshot::shot
