#include "anthropic_translate_config.h"
#include "anthropic_translate_plugin.h"
#include "anthropic_translate_request.h"

#include <QtTest/QtTest>

#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QTemporaryFile>
#include <QTcpServer>
#include <QTcpSocket>

using namespace markshot::translate_anthropic;

namespace {

class EnvGuard {
public:
    EnvGuard(QByteArray name, QByteArray value)
        : m_name(std::move(name))
        , m_hadValue(qEnvironmentVariableIsSet(m_name.constData()))
        , m_oldValue(qgetenv(m_name.constData()))
    {
        qputenv(m_name.constData(), value);
    }

    ~EnvGuard()
    {
        if (m_hadValue) {
            qputenv(m_name.constData(), m_oldValue);
        } else {
            qunsetenv(m_name.constData());
        }
    }

private:
    QByteArray m_name;
    bool m_hadValue = false;
    QByteArray m_oldValue;
};

class MockAnthropicServer final : public QTcpServer {
public:
    explicit MockAnthropicServer(QByteArray responseBody, int statusCode = 200)
        : m_responseBody(std::move(responseBody))
        , m_statusCode(statusCode)
    {
        connect(this, &QTcpServer::newConnection, this, [this] { handleConnection(); });
    }

    bool start()
    {
        return listen(QHostAddress::LocalHost, 0);
    }

    QString endpoint() const
    {
        return QStringLiteral("http://127.0.0.1:%1/v1").arg(serverPort());
    }

    QByteArray requestHeader() const { return m_requestHeader; }
    QByteArray requestBody() const { return m_requestBody; }

private:
    void handleConnection()
    {
        QTcpSocket *socket = nextPendingConnection();
        connect(socket, &QTcpSocket::readyRead, this, [this, socket] {
            m_request += socket->readAll();
            const int headerEnd = m_request.indexOf("\r\n\r\n");
            if (headerEnd < 0) {
                return;
            }
            m_requestHeader = m_request.left(headerEnd);
            const QByteArray marker = QByteArrayLiteral("Content-Length:");
            const int lengthPos = m_requestHeader.indexOf(marker);
            int contentLength = 0;
            if (lengthPos >= 0) {
                const int lineEnd = m_requestHeader.indexOf('\n', lengthPos);
                contentLength = m_requestHeader.mid(lengthPos + marker.size(),
                                                   lineEnd < 0 ? -1 : lineEnd - lengthPos - marker.size())
                                    .trimmed()
                                    .toInt();
            }
            if (m_request.size() < headerEnd + 4 + contentLength) {
                return;
            }
            m_requestBody = m_request.mid(headerEnd + 4, contentLength);
            const QByteArray statusLine = m_statusCode == 200
                ? QByteArrayLiteral("HTTP/1.1 200 OK\r\n")
                : QByteArrayLiteral("HTTP/1.1 400 Bad Request\r\n");
            const QByteArray response = statusLine + "Content-Type: application/json\r\n"
                + "Content-Length: " + QByteArray::number(m_responseBody.size())
                + QByteArrayLiteral("\r\nConnection: close\r\n\r\n") + m_responseBody;
            socket->write(response);
            socket->disconnectFromHost();
        });
    }

    QByteArray m_responseBody;
    int m_statusCode = 200;
    QByteArray m_request;
    QByteArray m_requestHeader;
    QByteArray m_requestBody;
};

QTemporaryFile *writeTempConfig(const QString &endpoint, const QString &apiKey = QStringLiteral("test-claude-key"))
{
    auto *file = new QTemporaryFile();
    if (!file->open()) {
        delete file;
        return nullptr;
    }
    QJsonObject anthropic;
    anthropic.insert(QStringLiteral("endpoint"), endpoint);
    anthropic.insert(QStringLiteral("apiKey"), apiKey);
    anthropic.insert(QStringLiteral("model"), QStringLiteral("claude-haiku-4-5"));

    QJsonObject translation;
    translation.insert(QStringLiteral("anthropic"), anthropic);

    QJsonObject root;
    root.insert(QStringLiteral("translation"), translation);
    file->write(QJsonDocument(root).toJson());
    file->flush();
    return file;
}

QByteArray makeAnthropicSuccessResponse(const QString &jsonContent)
{
    QJsonObject contentItem;
    contentItem.insert(QStringLiteral("type"), QStringLiteral("text"));
    contentItem.insert(QStringLiteral("text"), jsonContent);

    QJsonObject root;
    root.insert(QStringLiteral("id"), QStringLiteral("msg_123"));
    root.insert(QStringLiteral("type"), QStringLiteral("message"));
    root.insert(QStringLiteral("role"), QStringLiteral("assistant"));
    root.insert(QStringLiteral("content"), QJsonArray{contentItem});
    return QJsonDocument(root).toJson();
}

QByteArray headerValue(const QByteArray &header, const QByteArray &name)
{
    const QByteArray lowered = header.toLower();
    const QByteArray needle = name.toLower() + ":";
    int lineStart = 0;
    while (lineStart < lowered.size()) {
        const int newlinePos = lowered.indexOf('\n', lineStart);
        const int lineStop = newlinePos < 0 ? lowered.size() : newlinePos;
        if (lowered.mid(lineStart, lineStop - lineStart).startsWith(needle)) {
            return header.mid(lineStart + needle.size(), lineStop - lineStart - needle.size()).trimmed();
        }
        if (newlinePos < 0) {
            break;
        }
        lineStart = newlinePos + 1;
    }
    return {};
}

}  // namespace

class TranslateAnthropicPluginTest final : public QObject {
    Q_OBJECT

private slots:
    void requiresApiKey()
    {
        AnthropicTranslateConfig config;
        config.apiKey = QString();
        QString error;
        QVERIFY(!validateAnthropicTranslateConfig(config, &error));
        QVERIFY(error.contains(QStringLiteral("missing anthropic apiKey")));

        config.apiKey = QStringLiteral("valid_key");
        QVERIFY(validateAnthropicTranslateConfig(config, &error));
    }

    void sendsCorrectHeadersAndPayload()
    {
        const QByteArray responseJson = makeAnthropicSuccessResponse(
            QStringLiteral("{\"translations\":[{\"id\":10,\"text\":\"测试成功\"}]}"));
        MockAnthropicServer server(responseJson);
        QVERIFY(server.start());

        std::unique_ptr<QTemporaryFile> configFile(writeTempConfig(server.endpoint(), QStringLiteral("sk-ant-test")));
        QVERIFY(configFile != nullptr);
        EnvGuard configGuard(QByteArrayLiteral("MARK_SHOT_CONFIG"), configFile->fileName().toUtf8());

        AnthropicTranslatePlugin plugin;
        QString error;
        QVERIFY(plugin.isAvailable(&error));

        QVector<markshot::plugin::TranslateSegment> translations;
        const QVector<markshot::plugin::TranslateSegment> segments = {
            {10, QStringLiteral("Test Success")}
        };

        QVERIFY(plugin.translate(segments, QStringLiteral("Simplified Chinese"), &translations, &error));
        QCOMPARE(translations.size(), 1);
        QCOMPARE(translations.at(0).text, QStringLiteral("测试成功"));

        // Verify request headers
        const QByteArray header = server.requestHeader();
        QCOMPARE(headerValue(header, "x-api-key"), QByteArrayLiteral("sk-ant-test"));
        QCOMPARE(headerValue(header, "anthropic-version"), QByteArrayLiteral("2023-06-01"));

        // Verify request payload
        const QJsonDocument reqDoc = QJsonDocument::fromJson(server.requestBody());
        QVERIFY(reqDoc.isObject());
        const QJsonObject root = reqDoc.object();
        QCOMPARE(root.value(QStringLiteral("model")).toString(), QStringLiteral("claude-haiku-4-5"));
        QVERIFY(root.contains(QStringLiteral("system")));
        QVERIFY(root.contains(QStringLiteral("messages")));
        QCOMPARE(root.value(QStringLiteral("max_tokens")).toInt(), 4096);
        // 新模型拒绝 temperature，未显式配置时不能下发
        QVERIFY(!root.contains(QStringLiteral("temperature")));
    }

    void appendsVersionToBareEndpoint()
    {
        QCOMPARE(buildAnthropicUrl(QStringLiteral("https://api.anthropic.com")),
                 QUrl(QStringLiteral("https://api.anthropic.com/v1/messages")));
        QCOMPARE(buildAnthropicUrl(QStringLiteral("https://api.anthropic.com/v1/")),
                 QUrl(QStringLiteral("https://api.anthropic.com/v1/messages")));
        QCOMPARE(buildAnthropicUrl(QStringLiteral("https://proxy.example/v1/messages")),
                 QUrl(QStringLiteral("https://proxy.example/v1/messages")));
    }

    void handlesApiErrorResponse()
    {
        QJsonObject errDetail;
        errDetail.insert(QStringLiteral("type"), QStringLiteral("authentication_error"));
        errDetail.insert(QStringLiteral("message"), QStringLiteral("invalid x-api-key"));

        QJsonObject root;
        root.insert(QStringLiteral("type"), QStringLiteral("error"));
        root.insert(QStringLiteral("error"), errDetail);

        MockAnthropicServer server(QJsonDocument(root).toJson(), 400);
        QVERIFY(server.start());

        std::unique_ptr<QTemporaryFile> configFile(writeTempConfig(server.endpoint()));
        QVERIFY(configFile != nullptr);
        EnvGuard configGuard(QByteArrayLiteral("MARK_SHOT_CONFIG"), configFile->fileName().toUtf8());

        AnthropicTranslatePlugin plugin;
        QVector<markshot::plugin::TranslateSegment> translations;
        QString error;
        QVERIFY(!plugin.translate({{1, QStringLiteral("Hello")}}, QStringLiteral("Simplified Chinese"), &translations, &error));
        QVERIFY(error.contains(QStringLiteral("invalid x-api-key")));
    }
};

QTEST_GUILESS_MAIN(TranslateAnthropicPluginTest)

#include "translate_anthropic_plugin_test.moc"
