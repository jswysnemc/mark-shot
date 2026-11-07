/** 【翻译服务对照】【原版实现】只读编译原版签名和解析函数，输出测试向量，不读取配置或连接网络 */
#include "../../../../mark-shot/qt/plugins/translate-tencent/tencent_tc3_signer.cpp"
#include "../../../../mark-shot/qt/plugins/translate-baidu/baidu_translate_signer.cpp"
#include "../../../../mark-shot/qt/plugins/translate-youdao/youdao_translate_signer.cpp"
#include "../../../../mark-shot/qt/plugins/translate-openai/openai_translation_parser.cpp"
#include "../../../../mark-shot/qt/plugins/translate-common/translate_language_code.cpp"
#include "translation_language_options.h"
#include "ui/i18n.h"
#include <QCoreApplication>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QUrl>
#include <iostream>

/** 【翻译服务对照】【入口】argc、argv 指定程序参数，返回成功状态；仅使用固定原文和虚构密钥 */
int main(int argc, char **argv)
{
    QCoreApplication application(argc, argv);
    markshot::i18n::setLanguage(markshot::i18n::Language::English);
    QJsonArray baidu, youdao, tc3, content, languages, aliases, localized;
    const QString astral = QString::fromUcs4(U"\U0001F680");
    for (const QString &query : {QStringLiteral("hello"), QStringLiteral("原文加号+与空格 %\n第二行"),
        QStringLiteral("12345678901234567890"), QStringLiteral("123456789012345678901"),
        QStringLiteral("123456789") + astral + QStringLiteral("abcdefghijkmnopqrstuv"),
        QStringLiteral("abcdefghijklmnopqrstuvw") + astral + QStringLiteral("123456789")}) {
        const QString input = markshot::translate_youdao::youdaoSignInput(query);
        youdao.append(QJsonObject{{"query", query}, {"utf16_length", query.size()},
            {"input_utf8_hex", QString::fromLatin1(input.toUtf8().toHex())},
            {"signature", markshot::translate_youdao::youdaoSignature("test-app", input, "test-salt", "1551113065", "test-secret")}});
        baidu.append(QJsonObject{{"query", query}, {"signature", markshot::translate_baidu::baiduSignature("test-app", query, "test-salt", "test-secret")}});
    }
    for (const qint64 timestamp : {qint64(1551113065), qint64(0), qint64(86400), qint64(-1)}) {
        markshot::translate_tencent::Tc3SigningInput input;
        input.host = "TMT.TencentCloudApi.com";
        input.payload = QByteArrayLiteral("{\"ProjectId\":0,\"Source\":\"auto\",\"SourceTextList\":[\"hello\"],\"Target\":\"zh\"}");
        input.secretId = "test-id"; input.secretKey = "test-secret"; input.timestamp = timestamp;
        const QByteArray canonical = markshot::translate_tencent::tc3CanonicalRequest(input);
        tc3.append(QJsonObject{{"timestamp", timestamp}, {"body", QString::fromUtf8(input.payload)},
            {"canonical", QString::fromUtf8(canonical)}, {"date", markshot::translate_tencent::tc3UtcDate(timestamp)},
            {"authorization", QString::fromUtf8(markshot::translate_tencent::tc3AuthorizationHeader(input))}});
    }
    for (const QString &value : {QStringLiteral("{\"translations\":[{\"id\":2,\"text\":\" first \"},{\"id\":2,\"text\":\"last\"}]}"),
        QStringLiteral("```json\n{\"translations\":[{\"id\":0,\"text\":\"译文\"}]}\n```"),
        QStringLiteral("{\"translations\":[null,{\"text\":\"missing id\"},{\"id\":\"7\",\"text\":\"string id\"},{\"id\":3.5,\"text\":\"fraction\"},{\"id\":2.0,\"text\":true}]}"),
        QStringLiteral("{\"translations\":[]}"), QStringLiteral("{}"), QStringLiteral("not JSON")}) {
        QHash<int, QString> translations;
        QString error;
        const bool ok = markshot::translate_openai::parseTranslationContent(value, &translations, &error);
        QJsonObject parsed;
        for (auto it = translations.cbegin(); it != translations.cend(); ++it) parsed[QString::number(it.key())] = it.value();
        content.append(QJsonObject{{"input", value}, {"ok", ok}, {"translations", parsed}, {"error", error}});
    }
    for (const QString &value : {QString(), QStringLiteral("Simplified Chinese"), QStringLiteral("zh_CN"),
        QStringLiteral("繁體中文"), QStringLiteral("简体中文"), QStringLiteral("繁体中文"), QStringLiteral("English"), QStringLiteral("Japanese"), QStringLiteral("Korean"),
        QStringLiteral("Deutsch"), QStringLiteral("French"), QStringLiteral("Spanish"), QStringLiteral("Italian"), QStringLiteral("Klingon")}) {
        languages.append(QJsonObject{{"input", value}, {"key", markshot::translate_common::canonicalLanguageKey(value)},
            {"ui_value", markshot::translationLanguageValueFromText(value)}});
    }
    QStringList keys = markshot::translate_common::languageAliases().keys();
    keys.sort();
    for (const QString &key : keys) aliases.append(QJsonObject{{"input", key}, {"key", markshot::translate_common::canonicalLanguageKey(key)}});
    markshot::i18n::setLanguage(markshot::i18n::Language::Chinese);
    for (const auto &option : markshot::translationLanguageOptions()) localized.append(QJsonObject{{"input", option.label},
        {"ui_value", markshot::translationLanguageValueFromText(option.label)}});
    const QString plain = "A+B %~*";
    const QByteArray encoded = QUrl::toPercentEncoding(plain);
    QByteArray formBaidu = encoded; formBaidu.replace("%20", "+");
    const QJsonObject result{{"baidu", baidu}, {"youdao", youdao}, {"tc3", tc3}, {"content", content}, {"languages", languages}, {"aliases", aliases}, {"localized", localized},
        {"form", QJsonObject{{"plain", plain}, {"baidu", QString::fromLatin1(formBaidu)}, {"youdao", QString::fromLatin1(encoded)}}}};
    std::cout << QJsonDocument(result).toJson(QJsonDocument::Compact).toStdString() << std::flush;
    return 0;
}
