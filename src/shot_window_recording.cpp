#include "shot_window_module.h"

#include "recording/recording_config_dialog.h"
#include "recording/recording_start_flow.h"
#include "recording/recording_session_manager.h"

using namespace markshot::shot;

namespace {

/**
 * 判断当前是否存在可见顶层窗口。
 * @return 存在可见顶层窗口时返回 true。
 */
bool hasVisibleTopLevelWindow()
{
    const QWidgetList widgets = QApplication::topLevelWidgets();
    for (QWidget *widget : widgets) {
        if (widget && widget->isVisible()) {
            return true;
        }
    }
    return false;
}

}  // namespace

/**
 * 将启动工具转换为录制模式。
 * @param tool 启动阶段工具。
 * @return 录制模式，非录制工具返回空值。
 */
std::optional<markshot::recording::RecordingMode> ShotWindow::recordingModeForStartupTool(StartupTool tool) const
{
    if (tool == StartupTool::GifRecorder) {
        return markshot::recording::RecordingMode::Gif;
    }
    if (tool == StartupTool::VideoRecorder) {
        return markshot::recording::RecordingMode::Video;
    }
    return std::nullopt;
}

/**
 * 打开启动阶段录制配置。
 * @param mode 录制模式。
 * @return 无返回值。
 */
void ShotWindow::beginStartupRecording(markshot::recording::RecordingMode mode)
{
    if (m_mode != Mode::Selecting || m_recordingConfigDialogOpen || activeRecordingAvailable()) {
        return;
    }

    m_recordingConfigDialogOpen = true;
    const bool overlayWasVisible = isVisible();
    const bool quitOnLastWindowClosed = QApplication::quitOnLastWindowClosed();
    QApplication::setQuitOnLastWindowClosed(false);

    auto restoreOverlay = [this, overlayWasVisible, quitOnLastWindowClosed] {
        QApplication::setQuitOnLastWindowClosed(quitOnLastWindowClosed);
        if (overlayWasVisible && !isVisible()) {
            show();
            raise();
            activateWindow();
            setFocus(Qt::ShortcutFocusReason);
        }
    };

    if (overlayWasVisible) {
        hide();
        QApplication::processEvents();
    }

    markshot::recording::RecordingStartFlowRequest request;
    request.initialMode = mode;
    request.stayOnTop = true;
    request.startDisplayRecording = [this, quitOnLastWindowClosed](markshot::recording::RecordingOptions options) {
        QApplication::setQuitOnLastWindowClosed(quitOnLastWindowClosed);
        m_pendingRecordingOptions.reset();
        startRecording(std::move(options));
    };
    request.selectRegionRecording = [this, restoreOverlay](markshot::recording::RecordingOptions options) {
        restoreOverlay();
        beginRegionRecordingSelection(std::move(options));
    };
    request.showError = [this, restoreOverlay](const QString &message) {
        restoreOverlay();
        QMessageBox::warning(this, QStringLiteral("Mark Shot"), message);
    };

    const bool accepted = markshot::recording::runRecordingStartFlow(request);
    m_recordingConfigDialogOpen = false;
    if (!accepted) {
        restoreOverlay();
        if (recordingModeForStartupTool(m_startupTool).has_value()) {
            leaveStartupTool();
        }
        return;
    }
}

/**
 * 使用已确认录制配置进入区域录制选区状态。
 * @param options 已确认的录制配置。
 * @return 无返回值。
 */
void ShotWindow::beginRegionRecordingSelection(markshot::recording::RecordingOptions options)
{
    if (m_mode != Mode::Selecting || activeRecordingAvailable()) {
        return;
    }

    options.scope = markshot::recording::RecordingScope::Region;
    m_pendingRecordingOptions = std::move(options);
    const StartupTool targetTool = m_pendingRecordingOptions->mode == markshot::recording::RecordingMode::Gif
        ? StartupTool::GifRecorder
        : StartupTool::VideoRecorder;
    if (m_startupTool != targetTool) {
        setStartupTool(targetTool);
    }
    showToast(MS_TR("Select recording region"), 1800);
}

/**
 * 使用当前选区启动待处理录制任务。
 * @return 已处理录制选区时返回 true。
 */
bool ShotWindow::handleStartupRecordingSelection()
{
    if (!recordingModeForStartupTool(m_startupTool).has_value()) {
        return false;
    }
    if (!m_pendingRecordingOptions.has_value()) {
        leaveStartupTool();
        return true;
    }

    const QRect imageBounds(QPoint(0, 0), m_frozenFrame.size());
    const QRect imageSelection = normalizedSelection().toAlignedRect().intersected(imageBounds);
    const QRect geometry = markshot::capture::geometryFromImageRect(imageSelection,
                                                                    m_sourceGeometry,
                                                                    m_frozenFrame.size());
    if (geometry.isEmpty()) {
        showToast(MS_TR("Invalid recording region"), 1800);
        m_selection = {};
        update();
        return true;
    }

    markshot::recording::RecordingOptions options = *m_pendingRecordingOptions;
    options.scope = markshot::recording::RecordingScope::Region;
    options.captureGeometry = geometry;
    m_pendingRecordingOptions.reset();
    startRecording(std::move(options));
    return true;
}

/**
 * 启动录制控制器并关闭截图覆盖层。
 * @param options 录制配置。
 * @return 无返回值。
 */
void ShotWindow::startRecording(markshot::recording::RecordingOptions options)
{
    const bool quitOnLastWindowClosed = QApplication::quitOnLastWindowClosed();
    QApplication::setQuitOnLastWindowClosed(false);

    hide();
    QApplication::processEvents();

    auto &manager = markshot::recording::RecordingSessionManager::instance();
    QString error;
    if (!manager.start(options, qApp, &error)) {
        QApplication::setQuitOnLastWindowClosed(quitOnLastWindowClosed);
        show();
        raise();
        QMessageBox::warning(this,
                             QStringLiteral("Mark Shot"),
                             error.isEmpty() ? MS_TR("Recording failed to start") : error);
        return;
    }

    QObject::connect(&manager,
                     &markshot::recording::RecordingSessionManager::recordingFinished,
                     qApp,
                     [quitOnLastWindowClosed] {
                         QApplication::setQuitOnLastWindowClosed(quitOnLastWindowClosed);
                         if (quitOnLastWindowClosed && !hasVisibleTopLevelWindow()) {
                             QCoreApplication::quit();
                         }
                     },
                     Qt::SingleShotConnection);

    emit sessionCancelRequested();
    close();
}
