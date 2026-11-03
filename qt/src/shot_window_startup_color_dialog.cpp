#include "shot_window_module.h"

#include "ui/color_history_store.h"

namespace {

/// @brief 返回启动取色弹窗需要展示的当前色与历史色。
/// @param color 当前取色。
/// @return 首位为当前色，后续为历史色的颜色列表。
QVector<QColor> startupColorSwatches(const QColor &color)
{
    QVector<QColor> swatches;
    swatches.push_back(color);
    for (const QColor &historyColor : markshot::ui::readColorHistory()) {
        if (!historyColor.isValid() || historyColor.rgba() == color.rgba()) {
            continue;
        }
        swatches.push_back(historyColor);
        if (swatches.size() >= 8) {
            break;
        }
    }
    return swatches;
}

/// @brief 将颜色转换成 Qt stylesheet 稳定支持的 rgba() 文本。
/// @param color 颜色。
/// @return rgba(r,g,b,a) 样式文本。
QString stylesheetRgba(const QColor &color)
{
    return QStringLiteral("rgba(%1,%2,%3,%4)")
        .arg(color.red())
        .arg(color.green())
        .arg(color.blue())
        .arg(color.alpha());
}

/// @brief 【启动取色】【主题】生成取色面板样式表，面板、格式行与按钮跟随截图会话主题。
/// @param palette 截图会话的工具栏配色。
/// @return 取色面板样式表文本。
QString startupColorPanelStyleSheet(const markshot::ui::ToolbarPalette &palette)
{
    using markshot::ui::cssColor;
    return QStringLiteral(
               "QWidget#startupColorInspector {"
               " background: %1;"
               " border: 1px solid %2;"
               " border-radius: 16px;"
               "}"
               "QLabel { color: %3; font-size: 12px; }"
               "QLabel#formatName { font-weight: 700; color: %4; min-width: 76px; }"
               "QLabel#formatValue {"
               " font-family: %5;"
               " font-weight: 700;"
               " color: %3;"
               "}"
               "QFrame#formatRow {"
               " background: %6;"
               " border-radius: 9px;"
               "}"
               "QPushButton {"
               " background: %7;"
               " border: 0;"
               " border-radius: 7px;"
               " padding: 5px 9px;"
               " color: %8;"
               " font-weight: 700;"
               "}"
               "QPushButton:hover { background: %9; color: %10; }")
        .arg(cssColor(palette.popupFill), cssColor(palette.panelBorder), cssColor(palette.strongText),
             cssColor(palette.mutedText), markshot::theme::monospaceFontFamilyCss(),
             cssColor(palette.controlFill), cssColor(palette.controlHoverFill), cssColor(palette.text),
             cssColor(palette.hoverFill))
        .arg(cssColor(palette.hoverText));
}

}  // namespace

using namespace markshot::shot;

void ShotWindow::showStartupColorDialog(QColor color, QPoint anchor)
{
    if (!color.isValid()) {
        return;
    }
    markshot::ui::rememberColor(color);

    if (m_controls.m_startupColorPanel) {
        m_controls.m_startupColorPanel->deleteLater();
        m_controls.m_startupColorPanel = nullptr;
    }

    m_controls.m_startupColorPanel = new QWidget(this);
    m_controls.m_startupColorPanel->setObjectName(QStringLiteral("startupColorInspector"));
    m_controls.m_startupColorPanel->setAttribute(Qt::WA_DeleteOnClose, true);
    const markshot::ui::ToolbarPalette &palette = m_toolbarAppearance.palette;
    m_controls.m_startupColorPanel->setStyleSheet(startupColorPanelStyleSheet(palette));

    auto *layout = new QVBoxLayout(m_controls.m_startupColorPanel);
    layout->setContentsMargins(12, 12, 12, 12);
    layout->setSpacing(9);

    const QString hex = colorHexRgb(color);
    auto *preview = new QLabel(hex, m_controls.m_startupColorPanel);
    preview->setAlignment(Qt::AlignCenter);
    preview->setMinimumSize(300, 88);
    preview->setStyleSheet(QStringLiteral(
        "QLabel {"
        " background: %1;"
        " border-radius: 13px;"
        " color: %2;"
        " font-size: 20px;"
        " font-weight: 800;"
        "}").arg(hex, propertyIconInkForFill(color).name()));
    layout->addWidget(preview);

    auto *historyRow = new QHBoxLayout;
    historyRow->setContentsMargins(0, 0, 0, 0);
    historyRow->setSpacing(6);
    const QVector<QColor> swatches = startupColorSwatches(color);
    // 1. 色块描边取前景色，暗色主题下同样能看清深色色块的边界
    const auto swatchBorder = [&palette](int alpha) {
        QColor border = palette.strongText;
        border.setAlpha(alpha);
        return border;
    };
    for (int index = 0; index < 8; ++index) {
        const bool hasColor = index < swatches.size();
        const QColor swatchColor = hasColor ? swatches.at(index) : QColor(0, 0, 0, 0);
        auto *button = new QPushButton(m_controls.m_startupColorPanel);
        button->setFocusPolicy(Qt::NoFocus);
        button->setEnabled(hasColor && index > 0);
        button->setFixedSize(30, 24);
        button->setStyleSheet(QStringLiteral(
            "QPushButton, QPushButton:disabled {"
            " background: %1;"
            " border: 1px solid %2;"
            " border-radius: 7px;"
            " padding: 0;"
            "}"
            "QPushButton:hover { border-color: %3; }")
                .arg(hasColor ? stylesheetRgba(swatchColor) : stylesheetRgba(palette.controlFill),
                     stylesheetRgba(swatchBorder(index == 0 ? 180 : 80)),
                     stylesheetRgba(palette.focusBorder)));
        if (hasColor && index > 0) {
            connect(button, &QPushButton::clicked, this, [this, swatchColor, anchor] {
                showStartupColorDialog(swatchColor, anchor);
            });
        }
        historyRow->addWidget(button);
    }
    layout->addLayout(historyRow);

    struct FormatRow {
        QString name;
        QString value;
    };
    const QVector<FormatRow> rows = {
        {QStringLiteral("HEX"), hex},
        {QStringLiteral("hex lower"), hex.toLower()},
        {QStringLiteral("RGBA hex"), colorHexRgba(color)},
        {QStringLiteral("RGB"),
         QStringLiteral("rgb(%1, %2, %3)").arg(color.red()).arg(color.green()).arg(color.blue())},
        {QStringLiteral("RGBA"),
         QStringLiteral("rgba(%1, %2, %3, %4)")
             .arg(color.red()).arg(color.green()).arg(color.blue()).arg(alphaText(color))},
        {QStringLiteral("HSL"),
         QStringLiteral("hsl(%1, %2%, %3%)")
             .arg(colorHueOrZero(color.hslHue()))
             .arg(qRound(color.hslSaturationF() * 100.0))
             .arg(qRound(color.lightnessF() * 100.0))},
        {QStringLiteral("HSV"),
         QStringLiteral("hsv(%1, %2%, %3%)")
             .arg(colorHueOrZero(color.hsvHue()))
             .arg(qRound(color.hsvSaturationF() * 100.0))
             .arg(qRound(color.valueF() * 100.0))},
        {QStringLiteral("Qt"),
         QStringLiteral("Qt.rgba(%1, %2, %3, %4)")
             .arg(normalizedColorChannel(color.red()),
                  normalizedColorChannel(color.green()),
                  normalizedColorChannel(color.blue()),
                  alphaText(color))},
    };

    for (const FormatRow &row : rows) {
        auto *frame = new QFrame(m_controls.m_startupColorPanel);
        frame->setObjectName(QStringLiteral("formatRow"));
        auto *rowLayout = new QHBoxLayout(frame);
        rowLayout->setContentsMargins(10, 7, 8, 7);
        rowLayout->setSpacing(8);

        auto *nameLabel = new QLabel(row.name, frame);
        nameLabel->setObjectName(QStringLiteral("formatName"));
        rowLayout->addWidget(nameLabel);

        auto *valueLabel = new QLabel(row.value, frame);
        valueLabel->setObjectName(QStringLiteral("formatValue"));
        valueLabel->setTextInteractionFlags(Qt::TextSelectableByMouse);
        rowLayout->addWidget(valueLabel, 1);

        auto *copyButton = new QPushButton(MS_TR("Copy"), frame);
        copyButton->setFocusPolicy(Qt::NoFocus);
        connect(copyButton, &QPushButton::clicked, this, [this, value = row.value] {
            if (!markshot::copyTextToClipboard(value)) {
                return;
            }
            QTimer::singleShot(180, this, [this] {
                emit sessionCancelRequested();
                close();
            });
        });
        rowLayout->addWidget(copyButton);
        layout->addWidget(frame);
    }

    m_controls.m_startupColorPanel->adjustSize();
    const QSize panelSize = m_controls.m_startupColorPanel->sizeHint();
    int x = anchor.x() + 22;
    int y = anchor.y() + 22;
    if (x + panelSize.width() > width() - 12) {
        x = anchor.x() - panelSize.width() - 22;
    }
    if (y + panelSize.height() > height() - 12) {
        y = anchor.y() - panelSize.height() - 22;
    }
    x = std::clamp(x, 12, std::max(12, width() - panelSize.width() - 12));
    y = std::clamp(y, 12, std::max(12, height() - panelSize.height() - 12));
    m_controls.m_startupColorPanel->setGeometry(QRect(QPoint(x, y), panelSize));
    m_controls.m_startupColorPanel->show();
    m_controls.m_startupColorPanel->raise();
}
