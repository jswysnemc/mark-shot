#include "recording/ui/recording_control_bar.h"

#include "recording/ui/recording_control_icons.h"
#include "ui/configured_toolbar_palette.h"
#include "ui/i18n.h"
#include "ui/theme.h"

#include <QHBoxLayout>
#include <QLabel>
#include <QPainter>
#include <QPen>
#include <QPaintEvent>
#include <QToolButton>

#include <algorithm>

namespace markshot::recording::ui {
namespace {

// 控制条视觉常量，保持紧凑不喧宾夺主
constexpr int kBarHeight = 40;
constexpr int kCornerRadius = 8;
constexpr int kButtonSize = 26;
constexpr int kIndicatorSize = 9;

/**
 * 格式化已录制时长。
 * @param elapsedMs 已录制毫秒数。
 * @return 时长文本，超过一小时带小时段。
 */
QString formatElapsed(qint64 elapsedMs)
{
    const qint64 totalSeconds = std::max<qint64>(0, elapsedMs / 1000);
    const qint64 hours = totalSeconds / 3600;
    const qint64 minutes = (totalSeconds % 3600) / 60;
    const qint64 seconds = totalSeconds % 60;
    if (hours > 0) {
        return QStringLiteral("%1:%2:%3")
            .arg(hours, 2, 10, QChar(QLatin1Char('0')))
            .arg(minutes, 2, 10, QChar(QLatin1Char('0')))
            .arg(seconds, 2, 10, QChar(QLatin1Char('0')));
    }
    return QStringLiteral("%1:%2")
        .arg(minutes, 2, 10, QChar(QLatin1Char('0')))
        .arg(seconds, 2, 10, QChar(QLatin1Char('0')));
}

/**
 * 生成状态指示点样式。
 * @param paused 是否处于暂停状态。
 * @return 样式表文本。
 */
QString indicatorStyleSheet(bool paused)
{
    // 录制中为红点，暂停时为琥珀色
    const QString color = paused ? QStringLiteral("#f59e0b") : QStringLiteral("#ef4444");
    return QStringLiteral("border-radius: %1px; background-color: %2;")
        .arg(kIndicatorSize / 2)
        .arg(color);
}

/**
 * 生成控制条按钮样式。
 * @param palette 控制条配色，暗色保持原有白色半透明悬停底。
 * @return 样式表文本。
 */
QString buttonStyleSheet(const markshot::ui::ToolbarPalette &palette)
{
    const QString hover = palette.dark ? QStringLiteral("rgba(255, 255, 255, 30)") : markshot::ui::cssColor(palette.controlHoverFill);
    const QString pressed = palette.dark ? QStringLiteral("rgba(255, 255, 255, 52)") : markshot::ui::cssColor(palette.controlBorder);
    return QStringLiteral(
               "QToolButton { border: none; border-radius: 7px; background-color: transparent; }"
               "QToolButton:hover { background-color: %1; }"
               "QToolButton:pressed { background-color: %2; }")
        .arg(hover, pressed);
}

}  // namespace

RecordingControlBar::RecordingControlBar(QWidget *parent)
    : QWidget(parent)
    , m_palette(markshot::ui::configuredToolbarPalette())
{
    setAttribute(Qt::WA_TranslucentBackground, true);
    setObjectName(QStringLiteral("recordingControlBar"));
    setFont(markshot::theme::uiFont(10));
    setFixedHeight(kBarHeight);
    buildLayout();
    refreshPauseButton();
}

void RecordingControlBar::buildLayout()
{
    auto *row = new QHBoxLayout(this);
    row->setContentsMargins(12, 0, 8, 0);
    row->setSpacing(8);

    m_indicator = new QLabel(this);
    m_indicator->setFixedSize(kIndicatorSize, kIndicatorSize);
    m_indicator->setStyleSheet(indicatorStyleSheet(false));
    row->addWidget(m_indicator);

    m_elapsed = new QLabel(formatElapsed(0), this);
    m_elapsed->setObjectName(QStringLiteral("recordingElapsed"));
    m_elapsed->setFont(markshot::theme::monospaceFont(11, QFont::DemiBold));
    // 1. 【录制】【控制条】文字跟随界面主题，暗色保持原有浅色文字
    m_elapsed->setStyleSheet(QStringLiteral("color: %1;").arg(
        m_palette.dark ? QStringLiteral("#f8fafc") : markshot::ui::cssColor(m_palette.strongText)));
    m_elapsed->setMinimumWidth(52);
    row->addWidget(m_elapsed);

    m_state = new QLabel(MS_TR("Recording"), this);
    m_state->setObjectName(QStringLiteral("recordingState"));
    m_state->setFont(markshot::theme::uiFont(9));
    m_state->setStyleSheet(QStringLiteral("color: %1;").arg(
        m_palette.dark ? QStringLiteral("#CBD5E1") : markshot::ui::cssColor(m_palette.mutedText)));
    row->addWidget(m_state);

    row->addSpacing(2);

    m_pause = new QToolButton(this);
    m_pause->setObjectName(QStringLiteral("recordingPause"));
    m_pause->setFixedSize(kButtonSize, kButtonSize);
    m_pause->setStyleSheet(buttonStyleSheet(m_palette));
    m_pause->setCursor(Qt::PointingHandCursor);
    connect(m_pause, &QToolButton::clicked, this, &RecordingControlBar::pauseToggleRequested);
    row->addWidget(m_pause);

    m_stop = new QToolButton(this);
    m_stop->setObjectName(QStringLiteral("recordingStop"));
    m_stop->setFixedHeight(kButtonSize);
    m_stop->setMinimumWidth(68);
    m_stop->setText(MS_TR("Stop"));
    m_stop->setToolButtonStyle(Qt::ToolButtonTextBesideIcon);
    m_stop->setAccessibleName(MS_TR("Stop Recording"));
    m_stop->setStyleSheet(buttonStyleSheet(m_palette) + QStringLiteral("QToolButton { color: %1; padding: 0 6px; }").arg(
        m_palette.dark ? QStringLiteral("#F8FAFC") : markshot::ui::cssColor(m_palette.strongText)));
    m_stop->setCursor(Qt::PointingHandCursor);
    // 2. 停止图标保持红色警示，亮色底上改用更深的红色
    m_stop->setIcon(makeRecordingStopIcon(m_palette.dark ? QColor(248, 113, 113) : QColor(220, 38, 38)));
    m_stop->setToolTip(MS_TR("Stop Recording"));
    connect(m_stop, &QToolButton::clicked, this, &RecordingControlBar::stopRequested);
    row->addWidget(m_stop);
}

void RecordingControlBar::updateStatus(qint64 elapsedMs, bool paused)
{
    if (m_elapsed) {
        m_elapsed->setText(formatElapsed(elapsedMs));
    }
    if (m_paused == paused) {
        return;
    }
    m_paused = paused;
    m_state->setText(m_paused ? MS_TR("Paused") : MS_TR("Recording"));
    if (m_indicator) {
        m_indicator->setStyleSheet(indicatorStyleSheet(m_paused));
    }
    refreshPauseButton();
}

void RecordingControlBar::refreshPauseButton()
{
    if (!m_pause) {
        return;
    }
    const QColor ink = m_palette.dark ? QColor(226, 232, 240) : m_palette.text;
    m_pause->setIcon(m_paused ? makeRecordingResumeIcon(ink) : makeRecordingPauseIcon(ink));
    m_pause->setToolTip(m_paused ? MS_TR("Resume Recording") : MS_TR("Pause Recording"));
    m_pause->setAccessibleName(m_pause->toolTip());
}

void RecordingControlBar::paintEvent(QPaintEvent *event)
{
    Q_UNUSED(event)
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing, true);
    // 1. 亮色底板加一条细边，压在浅色桌面上保持边界
    if (m_palette.dark) {
        painter.setPen(Qt::NoPen);
        painter.setBrush(QColor(15, 18, 26, 235));
        painter.drawRoundedRect(rect(), kCornerRadius, kCornerRadius);
        return;
    }
    painter.setPen(QPen(m_palette.panelBorder, 1.0));
    painter.setBrush(m_palette.panelFill);
    painter.drawRoundedRect(QRectF(rect()).adjusted(0.5, 0.5, -0.5, -0.5), kCornerRadius, kCornerRadius);
}

}  // namespace markshot::recording::ui
