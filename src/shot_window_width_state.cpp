#include "shot_window_module.h"

using namespace markshot::shot;

/// @brief 返回当前工具使用的默认粗细或尺寸
/// @return 当前工具对应的粗细或尺寸数值
qreal ShotWindow::currentToolWidth() const
{
    switch (m_tool) {
    case Tool::Move:
    case Tool::Select:
        return m_strokeWidth;
    case Tool::Highlighter:
        return m_highlighterWidth;
    case Tool::Text:
        return m_textSize;
    case Tool::Number:
        return m_numberWidth;
    case Tool::Mosaic:
        return m_mosaicBlockSize;
    case Tool::Pen:
    case Tool::Line:
    case Tool::Arrow:
    case Tool::Rectangle:
    case Tool::Ellipse:
    case Tool::Magnifier:
    case Tool::Laser:
        return m_strokeWidth;
    }

    return m_strokeWidth;
}

/// @brief 返回当前工具的滚轮预览尺寸
/// @return 经过当前缩放比例换算后的预览尺寸
qreal ShotWindow::currentToolPreviewSize() const
{
    const qreal scale = annotationSizeScale(true);

    switch (m_tool) {
    case Tool::Move:
    case Tool::Select:
        return 8.0;
    case Tool::Pen:
    case Tool::Line:
    case Tool::Arrow:
    case Tool::Rectangle:
    case Tool::Ellipse:
    case Tool::Magnifier:
        return std::max<qreal>(1.5, currentToolWidth() * scale);
    case Tool::Highlighter:
        return std::max<qreal>(6.0, currentToolWidth() * scale);
    case Tool::Text:
        return std::max<qreal>(10.0, (19.0 + currentToolWidth()) * scale);
    case Tool::Number:
        return std::max<qreal>(26.0, (13.0 + currentToolWidth() * 1.35) * scale * 2.0);
    case Tool::Mosaic:
        return std::max<qreal>(2.0, currentToolWidth() * scale);
    case Tool::Laser:
        return std::max<qreal>(8.0, currentToolWidth() * scale);
    }

    return std::max<qreal>(1.5, currentToolWidth() * scale);
}

/// @brief 按滑条数值设置选中标注或当前工具的粗细
/// @param width 新粗细或尺寸
/// @return 发生实际改动时返回 true
bool ShotWindow::setSelectedAnnotationWidth(int width)
{
    return setSelectedAnnotationWidth(width, true);
}

/// @brief 设置选中标注或当前工具的粗细
/// @param width 新粗细或尺寸
/// @param captureHistory 是否在修改已有标注前写入撤销历史
/// @return 发生实际改动时返回 true
bool ShotWindow::setSelectedAnnotationWidth(int width, bool captureHistory)
{
    auto clampedWidthForTool = [](Tool tool, int requestedWidth) -> qreal {
        switch (tool) {
        case Tool::Mosaic:
            return std::clamp<qreal>(requestedWidth, kMinMosaicBlockSize, kMaxMosaicBlockSize);
        case Tool::Number:
            return std::clamp<qreal>(requestedWidth, kMinNumberWidth, kMaxNumberWidth);
        case Tool::Text:
            return std::clamp<qreal>(requestedWidth, 1.0, 1000.0);
        case Tool::Highlighter:
            return std::clamp<qreal>(requestedWidth, kMinStrokeWidth, kMaxHighlighterWidth);
        case Tool::Move:
        case Tool::Select:
        case Tool::Pen:
        case Tool::Line:
        case Tool::Rectangle:
        case Tool::Ellipse:
        case Tool::Arrow:
        case Tool::Magnifier:
        case Tool::Laser:
            return std::clamp<qreal>(requestedWidth, kMinStrokeWidth, kMaxStrokeWidth);
        }

        return std::clamp<qreal>(requestedWidth, kMinStrokeWidth, kMaxStrokeWidth);
    };
    auto syncDefaultWidthForTool = [this](Tool tool, qreal appliedWidth) {
        switch (tool) {
        case Tool::Highlighter:
            m_highlighterWidth = appliedWidth;
            break;
        case Tool::Mosaic:
            m_mosaicBlockSize = appliedWidth;
            break;
        case Tool::Text:
            m_textSize = appliedWidth;
            break;
        case Tool::Number:
            m_numberWidth = appliedWidth;
            break;
        case Tool::Move:
        case Tool::Select:
            break;
        case Tool::Pen:
        case Tool::Line:
        case Tool::Rectangle:
        case Tool::Ellipse:
        case Tool::Arrow:
        case Tool::Magnifier:
        case Tool::Laser:
            m_strokeWidth = appliedWidth;
            break;
        }
    };

    const QVector<int> selectedIds = selectedAnnotationIds();
    if (!selectedIds.isEmpty()) {
        bool changed = false;
        for (int id : selectedIds) {
            const Annotation *annotation = annotationById(id);
            if (annotation && !qFuzzyCompare(annotation->width, clampedWidthForTool(annotation->tool, width))) {
                changed = true;
                break;
            }
        }
        if (!changed) {
            return false;
        }
        if (captureHistory) {
            pushHistorySnapshot();
        }
        // 1. 先修改选中标注自身,使用每种工具对应的合法范围
        for (int id : selectedIds) {
            if (Annotation *annotation = annotationById(id)) {
                if (annotation->tool == Tool::Mosaic) {
                    annotation->width = clampedWidthForTool(annotation->tool, width);
                } else if (annotation->tool == Tool::Number) {
                    annotation->width = clampedWidthForTool(annotation->tool, width);
                } else if (annotation->tool == Tool::Text) {
                    const qreal oldWidth = annotation->width;
                    annotation->width = clampedWidthForTool(annotation->tool, width);
                    const qreal factor = ((19.0 + annotation->width) / (19.0 + oldWidth)) * 1.05;
                    annotation->rect.setWidth(annotation->rect.width() * factor);
                    annotation->rect = textContentRect(*annotation, false);
                    if (!annotation->points.isEmpty()) {
                        annotation->points[0] = annotation->rect.topLeft();
                    }
                } else {
                    annotation->width = clampedWidthForTool(annotation->tool, width);
                }
                // 2. 选中标注改粗细也同步对应工具默认值,保证滚轮与滑条持久化一致
                syncDefaultWidthForTool(annotation->tool, annotation->width);
            }
        }
    } else {
        const qreal newWidth = clampedWidthForTool(m_tool, width);
        if (qFuzzyCompare(currentToolWidth(), newWidth)) {
            return false;
        }
        switch (m_tool) {
        case Tool::Highlighter:
            m_highlighterWidth = newWidth;
            break;
        case Tool::Mosaic:
            m_mosaicBlockSize = newWidth;
            break;
        case Tool::Text:
            m_textSize = newWidth;
            break;
        case Tool::Number:
            m_numberWidth = newWidth;
            break;
        case Tool::Move:
        case Tool::Select:
            return false;
        case Tool::Pen:
        case Tool::Line:
        case Tool::Rectangle:
        case Tool::Ellipse:
        case Tool::Arrow:
        case Tool::Magnifier:
        case Tool::Laser:
            m_strokeWidth = newWidth;
            break;
        }
    }
    if (m_draft.has_value()) {
        m_draft->width = currentToolWidth();
    }
    updateAnnotationPropertyPanel();
    updateColorPalettePreview();
    update();
    persistAnnotationState();
    return true;
}
