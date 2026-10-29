#pragma once

#include "shot_window_types.h"

#include <QRectF>
#include <QSizeF>

#include <optional>

class QTextDocument;

namespace markshot::shot {

/**
 * 取文字标注用于排版的基准框，缺少尺寸的旧标注退回默认输入框。
 * @param annotation 文字标注。
 * @return 图像坐标中的基准框。
 */
QRectF textAnnotationBaseRect(const types::Annotation &annotation);

/**
 * 在图像坐标中判断文字标注是否需要按框宽换行。
 * 屏幕显示与导出共用这个判断，避免两边因字号取整不同而换行不一致。
 * @param annotation 文字标注，框宽包含左右背景内边距。
 * @return 单行自然宽度放不进框宽时返回 true。
 */
bool textAnnotationWraps(const types::Annotation &annotation);

/**
 * 按指定缩放测量文字标注的背景框尺寸。
 * @param annotation 文字标注。
 * @param scale 字号与内边距的缩放比例，导出为 1。
 * @return 包含左右、上下内边距的背景框尺寸。
 */
QSizeF textAnnotationBoxSize(const types::Annotation &annotation, qreal scale);

/**
 * 按新的换行框宽重算文字标注的图像坐标框。
 * @param annotation 需要更新的文字标注。
 * @param boxWidth 包含内边距的换行框宽；为空时按单行自然宽度收框。
 * @return 无返回值。
 */
void fitTextAnnotationRect(types::Annotation &annotation, std::optional<qreal> boxWidth = std::nullopt);

/**
 * 读取内联编辑器实际使用的换行宽度，并换算成图像坐标框宽。
 * @param document 编辑器文档。
 * @param scale 屏幕相对图像的缩放比例。
 * @return 编辑器内有段落换行时返回包含内边距的框宽，否则返回空值。
 */
std::optional<qreal> wrappedEditorBoxWidth(QTextDocument *document, qreal scale);

}
