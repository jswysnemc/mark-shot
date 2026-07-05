#include "openai_translate_config.h"
#include "openai_translate_plugin.h"
#include "openai_translation_parser.h"

#include <QtTest/QtTest>

#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QTemporaryFile>
#include <QTcpServer>
#include <QTcpSocket>

using namespace markshot::translate_openai;

namespace {

class EnvGuard {
public:
    /**
     * 保存并设置环境变量。
     * @param name 环境变量名。
     * @param value 临时变量值。
     */
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

class MockChatServer final : public QTcpServer {
public:
    /**
     * 创建 mock chat/completions 服务。
     * @param responseBody 固定响应体。
     */
    explicit MockChatServer(QByteArray responseBody)
        : m_responseBody(std::move(responseBody))
    {
        connect(this, &QTcpServer::newConnection, this, [this] { handleConnection(); });
    }

    /**
     * 启动本地监听。
     * @return 启动成功时返回 true。
     */
    bool start()
    {
        return listen(QHostAddress::LocalHost, 0);
    }

    /**
     * 读取 API 根地址。
     * @return mock 服务地址。
     */
    QString baseUrl() const
    {
        return QStringLiteral("http://127.0.0.1:%1/v1").arg(serverPort());
    }

    /**
     * 读取收到的请求体。
     * @return HTTP 请求体。
     */
    QByteArray requestBody() const { return m_requestBody; }

private:
    /**
     * 处理新连接。
     * @return 无返回值。
     */
    void handleConnection()
    {
        QTcpSocket *socket = nextPendingConnection();
        connect(socket, &QTcpSocket::readyRead, this, [this, socket] {
            m_request += socket->readAll();
            const int headerEnd = m_request.indexOf("\r\n\r\n");
            if (headerEnd < 0) {
                return;
            }
            const QByteArray header = m_request.left(headerEnd);
            const QByteArray marker = QByteArrayLiteral("Content-Length:");
            const int lengthPos = header.indexOf(marker);
            int contentLength = 0;
            if (lengthPos >= 0) {
                const int lineEnd = header.indexOf('\n', lengthPos);
                contentLength = header.mid(lengthPos + marker.size(),
                                           lineEnd < 0 ? -1 : lineEnd - lengthPos - marker.size())
                                    .trimmed()
                                    .toInt();
            }
            if (m_request.size() < headerEnd + 4 + contentLength) {
                return;
            }
            m_requestBody = m_request.mid(headerEnd + 4, contentLength);
            const QByteArray response = QByteArrayLiteral("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n")
                + "Content-Length: " + QByteArray::number(m_responseBody.size())
                + QByteArrayLiteral("\r\nConnection: close\r\n\r\n") + m_responseBody;
            socket->write(response);
            socket->disconnectFromHost();
        });
    }

    QByteArray m_responseBody;
    QByteArray m_request;
    QByteArray m_requestBody;
};

/**
 * 写入临时 Mark Shot 配置。
 * @param apiBase API 根地址。
 * @return 临时配置文件。
 */
QTemporaryFile *writeTempConfig(const QString &apiBase)
{
    auto *file = new QTemporaryFile();
    if (!file->open()) {
        delete file;
        return nullptr;
    }
    QJsonObject translation;
    translation.insert(QStringLiteral("apiBase"), apiBase);
    translation.insert(QStringLiteral("apiKey"), QStringLiteral("test-key"));
    translation.insert(QStringLiteral("model"), QStringLiteral("test-model"));
    translation.insert(QStringLiteral("temperature"), 0.0);
    translation.insert(QStringLiteral("timeoutMs"), 5000);
    QJsonObject root;
    root.insert(QStringLiteral("translation"), translation);
    file->write(QJsonDocument(root).toJson(QJsonDocument::Compact));
    file->flush();
    return file;
}

}  // namespace

class TranslateOpenAiPluginTest : public QObject {
    Q_OBJECT

private slots:
    /**
     * 验证 Markdown 包裹的翻译 JSON 可被解析。
     * @return 无返回值。
     */
    void parsesMarkdownWrappedContent()
    {
        QHash<int, QString> translations;
        QString error;
        QVERIFY2(parseTranslationContent(QStringLiteral("```json\n{\"translations\":[{\"id\":4,\"text\":\"你好\"}]}\n```"),
                                         &translations,
                                         &error),
                 qPrintable(error));
        QCOMPARE(translations.value(4), QStringLiteral("你好"));
    }

    /**
     * 验证插件可调用 OpenAI-compatible chat/completions 接口。
     * @return 无返回值。
     */
    void translatesViaMockServer()
    {
        QJsonObject message;
        message.insert(QStringLiteral("content"),
                       QStringLiteral("{\"translations\":[{\"id\":7,\"text\":\"你好\"}]}"));
        QJsonObject choice;
        choice.insert(QStringLiteral("message"), message);
        QJsonObject root;
        root.insert(QStringLiteral("choices"), QJsonArray{choice});
        MockChatServer server(QJsonDocument(root).toJson(QJsonDocument::Compact));
        QVERIFY(server.start());

        std::unique_ptr<QTemporaryFile> config(writeTempConfig(server.baseUrl()));
        QVERIFY(config != nullptr);
        EnvGuard configGuard(QByteArrayLiteral("MARK_SHOT_CONFIG"), config->fileName().toUtf8());

        OpenAiTranslatePlugin plugin;
        QString error;
        QVERIFY2(plugin.isAvailable(&error), qPrintable(error));

        QVector<markshot::plugin::TranslateSegment> translations;
        QVERIFY2(plugin.translate({{7, QStringLiteral("hello")}},
                                  QStringLiteral("Simplified Chinese"),
                                  &translations,
                                  &error),
                 qPrintable(error));
        QCOMPARE(translations.size(), 1);
        QCOMPARE(translations.first().id, 7);
        QCOMPARE(translations.first().text, QStringLiteral("你好"));
        QVERIFY(server.requestBody().contains("\"model\":\"test-model\""));
    }
};

QTEST_GUILESS_MAIN(TranslateOpenAiPluginTest)
#include "translate_openai_plugin_test.moc"
