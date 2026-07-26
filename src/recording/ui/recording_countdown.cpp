#include "recording/ui/recording_countdown.h"

#include "recording/ui/recording_overlay_layout.h"
#include "ui/i18n.h"
#include "ui/theme.h"
#include "windows_integration.h"

#include <QGuiApplication>
#include <QLabel>
#include <QPainter>
#include <QScreen>
#include <QTimer>
#include <QVBoxLayout>
#include <QWidget>

#include <memory>
#include <utility>

namespace markshot::recording::ui {
namespace {

// 倒计时面板尺寸，与录制控制条保持同一视觉体量
constexpr int kPanelWidth = 236;
constexpr int kPanelHeight = 40;
constexpr int kCornerRadius = 10;

/**
 * 【录制】【倒计时】倒计时提示面板。
 */
class RecordingCountdownPanel final : public QWidget {
public:
    /**
     * 创建倒计时面板。
     * @param screen 目标屏幕。
     * @param regionRect 录制区域矩形。
     */
    RecordingCountdownPanel(QScreen *screen, const QRect &regionRect)
        : QWidget(nullptr)
    {
        setWindowFlags(Qt::Tool | Qt::FramelessWindowHint | Qt::WindowStaysOnTopHint
                       | Qt::WindowDoesNotAcceptFocus | Qt::BypassWindowManagerHint);
        setAttribute(Qt::WA_TranslucentBackground, true);
        setAttribute(Qt::WA_ShowWithoutActivating, true);
        setAttribute(Qt::WA_TransparentForMouseEvents, true);
        setFocusPolicy(Qt::NoFocus);
        if (screen) {
            setScreen(screen);
        }

        auto *layout = new QVBoxLayout(this);
        layout->setContentsMargins(16, 0, 16, 0);
        m_label = new QLabel(this);
        m_label->setAlignment(Qt::AlignCenter);
        m_label->setFont(markshot::theme::uiFont(12, QFont::DemiBold));
        m_label->setStyleSheet(QStringLiteral("color: #f8fafc;"));
        layout->addWidget(m_label);

        const QRect screenRect = screen ? screen->geometry() : QRect();
        const QRect panelRect = recordingControlBarRect(regionRect,
                                                        screenRect,
                                                        QSize(kPanelWidth, kPanelHeight));
        setGeometry(panelRect.isEmpty() ? QRect(0, 0, kPanelWidth, kPanelHeight) : panelRect);
        markshot::windows::setExcludedFromCapture(this);
    }

    /**
     * 更新剩余秒数文案。
     * @param seconds 剩余秒数。
     * @return 无返回值。
     */
    void setRemaining(int seconds)
    {
        if (m_label) {
            m_label->setText(MS_TR("Recording starts in %1...").arg(seconds));
        }
    }

protected:
    /**
     * 绘制面板背景。
     * @param event 绘制事件。
     * @return 无返回值。
     */
    void paintEvent(QPaintEvent *event) override
    {
        Q_UNUSED(event)
        QPainter painter(this);
        painter.setRenderHint(QPainter::Antialiasing, true);
        painter.setPen(Qt::NoPen);
        painter.setBrush(QColor(15, 18, 26, 235));
        painter.drawRoundedRect(rect(), kCornerRadius, kCornerRadius);
    }

private:
    QLabel *m_label = nullptr;
};

}  // namespace

void runRecordingCountdown(int seconds,
                           const QRect &regionRect,
                           QScreen *screen,
                           std::function<void()> onFinished)
{
    // 1. 未开启倒计时时立即启动录制
    if (seconds <= 0) {
        if (onFinished) {
            onFinished();
        }
        return;
    }

    QScreen *targetScreen = screen ? screen : QGuiApplication::primaryScreen();
    auto *panel = new RecordingCountdownPanel(targetScreen, regionRect);
    panel->setRemaining(seconds);
    panel->show();

    // 2. 每秒递减，结束后销毁面板并启动录制
    auto *timer = new QTimer(panel);
    auto remaining = std::make_shared<int>(seconds);
    timer->setInterval(1000);
    QObject::connect(timer, &QTimer::timeout, panel, [panel, timer, remaining, onFinished] {
        --(*remaining);
        if (*remaining > 0) {
            panel->setRemaining(*remaining);
            return;
        }
        timer->stop();
        panel->hide();
        panel->deleteLater();
        if (onFinished) {
            onFinished();
        }
    });
    timer->start();
}

}  // namespace markshot::recording::ui
