#include "gemini_translate_config.h"
#include "gemini_translate_plugin.h"
#include "gemini_translate_request.h"

#include <QtTest/QtTest>

#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QTemporaryFile>
#include <QTcpServer>
#include <QTcpSocket>

using namespace markshot::translate_gemini;

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

class MockGeminiServer final : public QTcpServer {
public:
    explicit MockGeminiServer(QByteArray responseBody, int statusCode = 200)
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
        return QStringLiteral("http://127.0.0.1:%1/v1beta").arg(serverPort());
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

QTemporaryFile *writeTempConfig(const QString &endpoint, const QString &apiKey = QStringLiteral("test-gemini-key"))
{
    auto *file = new QTemporaryFile();
    if (!file->open()) {
        delete file;
        return nullptr;
    }
    QJsonObject gemini;
    gemini.insert(QStringLiteral("endpoint"), endpoint);
    gemini.insert(QStringLiteral("apiKey"), apiKey);
    gemini.insert(QStringLiteral("model"), QStringLiteral("gemini-3.5-flash-lite"));

    QJsonObject translation;
    translation.insert(QStringLiteral("gemini"), gemini);

    QJsonObject root;
    root.insert(QStringLiteral("translation"), translation);
    file->write(QJsonDocument(root).toJson());
    file->flush();
    return file;
}

QByteArray makeGeminiSuccessResponse(const QString &jsonContent)
{
    QJsonObject part;
    part.insert(QStringLiteral("text"), jsonContent);

    QJsonObject content;
    content.insert(QStringLiteral("parts"), QJsonArray{part});
    content.insert(QStringLiteral("role"), QStringLiteral("model"));

    QJsonObject candidate;
    candidate.insert(QStringLiteral("content"), content);
    candidate.insert(QStringLiteral("finishReason"), QStringLiteral("STOP"));

    QJsonObject root;
    root.insert(QStringLiteral("candidates"), QJsonArray{candidate});
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

class TranslateGeminiPluginTest final : public QObject {
    Q_OBJECT

private slots:
    void requiresApiKey()
    {
        GeminiTranslateConfig config;
        config.apiKey = QString();
        QString error;
        QVERIFY(!validateGeminiTranslateConfig(config, &error));
        QVERIFY(error.contains(QStringLiteral("missing gemini apiKey")));

        config.apiKey = QStringLiteral("valid_key");
        QVERIFY(validateGeminiTranslateConfig(config, &error));
    }

    void sendsCorrectHeadersAndPayload()
    {
        const QByteArray responseJson = makeGeminiSuccessResponse(
            QStringLiteral("{\"translations\":[{\"id\":1,\"text\":\"世界\"},{\"id\":2,\"text\":\"你好\"}]}"));
        MockGeminiServer server(responseJson);
        QVERIFY(server.start());

        std::unique_ptr<QTemporaryFile> configFile(writeTempConfig(server.endpoint(), QStringLiteral("my-secret-key")));
        QVERIFY(configFile != nullptr);
        EnvGuard configGuard(QByteArrayLiteral("MARK_SHOT_CONFIG"), configFile->fileName().toUtf8());

        GeminiTranslatePlugin plugin;
        QString error;
        QVERIFY(plugin.isAvailable(&error));

        QVector<markshot::plugin::TranslateSegment> translations;
        const QVector<markshot::plugin::TranslateSegment> segments = {
            {1, QStringLiteral("World")},
            {2, QStringLiteral("Hello")}
        };

        QVERIFY(plugin.translate(segments, QStringLiteral("Simplified Chinese"), &translations, &error));
        QCOMPARE(translations.size(), 2);
        QCOMPARE(translations.at(0).text, QStringLiteral("世界"));
        QCOMPARE(translations.at(1).text, QStringLiteral("你好"));

        // Verify request headers
        const QByteArray header = server.requestHeader();
        QCOMPARE(headerValue(header, "x-goog-api-key"), QByteArrayLiteral("my-secret-key"));

        // Verify request payload
        const QJsonDocument reqDoc = QJsonDocument::fromJson(server.requestBody());
        QVERIFY(reqDoc.isObject());
        const QJsonObject root = reqDoc.object();
        QVERIFY(root.contains(QStringLiteral("systemInstruction")));
        QVERIFY(root.contains(QStringLiteral("contents")));
        QVERIFY(root.contains(QStringLiteral("generationConfig")));
        QCOMPARE(root.value(QStringLiteral("generationConfig")).toObject().value(QStringLiteral("responseMimeType")).toString(),
                 QStringLiteral("application/json"));
        // 未显式配置时不下发 temperature 与 thinkingConfig，沿用模型默认值
        QVERIFY(!root.value(QStringLiteral("generationConfig")).toObject().contains(QStringLiteral("thinkingConfig")));
        QVERIFY(!root.value(QStringLiteral("generationConfig")).toObject().contains(QStringLiteral("temperature")));
    }

    void ignoresSharedOpenAiConfig()
    {
        // 公共 translation 节属于 OpenAI 兼容服务，不能被 Gemini 插件继承
        std::unique_ptr<QTemporaryFile> file(new QTemporaryFile());
        QVERIFY(file->open());
        QJsonObject translation;
        translation.insert(QStringLiteral("apiKey"), QStringLiteral("sk-openai"));
        translation.insert(QStringLiteral("model"), QStringLiteral("gpt-4o-mini"));
        file->write(QJsonDocument(QJsonObject{{QStringLiteral("translation"), translation}}).toJson());
        file->flush();
        EnvGuard configGuard(QByteArrayLiteral("MARK_SHOT_CONFIG"), file->fileName().toUtf8());
        EnvGuard keyGuard(QByteArrayLiteral("GEMINI_API_KEY"), QByteArray());
        EnvGuard markKeyGuard(QByteArrayLiteral("MARK_SHOT_GEMINI_API_KEY"), QByteArray());
        EnvGuard modelGuard(QByteArrayLiteral("GEMINI_MODEL"), QByteArray());
        EnvGuard markModelGuard(QByteArrayLiteral("MARK_SHOT_GEMINI_MODEL"), QByteArray());

        const GeminiTranslateConfig config = readGeminiTranslateConfig();
        QVERIFY(config.apiKey.isEmpty());
        QCOMPARE(config.model, QStringLiteral("gemini-3.5-flash-lite"));
    }

    void sendsThinkingLevelWhenConfigured()
    {
        GeminiTranslateConfig config;
        config.thinkingLevel = QStringLiteral("minimal");
        config.temperature = 0.4;
        const QJsonObject generation = QJsonDocument::fromJson(
            buildGeminiPayload(config, {{0, QStringLiteral("Hello")}}, QString()))
            .object().value(QStringLiteral("generationConfig")).toObject();
        QCOMPARE(generation.value(QStringLiteral("thinkingConfig")).toObject()
                     .value(QStringLiteral("thinkingLevel")).toString(),
                 QStringLiteral("MINIMAL"));
        QCOMPARE(generation.value(QStringLiteral("temperature")).toDouble(), 0.4);
    }

    void handlesApiErrorResponse()
    {
        QJsonObject errObj;
        errObj.insert(QStringLiteral("message"), QStringLiteral("API key expired"));
        QJsonObject root;
        root.insert(QStringLiteral("error"), errObj);

        MockGeminiServer server(QJsonDocument(root).toJson(), 400);
        QVERIFY(server.start());

        std::unique_ptr<QTemporaryFile> configFile(writeTempConfig(server.endpoint()));
        QVERIFY(configFile != nullptr);
        EnvGuard configGuard(QByteArrayLiteral("MARK_SHOT_CONFIG"), configFile->fileName().toUtf8());

        GeminiTranslatePlugin plugin;
        QVector<markshot::plugin::TranslateSegment> translations;
        QString error;
        QVERIFY(!plugin.translate({{1, QStringLiteral("Hello")}}, QStringLiteral("Simplified Chinese"), &translations, &error));
        QVERIFY(error.contains(QStringLiteral("API key expired")));
    }
};

QTEST_GUILESS_MAIN(TranslateGeminiPluginTest)

#include "translate_gemini_plugin_test.moc"
