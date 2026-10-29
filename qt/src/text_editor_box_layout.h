#pragma once

#include <QPoint>
#include <QRect>
#include <QSize>

namespace markshot::shot {

/// 文字输入框四边到文字的距离，单位为窗口像素
struct TextEditorBoxInsets {
    int left = 0;
    int top = 0;
    int right = 0;
    int bottom = 0;
};

/**
 * 计算文字输入框的内边距：左上与提交后的背景内边距一致，右侧至少放得下行尾光标。
 * @param scale 标注尺寸在窗口中的缩放比例。
 * @param cursorWidth 编辑器光标宽度。
 * @return 输入框四边内边距。
 */
TextEditorBoxInsets textEditorBoxInsets(qreal scale, int cursorWidth);

/**
 * 计算新建文字时输入框左上角：从点击处开始，右侧或下方放不下最小输入框时向选区内回移。
 * @param click 点击位置，窗口坐标。
 * @param selection 截图选区，窗口坐标。
 * @param minimumBox 最小输入框尺寸。
 * @return 输入框左上角，窗口坐标。
 */
QPoint newTextEditorBoxTopLeft(QPoint click, const QRect &selection, const QSize &minimumBox);

}  // namespace markshot::shot
