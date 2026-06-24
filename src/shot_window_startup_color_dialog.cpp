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

}  // namespace

using namespace markshot::shot;

void ShotWindow::showStartupColorDialog(QColor color, QPoint anchor)
{
    if (!color.isValid()) {
        return;
    }
    markshot::ui::rememberColor(color);

    if (m_startupColorPanel) {
        m_startupColorPanel->deleteLater();
        m_startupColorPanel = nullptr;
    }

    m_startupColorPanel = new QWidget(this);
    m_startupColorPanel->setObjectName(QStringLiteral("startupColorInspector"));
    m_startupColorPanel->setAttribute(Qt::WA_DeleteOnClose, true);
    m_startupColorPanel->setStyleSheet(QStringLiteral(
        "QWidget#startupColorInspector {"
        " background: rgba(229, 231, 235, 238);"
        " border: 1px solid rgba(15, 23, 42, 55);"
        " border-radius: 16px;"
        "}"
        "QLabel { color: #172033; font-size: 12px; }"
        "QLabel#formatName { font-weight: 700; color: #475569; min-width: 76px; }"
        "QLabel#formatValue {"
        " font-family: %1;"
        " font-weight: 700;"
        " color: #172033;"
        "}"
        "QFrame#formatRow {"
        " background: rgba(248, 250, 252, 228);"
        " border-radius: 9px;"
        "}"
        "QPushButton {"
        " background: rgba(148, 163, 184, 70);"
        " border: 0;"
        " border-radius: 7px;"
        " padding: 5px 9px;"
        " color: #334155;"
        " font-weight: 700;"
        "}"
        "QPushButton:hover { background: rgba(45, 212, 191, 150); color: #042F2E; }")
            .arg(markshot::theme::monospaceFontFamilyCss()));

    auto *layout = new QVBoxLayout(m_startupColorPanel);
    layout->setContentsMargins(12, 12, 12, 12);
    layout->setSpacing(9);

    const QString hex = colorHexRgb(color);
    auto *preview = new QLabel(hex, m_startupColorPanel);
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
    for (int index = 0; index < 8; ++index) {
        const bool hasColor = index < swatches.size();
        const QColor swatchColor = hasColor ? swatches.at(index) : QColor(0, 0, 0, 0);
        auto *button = new QPushButton(m_startupColorPanel);
        button->setFocusPolicy(Qt::NoFocus);
        button->setEnabled(hasColor && index > 0);
        button->setFixedSize(30, 24);
        button->setStyleSheet(QStringLiteral(
            "QPushButton {"
            " background: %1;"
            " border: 1px solid rgba(15, 23, 42, %2);"
            " border-radius: 7px;"
            " padding: 0;"
            "}"
            "QPushButton:hover { border-color: rgba(45, 212, 191, 210); }")
                .arg(hasColor ? colorHexRgba(swatchColor) : QStringLiteral("rgba(15,23,42,38)"))
                .arg(index == 0 ? 180 : 80));
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
        auto *frame = new QFrame(m_startupColorPanel);
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

    m_startupColorPanel->adjustSize();
    const QSize panelSize = m_startupColorPanel->sizeHint();
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
    m_startupColorPanel->setGeometry(QRect(QPoint(x, y), panelSize));
    m_startupColorPanel->show();
    m_startupColorPanel->raise();
}
