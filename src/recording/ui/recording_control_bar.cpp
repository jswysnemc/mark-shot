#include "recording/ui/recording_control_bar.h"

#include "recording/ui/recording_control_icons.h"
#include "ui/i18n.h"
#include "ui/theme.h"

#include <QHBoxLayout>
#include <QLabel>
#include <QPainter>
#include <QPaintEvent>
#include <QToolButton>

#include <algorithm>

namespace markshot::recording::ui {
namespace {

// 控制条视觉常量，保持紧凑不喧宾夺主
constexpr int kBarHeight = 36;
constexpr int kCornerRadius = 10;
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
 * @return 样式表文本。
 */
QString buttonStyleSheet()
{
    return QStringLiteral(
               "QToolButton { border: none; border-radius: 7px; background-color: transparent; }"
               "QToolButton:hover { background-color: rgba(255, 255, 255, 30); }"
               "QToolButton:pressed { background-color: rgba(255, 255, 255, 52); }");
}

}  // namespace

RecordingControlBar::RecordingControlBar(QWidget *parent)
    : QWidget(parent)
{
    setAttribute(Qt::WA_TranslucentBackground, true);
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
    m_elapsed->setFont(markshot::theme::monospaceFont(11, QFont::DemiBold));
    m_elapsed->setStyleSheet(QStringLiteral("color: #f8fafc;"));
    m_elapsed->setMinimumWidth(52);
    row->addWidget(m_elapsed);

    row->addSpacing(2);

    m_pause = new QToolButton(this);
    m_pause->setFixedSize(kButtonSize, kButtonSize);
    m_pause->setStyleSheet(buttonStyleSheet());
    m_pause->setCursor(Qt::PointingHandCursor);
    connect(m_pause, &QToolButton::clicked, this, &RecordingControlBar::pauseToggleRequested);
    row->addWidget(m_pause);

    m_stop = new QToolButton(this);
    m_stop->setFixedSize(kButtonSize, kButtonSize);
    m_stop->setStyleSheet(buttonStyleSheet());
    m_stop->setCursor(Qt::PointingHandCursor);
    m_stop->setIcon(makeRecordingStopIcon(QColor(248, 113, 113)));
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
    const QColor ink(226, 232, 240);
    m_pause->setIcon(m_paused ? makeRecordingResumeIcon(ink) : makeRecordingPauseIcon(ink));
    m_pause->setToolTip(m_paused ? MS_TR("Resume Recording") : MS_TR("Pause Recording"));
}

void RecordingControlBar::paintEvent(QPaintEvent *event)
{
    Q_UNUSED(event)
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.setPen(Qt::NoPen);
    painter.setBrush(QColor(15, 18, 26, 235));
    painter.drawRoundedRect(rect(), kCornerRadius, kCornerRadius);
}

}  // namespace markshot::recording::ui
