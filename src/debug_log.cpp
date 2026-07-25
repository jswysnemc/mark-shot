#include "debug_log.h"

#include <QByteArray>
#include <QDateTime>
#include <QDir>
#include <QFileInfo>
#include <QString>

#include <cstdio>
#include <cstdlib>
#include <cstring>

namespace markshot {

namespace {

struct DebugState {
    bool configured = false;
    bool enabled = false;
    QString logPath;
};

DebugState &debugState()
{
    static DebugState state;
    return state;
}

bool truthyDebugValue(const QByteArray &raw)
{
    const QByteArray value = raw.trimmed().toLower();
    if (value.isEmpty() || value == "0" || value == "false" || value == "off" || value == "no") {
        return false;
    }
    return true;
}

bool environmentDebugEnabled()
{
    const char *raw = std::getenv("MARK_SHOT_DEBUG");
    if (raw) {
        return truthyDebugValue(QByteArray(raw));
    }
    raw = std::getenv("DEBUG");
    return raw && truthyDebugValue(QByteArray(raw));
}

QString defaultLogPath()
{
    const char *override = std::getenv("MARK_SHOT_DEBUG_LOG");
    if (override && override[0] != '\0') {
        return QString::fromLocal8Bit(override);
    }
    return QDir::temp().filePath(QStringLiteral("mark-shot-scroll.log"));
}

void ensureLogDir(const QString &path)
{
    const QString dirPath = QFileInfo(path).absolutePath();
    if (!dirPath.isEmpty()) {
        QDir().mkpath(dirPath);
    }
}

}  // namespace

void configureDebugLogging(bool enabled, const QString &logPath)
{
    DebugState &state = debugState();
    state.configured = true;
    state.enabled = enabled;
    state.logPath = logPath.trimmed().isEmpty() ? defaultLogPath() : logPath;
}

bool debugEnabled()
{
    const DebugState &state = debugState();
    return state.configured ? state.enabled : environmentDebugEnabled();
}

QString debugLogPath()
{
    const DebugState &state = debugState();
    if (state.configured && !state.logPath.isEmpty()) {
        return state.logPath;
    }
    return defaultLogPath();
}

/// @brief Helper to write formatted debug log messages to stderr and/or the debug log file.
/// @param category The logging category or tag.
/// @param format The format string for the log message.
/// @param args The variable arguments list.
void debugLogV(const char *category, const char *format, va_list args)
{
    if (!debugEnabled()) {
        return;
    }

    const QByteArray stamp =
        QDateTime::currentDateTime().toString(QStringLiteral("HH:mm:ss.zzz")).toUtf8();
    const char *tag = category ? category : "log";

    // Render the message once so the same text reaches both sinks.
    char message[2048];
    std::vsnprintf(message, sizeof(message), format, args);

    const QString path = debugLogPath();
    ensureLogDir(path);
    const QByteArray localPath = path.toLocal8Bit();
    if (FILE *file = std::fopen(localPath.constData(), "a")) {
        std::fprintf(file, "%s [%s] %s\n", stamp.constData(), tag, message);
        std::fclose(file);
    }
    std::fprintf(stderr, "[mark-shot][%s] %s\n", tag, message);
    std::fflush(stderr);
}

void debugLog(const char *category, const char *format, ...)
{
    if (!debugEnabled()) {
        return;
    }
    va_list args;
    va_start(args, format);
    debugLogV(category, format, args);
    va_end(args);
}

}  // namespace markshot
