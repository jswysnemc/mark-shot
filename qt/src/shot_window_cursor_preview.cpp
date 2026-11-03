#include "shot_window_module.h"

#include "annotation_size_preview.h"
#include "ui/overlay_chip_colors.h"

using namespace markshot::shot;

void ShotWindow::startWheelPreview(QPointF widgetPoint, WheelPreview preview)
{
    m_wheelPreview = preview;
    m_wheelPreviewPosition = widgetPoint;
    m_wheelPreviewTimer.restart();
    if (!m_wheelPreviewHideTimer) {
        m_wheelPreviewHideTimer = new QTimer(this);
        m_wheelPreviewHideTimer->setSingleShot(true);
        m_wheelPreviewHideTimer->setTimerType(Qt::PreciseTimer);
        connect(m_wheelPreviewHideTimer, &QTimer::timeout, this, &ShotWindow::clearWheelPreview);
    }
    // 1. 【标注】【指针预览】由独立定时器结束预览，恢复光标不依赖下一次移动或重绘
    m_wheelPreviewHideTimer->start(900);
    updatePointerCursor(widgetPoint);
    update();
}

bool ShotWindow::wheelPreviewVisible() const
{
    return m_wheelPreview != WheelPreview::None && m_wheelPreviewTimer.isValid()
        && m_wheelPreviewTimer.elapsed() <= 900 && m_mode == Mode::Editing
        && !m_operationBusy && !m_dragging && !m_imagePanning && !m_toolbarDragging
        && m_frozenImageRect.contains(m_wheelPreviewPosition)
        && !mouseOverUiWidget(m_wheelPreviewPosition) && !propertyComboPopupVisible();
}

void ShotWindow::clearWheelPreview()
{
    if (m_wheelPreviewHideTimer) {
        m_wheelPreviewHideTimer->stop();
    }
    if (m_wheelPreview == WheelPreview::None) {
        return;
    }
    m_wheelPreview = WheelPreview::None;
    m_wheelPreviewTimer.invalidate();
    updateCursor();
    update();
}

void ShotWindow::drawWheelPreview(QPainter &painter)
{
    if (!wheelPreviewVisible()) {
        return;
    }
    painter.save();
    painter.setRenderHint(QPainter::Antialiasing, true);
    if (m_wheelPreview == WheelPreview::ImageZoom) {
        const QString zoomText = QStringLiteral("%1%").arg(qRound(m_imageZoom * 100.0));
        painter.setFont(markshot::theme::uiFont(11, QFont::DemiBold));
        const QFontMetrics metrics(painter.font());
        const QRectF textBounds = metrics.boundingRect(zoomText);
        QRectF bubble(m_wheelPreviewPosition + QPointF(14, 14),
                       QSizeF(textBounds.width() + 20, textBounds.height() + 10));
        bubble.moveLeft(std::clamp(bubble.left(), 8.0, std::max(8.0, width() - bubble.width() - 8.0)));
        bubble.moveTop(std::clamp(bubble.top(), 8.0, std::max(8.0, height() - bubble.height() - 8.0)));
        // 【截图】【缩放比例】气泡跟随截图会话主题，暗色保持原配色
        const markshot::ui::OverlayChipColors chip = markshot::ui::overlayChipColors(
            m_toolbarAppearance.palette,
            {QColor(17, 24, 39, 235), QColor(148, 163, 184, 80), QColor(229, 231, 235), {}, {}});
        painter.setPen(QPen(chip.border, 1));
        painter.setBrush(chip.fill);
        painter.drawRoundedRect(bubble, 6, 6);
        painter.setPen(chip.text);
        painter.drawText(bubble, Qt::AlignCenter, zoomText);
    } else {
        // 1. 【标注】【粗细预览】选择工具读取实际标注，避免使用固定尺寸或默认颜色
        const QVector<int> selectedIds = selectedAnnotationIds();
        const Annotation *selected = m_tool == Tool::Select && !selectedIds.isEmpty()
            ? annotationById(selectedIds.first()) : nullptr;
        const AnnotationSizePreview preview{
            selected ? selected->tool : m_tool,
            selected ? selected->width : currentToolWidth(),
            annotationSizeScale(true),
            selected ? selected->color : m_currentColor,
        };
        QVector<QRect> obstacles;
        const QWidget *panels[] = {m_controls.m_toolbar, m_controls.m_actionToolbar, m_controls.m_annotationPropertyPanel,
            m_controls.m_colorPalette, m_controls.m_propertyColorDialogPanel, m_controls.m_propertyFontPanel,
            m_controls.m_openWithPanel, m_controls.m_extensionPanel, m_textEditor, m_controls.m_shapeMarkerPopup};
        for (const QWidget *panel : panels) {
            if (panel && panel->isVisible()) {
                obstacles.append(panel->geometry());
            }
        }
        drawAnnotationSizePreview(painter, preview, m_wheelPreviewPosition, rect(), obstacles,
                                  m_toolbarAppearance.palette);
    }
    painter.restore();
}
