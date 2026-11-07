/** 【模型翻译对照】【原版解析】只读编译原版模型解析，不调用请求函数、读取配置或使用真实凭据 */
#include "../../../../mark-shot/qt/plugins/translate-gemini/gemini_translate_request.cpp"
#include "../../../../mark-shot/qt/plugins/translate-anthropic/anthropic_translate_request.cpp"
#include "../../../../mark-shot/qt/plugins/translate-common/translate_http_client.cpp"
#include <QCoreApplication>
#include <iostream>

/** 【模型翻译对照】【解析记录】按 provider 和 body 调用真实原版解析器，返回状态、错误和身份映射 */
static QJsonObject record(const QString &provider, const QByteArray &body)
{
    QHash<int, QString> translations;
    QString error;
    const bool ok = provider == "gemini"
        ? markshot::translate_gemini::parseGeminiResponse(body, &translations, &error)
        : markshot::translate_anthropic::parseAnthropicResponse(body, &translations, &error);
    QJsonObject output;
    for (auto it = translations.cbegin(); it != translations.cend(); ++it) output[QString::number(it.key())] = it.value();
    return {{"provider", provider}, {"body", QString::fromUtf8(body)}, {"ok", ok}, {"error", error}, {"translations", output}};
}

/** 【模型翻译对照】【入口】argc、argv 指定普通程序参数，返回成功状态，只执行固定成功和失败输入 */
int main(int argc, char **argv)
{
    QCoreApplication application(argc, argv);
    QJsonArray cases;
    const QString content = "{\"translations\":[{\"id\":2,\"text\":\" translated \"},{\"id\":2,\"text\":\"last\"}]}";
    for (const QString &provider : {QStringLiteral("gemini"), QStringLiteral("anthropic")}) {
        for (const QByteArray &body : {QByteArrayLiteral("{}"), QByteArrayLiteral("{\"error\":{\"type\":\"authentication_error\",\"message\":\"denied\"}}"), QByteArrayLiteral("{\"error\":null}")}) cases.append(record(provider, body));
        for (const QString &text : {content, QStringLiteral("```json\n") + content + "\n```", QStringLiteral("not JSON"), QStringLiteral("{}"), QStringLiteral("{\"translations\":[]}"), QStringLiteral(" ")}) {
            const QJsonArray parts{QJsonObject{{"type", "text"}, {"text", text}}};
            const QJsonObject body = provider == "gemini"
                ? QJsonObject{{"candidates", QJsonArray{QJsonObject{{"content", QJsonObject{{"parts", parts}}}}}}}
                : QJsonObject{{"content", parts}};
            cases.append(record(provider, QJsonDocument(body).toJson(QJsonDocument::Compact)));
        }
    }
    for (const QByteArray &body : {QByteArrayLiteral("{\"candidates\":[]}"), QByteArrayLiteral("{\"candidates\":[{}]}"), QByteArrayLiteral("{\"candidates\":[{\"content\":{\"parts\":[]}}]}"), QByteArrayLiteral("{\"candidates\":[{\"content\":{\"parts\":[{}, {\"text\":\"ignored\"}]}}]}"), QByteArrayLiteral("{\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"{\\\"translations\\\":[]}\"}]}},{\"content\":{\"parts\":[{\"text\":\"ignored\"}]}}]}"), QByteArrayLiteral("{\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"{\\\"translations\\\":[]}\"},{\"text\":\"ignored\"}]}}]}")}) cases.append(record("gemini", body));
    for (const QByteArray &body : {QByteArrayLiteral("{\"content\":[]}"), QByteArrayLiteral("{\"content\":[{\"type\":\"thinking\",\"text\":\"ignored\"}]}"), QByteArrayLiteral("{\"content\":[{\"type\":\"text\",\"text\":\"{\\\"translations\\\":\"},{\"type\":\"thinking\",\"text\":\"ignored\"},{\"type\":\"text\",\"text\":\"[]}\"}]}"), QByteArrayLiteral("{\"content\":[{\"text\":\"ignored\"}]}")}) cases.append(record("anthropic", body));
    std::cout << QJsonDocument(QJsonObject{{"cases", cases}}).toJson(QJsonDocument::Compact).toStdString() << std::flush;
    return 0;
}
