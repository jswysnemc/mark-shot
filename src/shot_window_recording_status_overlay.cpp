#include "shot_window_module.h"

#include "recording/recording_session_manager.h"
#include "recording/recording_status.h"

using namespace markshot::shot;

namespace {

/**
 * 格式化录制持续时间。
 * @param elapsedMs 已录制毫秒数。
 * @return 持续时间文本。
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
 * 返回录制模式显示名称。
 * @param mode 录制模式。
 * @return 模式显示名称。
 */
QString modeLabel(markshot::recording::RecordingMode mode)
{
    return mode == markshot::recording::RecordingMode::Gif
        ? QStringLiteral("GIF")
        : MS_TR("Video");
}

/**
 * 计算录制状态面板区域。
 * @param bounds 窗口边界。
 * @return 面板矩形。
 */
QRectF panelRect(const QRect &bounds)
{
    const qreal width = std::clamp<qreal>(bounds.width() * 0.42, 380.0, 620.0);
    const qreal height = 188.0;
    return QRectF((bounds.width() - width) / 2.0,
                  (bounds.height() - height) / 2.0,
                  width,
                  height);
}

/**
 * 计算停止按钮区域。
 * @param panel 面板矩形。
 * @return 停止按钮矩形。
 */
QRectF stopButtonRect(const QRectF &panel)
{
    const QSizeF buttonSize(168.0, 38.0);
    return QRectF(panel.center().x() - buttonSize.width() / 2.0,
                  panel.bottom() - buttonSize.height() - 18.0,
                  buttonSize.width(),
                  buttonSize.height());
}

}  // namespace

void ShotWindow::initializeActiveRecordingOverlay()
{
    m_activeRecordingOverlayTimer = new QTimer(this);
    m_activeRecordingOverlayTimer->setInterval(1000);
    connect(m_activeRecordingOverlayTimer, &QTimer::timeout, this, [this] {
        if (activeRecordingAvailable()) {
            update();
        } else if (m_activeRecordingOverlayTimer->isActive()) {
            m_activeRecordingOverlayTimer->stop();
            update();
        }
    });
    connect(&markshot::recording::RecordingSessionManager::instance(),
            &markshot::recording::RecordingSessionManager::statusChanged,
            this,
            [this] {
                if (activeRecordingAvailable()) {
                    if (!m_activeRecordingOverlayTimer->isActive()) {
                        m_activeRecordingOverlayTimer->start();
                    }
                } else if (m_activeRecordingOverlayTimer->isActive()) {
                    m_activeRecordingOverlayTimer->stop();
                }
                update();
            });
    if (activeRecordingAvailable()) {
        m_activeRecordingOverlayTimer->start();
    }
}

bool ShotWindow::activeRecordingAvailable() const
{
    return markshot::recording::RecordingSessionManager::instance().status().active;
}

bool ShotWindow::stopActiveRecordingFromOverlay()
{
    QString error;
    const bool stopped = markshot::recording::RecordingSessionManager::instance().stop(&error);
    if (!stopped && !error.isEmpty()) {
        showToast(error, 1800);
    }
    update();
    return stopped;
}

QRectF ShotWindow::activeRecordingStopButtonRect() const
{
    return stopButtonRect(panelRect(rect()));
}

void ShotWindow::drawActiveRecordingStatus(QPainter &painter)
{
    const markshot::recording::RecordingStatus status =
        markshot::recording::RecordingSessionManager::instance().status();
    if (!status.active || m_mode != Mode::Selecting || hasUsableSelection()) {
        return;
    }

    const QRectF panel = panelRect(rect());
    const QRectF button = stopButtonRect(panel);
    painter.save();
    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.setPen(Qt::NoPen);
    painter.setBrush(QColor(3, 7, 18, 224));
    painter.drawRoundedRect(panel, 18.0, 18.0);

    painter.setFont(markshot::theme::uiFont(15, QFont::DemiBold));
    painter.setPen(QColor(248, 250, 252));
    painter.drawText(panel.adjusted(24.0, 20.0, -24.0, 0.0),
                     Qt::AlignTop | Qt::AlignHCenter,
                     MS_TR("Recording in Progress"));

    painter.setFont(markshot::theme::uiFont(11, QFont::DemiBold));
    painter.setPen(QColor(125, 211, 252));
    const QString statusText = MS_TR("%1 recording  %2  %3 fps")
        .arg(modeLabel(status.mode), formatElapsed(status.elapsedMs))
        .arg(status.fps);
    painter.drawText(panel.adjusted(24.0, 54.0, -24.0, 0.0),
                     Qt::AlignTop | Qt::AlignHCenter,
                     statusText);

    painter.setFont(markshot::theme::uiFont(10));
    painter.setPen(QColor(203, 213, 225));
    const QString outputText = status.outputPath.isEmpty()
        ? QString()
        : MS_TR("Output: %1").arg(status.outputPath);
    painter.drawText(panel.adjusted(28.0, 84.0, -28.0, 0.0),
                     Qt::AlignTop | Qt::AlignHCenter,
                     painter.fontMetrics().elidedText(outputText, Qt::ElideMiddle, qRound(panel.width() - 56.0)));

    painter.setPen(QColor(226, 232, 240));
    painter.drawText(panel.adjusted(28.0, 111.0, -28.0, 0.0),
                     Qt::AlignTop | Qt::AlignHCenter,
                     MS_TR("Hold S to stop recording, or continue selecting a screenshot region."));

    painter.setPen(Qt::NoPen);
    painter.setBrush(QColor(220, 38, 38));
    painter.drawRoundedRect(button, 12.0, 12.0);
    painter.setFont(markshot::theme::uiFont(11, QFont::DemiBold));
    painter.setPen(QColor(255, 255, 255));
    painter.drawText(button, Qt::AlignCenter, MS_TR("Stop Recording"));
    painter.restore();
}
