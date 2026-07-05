#include "openai_translate_config.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonDocument>
#include <QJsonObject>
#include <QProcessEnvironment>
#include <QStandardPaths>

#include <algorithm>

namespace markshot::translate_openai {
namespace {

/**
 * 读取去除空白后的环境变量。
 * @param env 环境变量集合。
 * @param name 变量名。
 * @return 变量值。
 */
QString envValue(const QProcessEnvironment &env, const QString &name)
{
    return env.value(name).trimmed();
}

/**
 * 读取首个非空字符串。
 * @param configValue 配置文件取值。
 * @param env 环境变量集合。
 * @param envNames 环境变量候选名。
 * @param fallback 兜底值。
 * @return 解析后的取值。
 */
QString firstNonEmpty(const QString &configValue,
                      const QProcessEnvironment &env,
                      const QStringList &envNames,
                      const QString &fallback)
{
    if (!configValue.trimmed().isEmpty()) {
        return configValue.trimmed();
    }
    for (const QString &name : envNames) {
        const QString value = envValue(env, name);
        if (!value.isEmpty()) {
            return value;
        }
    }
    return fallback;
}

/**
 * 追加配置文件候选路径。
 * @param paths 路径列表。
 * @param path 待追加路径。
 * @return 无返回值。
 */
void addConfigPath(QStringList *paths, const QString &path)
{
    const QString trimmed = path.trimmed();
    if (!trimmed.isEmpty() && !paths->contains(trimmed)) {
        paths->append(trimmed);
    }
}

/**
 * 追加配置目录候选路径。
 * @param paths 路径列表。
 * @param dir 配置目录。
 * @return 无返回值。
 */
void addConfigDir(QStringList *paths, const QString &dir)
{
    if (!dir.trimmed().isEmpty()) {
        addConfigPath(paths, QDir(dir).filePath(QStringLiteral("config.json")));
    }
}

#if defined(Q_OS_WIN)
/**
 * 按 Windows 环境变量构造配置目录。
 * @param env 环境变量集合。
 * @param name 变量名。
 * @param relativePath 相对路径。
 * @return 配置目录。
 */
QString windowsConfigDir(const QProcessEnvironment &env,
                         const QString &name,
                         const QString &relativePath = QStringLiteral("mark-shot"))
{
    const QString root = envValue(env, name);
    return root.isEmpty() ? QString() : QDir(root).filePath(relativePath);
}
#endif

/**
 * 读取配置文件候选路径。
 * @param env 环境变量集合。
 * @return 候选配置文件路径列表。
 */
QStringList configPathCandidates(const QProcessEnvironment &env)
{
    QStringList paths;
    addConfigPath(&paths, envValue(env, QStringLiteral("MARK_SHOT_CONFIG")));

#if defined(Q_OS_WIN)
    addConfigDir(&paths, windowsConfigDir(env, QStringLiteral("LOCALAPPDATA")));
    addConfigDir(&paths, windowsConfigDir(env, QStringLiteral("APPDATA")));
    addConfigDir(&paths,
                 windowsConfigDir(env, QStringLiteral("USERPROFILE"), QStringLiteral("AppData/Local/mark-shot")));
#endif

    const QString appConfig = QStandardPaths::writableLocation(QStandardPaths::AppConfigLocation);
    addConfigDir(&paths, appConfig);

    const QString genericConfig = QStandardPaths::writableLocation(QStandardPaths::GenericConfigLocation);
    addConfigDir(&paths, QDir(genericConfig).filePath(QStringLiteral("mark-shot")));
    addConfigDir(&paths, QDir::home().filePath(QStringLiteral(".config/mark-shot")));
    return paths;
}

/**
 * 读取应用配置中的 translation 对象。
 * @param env 环境变量集合。
 * @return translation JSON 对象，读取失败时为空对象。
 */
QJsonObject readTranslationConfig(const QProcessEnvironment &env)
{
    for (const QString &path : configPathCandidates(env)) {
        QFile file(path);
        if (!QFileInfo::exists(path) || !file.open(QIODevice::ReadOnly | QIODevice::Text)) {
            continue;
        }
        const QJsonDocument document = QJsonDocument::fromJson(file.readAll());
        if (document.isObject()) {
            return document.object().value(QStringLiteral("translation")).toObject();
        }
    }
    return {};
}

}  // namespace

QString defaultSystemPrompt()
{
    return QStringLiteral("You translate OCR text segments. Preserve meaning, keep segment count and ids "
                          "unchanged, and return only valid JSON.");
}

OpenAiTranslateConfig readOpenAiTranslateConfig()
{
    const QProcessEnvironment env = QProcessEnvironment::systemEnvironment();
    const QJsonObject config = readTranslationConfig(env);

    OpenAiTranslateConfig result;
    result.apiBase = firstNonEmpty(
        config.value(QStringLiteral("apiBase")).toString(config.value(QStringLiteral("baseUrl")).toString()),
        env,
        {QStringLiteral("MARK_SHOT_LLM_API_BASE"),
         QStringLiteral("OPENAI_BASE_URL"),
         QStringLiteral("OPENAI_API_BASE")},
        result.apiBase);
    result.model = firstNonEmpty(config.value(QStringLiteral("model")).toString(),
                                 env,
                                 {QStringLiteral("MARK_SHOT_LLM_MODEL"), QStringLiteral("OPENAI_MODEL")},
                                 result.model);
    result.apiKeyEnv = firstNonEmpty(config.value(QStringLiteral("apiKeyEnv")).toString(),
                                     env,
                                     {},
                                     result.apiKeyEnv);
    result.apiKey = firstNonEmpty(config.value(QStringLiteral("apiKey")).toString(),
                                  env,
                                  {result.apiKeyEnv, QStringLiteral("MARK_SHOT_LLM_API_KEY")},
                                  {});
    result.systemPrompt = firstNonEmpty(config.value(QStringLiteral("systemPrompt")).toString(),
                                        env,
                                        {},
                                        defaultSystemPrompt());
    if (config.contains(QStringLiteral("temperature"))) {
        result.temperature = config.value(QStringLiteral("temperature")).toDouble(result.temperature);
    }
    if (config.contains(QStringLiteral("timeoutMs"))) {
        result.timeoutMs = std::clamp(config.value(QStringLiteral("timeoutMs")).toInt(result.timeoutMs),
                                      1000,
                                      300000);
    }
    return result;
}

bool validateOpenAiTranslateConfig(const OpenAiTranslateConfig &config, QString *error)
{
    if (config.apiBase.trimmed().isEmpty()) {
        if (error) {
            *error = QStringLiteral("missing translation apiBase");
        }
        return false;
    }
    if (config.model.trimmed().isEmpty()) {
        if (error) {
            *error = QStringLiteral("missing translation model");
        }
        return false;
    }
    if (config.apiKey.trimmed().isEmpty()) {
        if (error) {
            *error = QStringLiteral("missing api key: set %1 or translation.apiKey").arg(config.apiKeyEnv);
        }
        return false;
    }
    return true;
}

}  // namespace markshot::translate_openai
