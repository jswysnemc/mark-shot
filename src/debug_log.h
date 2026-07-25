#pragma once

#include <cstdarg>

#include <QString>

namespace markshot {

/// @brief 在启动配置与命令行解析完成后设置调试日志。
/// @param enabled 是否启用调试日志。
/// @param logPath 调试日志文件路径。
void configureDebugLogging(bool enabled, const QString &logPath = QString());

/// @brief 判断运行时配置、MARK_SHOT_DEBUG 或兼容变量 DEBUG 是否启用调试日志。
/// @return 启用调试日志时返回 true。
bool debugEnabled();

/// @brief 返回调试日志文件路径。
/// @return 调试日志文件路径。
QString debugLogPath();

/// @brief 按 printf 格式写入分类调试日志。
/// @param category 日志分类。
/// @param format printf 格式字符串。
void debugLog(const char *category, const char *format, ...)
#if defined(__GNUC__)
    __attribute__((format(printf, 2, 3)))
#endif
    ;

/// @brief 使用 va_list 写入分类调试日志。
/// @param category 日志分类。
/// @param format printf 格式字符串。
/// @param args 可变参数列表。
void debugLogV(const char *category, const char *format, va_list args);

}  // namespace markshot
