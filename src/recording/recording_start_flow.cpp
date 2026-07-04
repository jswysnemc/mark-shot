#include "recording/recording_start_flow.h"

#include "recording/recording_config_dialog.h"
#include "recording/recording_dialog_config.h"
#include "ui/i18n.h"
#include "windows_integration.h"

#include <QApplication>
#include <QDialog>
#include <QIcon>
#include <QMessageBox>

namespace markshot::recording {
namespace {

/**
 * 显示录制启动错误。
 * @param request 启动流程请求。
 * @param message 错误信息。
 * @return 无返回值。
 */
void showStartFlowError(const RecordingStartFlowRequest &request, const QString &message)
{
    if (request.showError) {
        request.showError(message);
        return;
    }
    QMessageBox::warning(request.parent, QStringLiteral("Mark Shot"), message);
}

}  // namespace

bool runRecordingStartFlow(const RecordingStartFlowRequest &request)
{
    RecordingConfigDialog dialog(request.initialMode, request.parent);
    const QIcon appIcon = QApplication::windowIcon();
    if (!appIcon.isNull()) {
        dialog.setWindowIcon(appIcon);
    }
    dialog.setWindowModality(Qt::ApplicationModal);
    if (request.stayOnTop) {
        dialog.setWindowFlag(Qt::WindowStaysOnTopHint, true);
    }
    markshot::windows::setExcludedFromCapture(&dialog);

    if (dialog.exec() != QDialog::Accepted) {
        return false;
    }

    RecordingOptions options = dialog.options();
    QString saveError;
    saveRecordingDialogConfig(recordingDialogConfigFromOptions(options), &saveError);
    if (options.display.geometry.isEmpty()) {
        showStartFlowError(request, MS_TR("No display is available for recording."));
        return true;
    }

    if (options.scope == RecordingScope::Display) {
        options.captureGeometry = options.display.geometry;
        if (request.startDisplayRecording) {
            request.startDisplayRecording(std::move(options));
        }
        return true;
    }

    options.scope = RecordingScope::Region;
    if (request.selectRegionRecording) {
        request.selectRegionRecording(std::move(options));
    }
    return true;
}

}  // namespace markshot::recording
