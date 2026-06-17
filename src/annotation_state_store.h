#pragma once

#include "shot_window.h"

#include <QColor>
#include <QString>

namespace markshot {

/// @brief 截图标注的工具默认值快照,用于跨会话保留用户上次使用的样式
///
/// 该结构覆盖所有可被用户调节的工具默认值:通用描边/荧光笔/数字/文字/
/// 马赛克的笔宽或尺寸,
/// 当前颜色与不透明度,矩形圆角与风格,放大镜倍率与形状,箭头/荧光笔/数字风格,
/// 文本字体与文字背景色等。
struct AnnotationState {
    // 颜色与不透明度(alpha 体现在 currentColor.alpha 中)
    QColor currentColor = QColor(255, 77, 77);

    // 各类工具的笔宽/尺寸默认值
    qreal strokeWidth = 3.0;
    qreal highlighterWidth = 6.0;
    qreal numberWidth = 3.0;
    qreal textSize = 3.0;
    qreal mosaicBlockSize = 14.0;

    // 矩形相关
    bool shapeFilled = false;
    qreal rectangleCornerRadius = 0.0;
    ShotWindow::RectangleStyle rectangleStyle = ShotWindow::RectangleStyle::Stroke;

    // 放大镜相关
    qreal magnifierScale = 2.75;
    ShotWindow::MagnifierShape magnifierShape = ShotWindow::MagnifierShape::Circle;

    // 各类样式枚举
    ShotWindow::ArrowStyle arrowStyle = ShotWindow::ArrowStyle::Fletched;
    ShotWindow::HighlighterStyle highlighterStyle = ShotWindow::HighlighterStyle::StraightLine;
    ShotWindow::NumberStyle numberStyle = ShotWindow::NumberStyle::Arabic;

    // 文本相关
    QString textFontFamily;
    QColor textBackgroundColor = QColor(0, 0, 0, 0);
};

/// @brief 返回标注状态文件的绝对路径
/// @return 形如 <configDir>/annotation-state.json 的路径
QString annotationStateFilePath();

/// @brief 从磁盘载入标注状态
/// @return 文件存在且解析成功时返回完整状态;否则返回包含默认值的状态
AnnotationState loadAnnotationState();

/// @brief 把标注状态原子写入磁盘
/// @param state 待保存的状态
/// @return 写入成功时返回 true
bool saveAnnotationState(const AnnotationState &state);

}  // namespace markshot
