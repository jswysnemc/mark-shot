#include "shot_window_module.h"

#include "text_annotation_layout.h"

using namespace markshot::shot;

/**
 * 计算文字标注在图像或窗口坐标中的内容边界。
 * @param annotation 需要测量的文字标注。
 * @param widgetCoordinates 是否返回窗口坐标。
 * @return 包含文字内容与背景内边距的边界矩形。
 */
QRectF ShotWindow::textContentRect(const Annotation &annotation, bool widgetCoordinates) const
{
    // 1. 换行判断固定在图像坐标完成，屏幕和导出只在字号缩放上不同
    const QPointF imageTopLeft = textAnnotationBaseRect(annotation).topLeft();
    const QPointF topLeft = widgetCoordinates ? imageToWidget(imageTopLeft) : imageTopLeft;
    return QRectF(topLeft, textAnnotationBoxSize(annotation, annotationSizeScale(widgetCoordinates)));
}
