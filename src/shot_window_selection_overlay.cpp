#include "shot_window_module.h"

#include "selection_loupe.h"

using namespace markshot::shot;

void ShotWindow::drawSelectionAdjustmentOverlay(QPainter &painter) const
{
    const bool adjusting = canAdjustSelection()
        && ((m_dragging && m_selectionDrag != SelectionDrag::None) || m_selectionKeyboardAdjusting);
    if ((m_mode != Mode::Selecting && !adjusting)
        || !m_startupHoverValid || m_frozenFrame.isNull()
        || m_startupTool == StartupTool::ColorPicker || m_startupTool == StartupTool::Ruler) {
        return;
    }

    // 1. 【截图】【选区放大镜】使用当前图像坐标生成预览和像素坐标说明
    const QPointF widgetPoint = imageToWidget(m_startupHoverImagePoint);
    if (m_selectionLoupeEnabled) {
        QVector<QRect> obstacles;
        for (const QWidget *panel : {m_toolbar, m_actionToolbar, m_openWithPanel, m_extensionPanel}) {
            if (panel && panel->isVisible()) {
                obstacles.append(panel->geometry());
            }
        }
        const SelectionLoupeLayout layout = selectionLoupeLayout(widgetPoint, size(),
            m_startupColorLoupeSize, m_startupHoverImagePoint, m_frozenFrame.size(), obstacles);
        const QString caption = QStringLiteral("%1, %2")
            .arg(qRound(m_startupHoverImagePoint.x())).arg(qRound(m_startupHoverImagePoint.y()));
        drawSelectionLoupe(painter, m_frozenFrame, layout, caption);
    }

    // 2. 【截图】【选区放大镜】系统拒绝移动指针时显示软件十字线
    if (m_selectionPointerDetached) {
        drawSelectionPointer(painter, widgetPoint);
    }
}

/**
 * 生成启动阶段快捷键提示项。
 * @return 当前选区状态下需要显示的提示项列表。
 */
QVector<markshot::startup_hint::ShortcutHintItem> ShotWindow::startupShortcutHintItems() const
{
    using markshot::startup_hint::InputIcon;

    if (m_mode != Mode::Selecting || hasUsableSelection()) {
        return {};
    }

    if (m_startupTool == StartupTool::CodeScanner) {
        return {
            {MS_TR("Drag"), MS_TR("Select code region"), InputIcon::Mouse},
            {MS_TR("Right/Esc"), MS_TR("Return to selection"), InputIcon::Mouse},
        };
    }

    if (recordingModeForStartupTool(m_startupTool).has_value()) {
        return {
            {MS_TR("Drag"), MS_TR("Select recording region"), InputIcon::Mouse},
            {MS_TR("Right/Esc"), MS_TR("Return to selection"), InputIcon::Mouse},
        };
    }

    if (m_startupTool != StartupTool::None) {
        return {};
    }

    auto shortcutTextOr = [](const QKeySequence &sequence, const QString &fallback) {
        const QString text = sequence.toString(QKeySequence::NativeText);
        return text.isEmpty() ? fallback : text;
    };

    QVector<markshot::startup_hint::ShortcutHintItem> items = {
        {MS_TR("Drag"), MS_TR("Select screenshot region"), InputIcon::Mouse},
    };
    if (m_selectionLoupeEnabled) {
        items.append({MS_TR("Arrow keys"), MS_TR("Nudge cursor"), InputIcon::Keyboard});
    }
    items.append({shortcutTextOr(m_startupColorPickerShortcut, QStringLiteral("C")), MS_TR("Pick color"), InputIcon::Keyboard});
    items.append({shortcutTextOr(m_startupRulerShortcut, QStringLiteral("R")), MS_TR("Measure size"), InputIcon::Keyboard});
    items.append({shortcutTextOr(m_startupCodeScannerShortcut, QStringLiteral("Q")), MS_TR("Scan QR or barcode"), InputIcon::Keyboard});
    items.append({shortcutTextOr(m_startupDisplayCaptureShortcut, QStringLiteral("D")), MS_TR("Quick display capture"), InputIcon::Keyboard});
    if (!activeRecordingAvailable()) {
        items.append({shortcutTextOr(m_startupGifRecorderShortcut, QStringLiteral("G")),
                      MS_TR("Record GIF"),
                      InputIcon::Keyboard});
        items.append({shortcutTextOr(m_startupVideoRecorderShortcut, QStringLiteral("V")),
                      MS_TR("Record video"),
                      InputIcon::Keyboard});
    }
    items.append({MS_TR("Middle"), MS_TR("Toggle fullscreen annotation"), InputIcon::Wheel});
    items.append({QStringLiteral("Ctrl+H"), MS_TR("Screenshot History"), InputIcon::Keyboard});
    items.append({MS_TR("Right/Esc"), MS_TR("Cancel"), InputIcon::Mouse});
    return items;
}

/**
 * 根据鼠标位置更新启动提示面板停靠位置。
 * @param pointer 当前鼠标位置。
 * @return 停靠位置发生变化时返回 true。
 */
bool ShotWindow::updateStartupShortcutHintAnchor(QPointF pointer)
{
    const QVector<markshot::startup_hint::ShortcutHintItem> items = startupShortcutHintItems();
    if (items.isEmpty()) {
        m_startupHintAnchor = markshot::startup_hint::PanelAnchor::BottomLeft;
        return false;
    }

    const markshot::startup_hint::PanelAnchor nextAnchor =
        markshot::startup_hint::preferredAnchor(pointer, items, size());
    if (nextAnchor == m_startupHintAnchor) {
        return false;
    }

    m_startupHintAnchor = nextAnchor;
    return true;
}

/**
 * 绘制启动阶段快捷键提示面板。
 * @param painter 当前绘图对象。
 */
void ShotWindow::drawStartupShortcutHint(QPainter &painter) const
{
    const QVector<markshot::startup_hint::ShortcutHintItem> items = startupShortcutHintItems();
    if (items.isEmpty()) {
        return;
    }

    const markshot::startup_hint::PanelLayout layout =
        markshot::startup_hint::layoutPanel(items, size(), m_startupHintAnchor);
    markshot::startup_hint::drawPanel(painter, items, layout);
}

/// @brief 绘制启动阶段的尺寸测量辅助层
/// @param painter 当前绘制器
/// @return 无返回值
void ShotWindow::drawStartupRuler(QPainter &painter) const
{
    if (!m_startupHoverValid && !m_startupRulerHasMeasure && !m_startupRulerDragging) {
        return;
    }

    painter.save();
    painter.setFont(markshot::theme::uiFont(10, QFont::DemiBold));
    painter.setPen(QPen(QColor(45, 212, 191, 150), 1.0, Qt::DashLine));
    auto clampFloatingRect = [this](QRectF rect) {
        if (rect.right() > width() - 8.0) {
            rect.moveRight(width() - 8.0);
        }
        if (rect.left() < 8.0) {
            rect.moveLeft(8.0);
        }
        if (rect.bottom() > height() - 8.0) {
            rect.moveBottom(height() - 8.0);
        }
        if (rect.top() < 8.0) {
            rect.moveTop(8.0);
        }
        return rect;
    };

    std::optional<QRectF> hoverLabelRect;
    if (m_startupHoverValid) {
        const QPointF hover = imageToWidget(m_startupHoverImagePoint);
        painter.drawLine(QPointF(m_frozenImageRect.left(), hover.y()), QPointF(m_frozenImageRect.right(), hover.y()));
        painter.drawLine(QPointF(hover.x(), m_frozenImageRect.top()), QPointF(hover.x(), m_frozenImageRect.bottom()));

        const QColor color = sampledImageColor(m_startupHoverImagePoint);
        const QString hoverText = QStringLiteral("x %1  y %2  %3")
                                      .arg(qRound(m_startupHoverImagePoint.x()))
                                      .arg(qRound(m_startupHoverImagePoint.y()))
                                      .arg(colorHexRgb(color));
        const QFontMetrics metrics(painter.font());
        QRectF hoverLabel(hover.x() + 14.0,
                          hover.y() + 14.0,
                          metrics.horizontalAdvance(hoverText) + 18.0,
                          metrics.height() + 10.0);
        if (hoverLabel.right() > width() - 8.0) {
            hoverLabel.moveRight(hover.x() - 14.0);
        }
        if (hoverLabel.bottom() > height() - 8.0) {
            hoverLabel.moveBottom(hover.y() - 14.0);
        }
        hoverLabel = clampFloatingRect(hoverLabel);
        hoverLabelRect = hoverLabel;
        drawRoundedLabel(painter, hoverLabel, hoverText, QColor(8, 13, 19, 225));
    }

    if (!m_startupRulerHasMeasure && !m_startupRulerDragging) {
        painter.restore();
        return;
    }

    const QRectF imageRect = normalizedRect(m_startupRulerStart, m_startupRulerEnd);
    if (imageRect.width() < 1.0 && imageRect.height() < 1.0) {
        painter.restore();
        return;
    }

    const QRectF widgetRect = imageRectToWidget(imageRect);
    painter.setPen(QPen(QColor(45, 212, 191), 2.0));
    painter.setBrush(QColor(45, 212, 191, 26));
    painter.drawRoundedRect(widgetRect, 3.0, 3.0);

    const int widthPx = qRound(imageRect.width());
    const int heightPx = qRound(imageRect.height());
    const qreal diagonal = std::hypot(imageRect.width(), imageRect.height());
    const QString info = QStringLiteral("%1 x %2 px   diag %3 px   area %4 px")
                             .arg(widthPx)
                             .arg(heightPx)
                             .arg(qRound(diagonal))
                             .arg(widthPx * heightPx);
    const QFontMetrics metrics(painter.font());
    const QSizeF infoSize(metrics.horizontalAdvance(info) + 20.0, metrics.height() + 10.0);
    constexpr qreal kRulerFloatingGap = 8.0;
    constexpr qreal kRulerMajorTickLength = 13.0;
    constexpr qreal kRulerMinorTickLength = 6.0;
    constexpr qreal kRulerTopLabelOffset = 30.0;
    constexpr qreal kRulerTopLabelWidth = 68.0;
    constexpr qreal kRulerTickLabelHeight = 16.0;
    constexpr qreal kRulerLeftLabelOffset = 62.0;
    constexpr qreal kRulerLeftLabelWidth = 48.0;
    constexpr qreal kRulerTopScaleGap = kRulerTopLabelOffset + kRulerFloatingGap;
    constexpr qreal kRulerBottomScaleGap = kRulerMajorTickLength + kRulerFloatingGap;
    constexpr qreal kRulerRightScaleGap = kRulerMajorTickLength + kRulerFloatingGap;
    constexpr qreal kRulerLeftScaleGap = kRulerLeftLabelOffset + kRulerFloatingGap;
    const QVector<QRectF> infoCandidates = {
        QRectF(QPointF(widgetRect.left(), widgetRect.bottom() + kRulerBottomScaleGap), infoSize),
        QRectF(QPointF(widgetRect.left(), widgetRect.top() - infoSize.height() - kRulerTopScaleGap), infoSize),
        QRectF(QPointF(widgetRect.right() + kRulerRightScaleGap, widgetRect.top()), infoSize),
        QRectF(QPointF(widgetRect.left() - infoSize.width() - kRulerLeftScaleGap, widgetRect.top()), infoSize),
    };
    const QRectF rulerScaleArea = widgetRect.adjusted(-kRulerLeftScaleGap,
                                                      -kRulerTopScaleGap,
                                                      kRulerRightScaleGap,
                                                      kRulerBottomScaleGap);
    auto overlapsInfoObstacle = [&](const QRectF &rect) {
        if (rect.intersects(rulerScaleArea)) {
            return true;
        }
        if (!hoverLabelRect.has_value()) {
            return false;
        }
        return rect.intersects(hoverLabelRect->adjusted(-kRulerFloatingGap,
                                                        -kRulerFloatingGap,
                                                        kRulerFloatingGap,
                                                        kRulerFloatingGap));
    };

    QRectF infoRect = clampFloatingRect(infoCandidates.first());
    for (const QRectF &candidate : infoCandidates) {
        const QRectF clamped = clampFloatingRect(candidate);
        if (!overlapsInfoObstacle(clamped)) {
            infoRect = clamped;
            break;
        }
    }
    drawRoundedLabel(painter, infoRect, info);

    const int stepX = rulerTickStep(imageRect.width());
    const int stepY = rulerTickStep(imageRect.height());
    const int minorX = std::max(1, stepX / 5);
    const int minorY = std::max(1, stepY / 5);
    painter.setPen(QPen(QColor(204, 251, 241, 230), 1.0));
    auto drawXTick = [&](int tick, bool major) {
        if (imageRect.width() <= 0.0) {
            return;
        }
        const qreal x = widgetRect.left() + tick * widgetRect.width() / imageRect.width();
        const qreal length = major ? kRulerMajorTickLength : kRulerMinorTickLength;
        painter.drawLine(QPointF(x, widgetRect.top()), QPointF(x, widgetRect.top() - length));
        painter.drawLine(QPointF(x, widgetRect.bottom()), QPointF(x, widgetRect.bottom() + length));
        if (major) {
            const QString text = QString::number(tick);
            painter.drawText(QRectF(x - kRulerTopLabelWidth / 2.0,
                                    widgetRect.top() - kRulerTopLabelOffset,
                                    kRulerTopLabelWidth,
                                    kRulerTickLabelHeight),
                             Qt::AlignCenter,
                             text);
        }
    };
    auto drawYTick = [&](int tick, bool major) {
        if (imageRect.height() <= 0.0) {
            return;
        }
        const qreal y = widgetRect.top() + tick * widgetRect.height() / imageRect.height();
        const qreal length = major ? kRulerMajorTickLength : kRulerMinorTickLength;
        painter.drawLine(QPointF(widgetRect.left(), y), QPointF(widgetRect.left() - length, y));
        painter.drawLine(QPointF(widgetRect.right(), y), QPointF(widgetRect.right() + length, y));
        if (major) {
            const QString text = QString::number(tick);
            painter.drawText(QRectF(widgetRect.left() - kRulerLeftLabelOffset,
                                    y - kRulerTickLabelHeight / 2.0,
                                    kRulerLeftLabelWidth,
                                    kRulerTickLabelHeight),
                             Qt::AlignRight | Qt::AlignVCenter,
                             text);
        }
    };

    for (int x = 0; x <= widthPx; x += minorX) {
        drawXTick(x, x % stepX == 0);
    }
    for (int y = 0; y <= heightPx; y += minorY) {
        drawYTick(y, y % stepY == 0);
    }

    painter.restore();
}

/// @brief 绘制当前启动辅助工具
/// @param painter 当前绘制器
/// @return 无返回值
void ShotWindow::drawStartupToolOverlay(QPainter &painter)
{
    if (m_startupTool == StartupTool::ColorPicker) {
        drawStartupColorLoupe(painter, m_startupHoverImagePoint);
    } else if (m_startupTool == StartupTool::Ruler) {
        drawStartupRuler(painter);
    }
}
