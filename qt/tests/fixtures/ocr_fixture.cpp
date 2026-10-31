#include <QCoreApplication>
#include <QFile>
#include <QFileInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QTextStream>

/**
 * 【翻译测试】【固定结果】把输入 JSON 中每段文字加上目标语言前缀后原样输出，框不变
 * @param inputPath 翻译输入 JSON 路径
 * @return 输入无效时返回 3，否则返回 0
 */
int writeFixtureTranslation(const QString &inputPath)
{
    QFile input(inputPath);
    if (!input.open(QIODevice::ReadOnly)) {
        return 3;
    }
    const QJsonObject root = QJsonDocument::fromJson(input.readAll()).object();
    const QString language = root.value(QStringLiteral("targetLanguage")).toString();
    QJsonArray tokens;
    for (const QJsonValue &value : root.value(QStringLiteral("tokens")).toArray()) {
        QJsonObject token = value.toObject();
        token.insert(QStringLiteral("text"),
                     QStringLiteral("[%1] %2").arg(language, token.value(QStringLiteral("text")).toString()));
        tokens.append(token);
    }
    if (tokens.isEmpty()) {
        return 3;
    }
    QTextStream(stdout) << QJsonDocument(QJsonObject{{QStringLiteral("tokens"), tokens}}).toJson(QJsonDocument::Compact);
    return 0;
}

/**
 * 【OCR测试】【固定结果】通过控制台进程输出可预测的识别或翻译结果
 * @param argc 参数数量
 * @param argv 参数列表，依次包含结果模式和图像或翻译输入文件路径
 * @return 参数或文件无效时返回非零状态
 */
int main(int argc, char **argv)
{
    QCoreApplication app(argc, argv);
    const QStringList args = app.arguments();
    if (args.size() != 3 || !QFileInfo::exists(args.at(2))) {
        QTextStream(stderr) << "Expected OCR mode and an existing image path\n";
        return 1;
    }
    if (args.at(1) == QStringLiteral("--translate-fixture")) {
        return writeFixtureTranslation(args.at(2));
    }
    if (args.at(1) == QStringLiteral("--ocr-empty")) {
        QTextStream(stdout) << "[]";
        return 0;
    }
    if (args.at(1) != QStringLiteral("--ocr-fixture")) {
        return 2;
    }
    QTextStream(stdout) << R"({"tokens":[{"text":"Hello","box":[40,40,60,40],"line":0,"index":0},{"text":"world","box":[120,40,60,40],"line":0,"index":1}]})";
    return 0;
}
