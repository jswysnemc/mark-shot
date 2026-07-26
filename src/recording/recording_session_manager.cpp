#include "recording/recording_session_manager.h"

#include "notifications/app_notifications.h"
#include "recording/recording_controller.h"

namespace markshot::recording {

RecordingSessionManager &RecordingSessionManager::instance()
{
    static RecordingSessionManager manager;
    return manager;
}

RecordingSessionManager::RecordingSessionManager(QObject *parent)
    : QObject(parent)
{
}

bool RecordingSessionManager::start(const RecordingOptions &options, QObject *parent, QString *error)
{
    if (error) {
        error->clear();
    }
    if (m_controller) {
        if (error) {
            *error = QStringLiteral("recording is already active");
        }
        return false;
    }

    auto *controller = new RecordingController(parent ? parent : this);
    connect(controller, &RecordingController::statusChanged, this, &RecordingSessionManager::statusChanged);
    connect(controller,
            &RecordingController::finished,
            this,
            [this, controller](bool ok, const QString &outputPath, const QString &message) {
                if (m_controller == controller) {
                    m_controller = nullptr;
                }
                emit statusChanged();
                if (ok) {
                    markshot::notifications::notifyRecordingSaved(outputPath);
                } else {
                    markshot::notifications::notifyRecordingFailed(message);
                }
                emit recordingFinished(ok, outputPath, message);
            });
    if (!controller->start(options, error)) {
        controller->deleteLater();
        return false;
    }

    m_controller = controller;
    emit statusChanged();
    markshot::notifications::notifyRecordingStarted(options);
    return true;
}

bool RecordingSessionManager::stop(QString *error)
{
    if (error) {
        error->clear();
    }
    if (!m_controller) {
        if (error) {
            *error = QStringLiteral("no active recording");
        }
        return false;
    }
    m_controller->requestStop();
    return true;
}

bool RecordingSessionManager::setPaused(bool paused, QString *error)
{
    if (error) {
        error->clear();
    }
    if (!m_controller) {
        if (error) {
            *error = QStringLiteral("no active recording");
        }
        return false;
    }
    if (!m_controller->setPaused(paused)) {
        if (error) {
            *error = paused ? QStringLiteral("recording is already paused")
                            : QStringLiteral("recording is not paused");
        }
        return false;
    }
    emit statusChanged();
    return true;
}

bool RecordingSessionManager::togglePause(QString *error)
{
    if (error) {
        error->clear();
    }
    if (!m_controller) {
        if (error) {
            *error = QStringLiteral("no active recording");
        }
        return false;
    }
    return setPaused(!m_controller->isPaused(), error);
}

RecordingStatus RecordingSessionManager::status() const
{
    return m_controller ? m_controller->status() : RecordingStatus();
}

}  // namespace markshot::recording
