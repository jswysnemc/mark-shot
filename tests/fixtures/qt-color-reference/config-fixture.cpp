// 【界面对照】【配置样本】只为原版控件提供隔离颜色历史，不读写用户配置

#include "app_config_store.h"
#include <QJsonArray>

namespace markshot {

/** 【界面对照】【历史输入】ok 接收读取成功状态，返回来自环境变量的固定 JSON 样本。 */
QJsonObject readAppConfigRoot(bool *ok)
{
    if (ok) *ok = true;
    const QJsonDocument document = QJsonDocument::fromJson(qgetenv("MARK_SHOT_REFERENCE_HISTORY"));
    return {{"colorPicker", QJsonObject{{"history", document.array()}}}};
}

/** 【界面对照】【只读边界】忽略 root，error 接收失败原因，返回 false；探针不允许持久化配置。 */
bool writeAppConfigRoot(const QJsonObject &, QString *error)
{
    if (error) *error = QStringLiteral("reference fixture is read-only");
    return false;
}

/** 【界面对照】【只读更新】忽略路径和值，error 接收失败原因，返回 false；不改变原版配置。 */
bool writeAppConfigValue(const QStringList &, const QJsonValue &, QString *error)
{
    return writeAppConfigRoot({}, error);
}
}
