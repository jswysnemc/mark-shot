// 【上传对照】【独立探针】完整生产函数插入后执行原版配置、命令和结果解析，外围只提供私有文件及复制接收端
#include <QApplication>
#include <QCloseEvent>
#include <QEventLoop>
#include <QFile>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QProcess>
#include <QProcessEnvironment>
#include <QScopedValueRollback>
#include <QScopeGuard>
#include <QTemporaryFile>
#include <QTimer>
#include <QUrl>
#include <QWidget>
#include <algorithm>
#include <optional>
#include "shell_command.h"
#define MS_TR(value) QStringLiteral(value)

static QJsonObject configuration;
static QString copied;
static QString path;
static bool copyAllowed = true;
namespace markshot::rust {
/** 【上传对照】【配置接收】返回本次配置对象，参数路径不连接用户文件，返回完整对象 */
std::optional<QJsonObject> readConfigObject(const QString &) { return configuration; }
}
namespace markshot {
/** 【上传对照】【剪贴板接收】记录原版实际交付文本，text 为 URL，返回本次配置的复制结果 */
bool copyTextToClipboard(const QString &text) { copied = text; return copyAllowed; }
}
/** 【上传对照】【配置路径】返回隔离探针路径，无参数，不读取用户配置 */
QString appConfigPath() { return QString(); }

inline constexpr int kUploadTimeoutMs = 60000;
/* PRODUCTION_CONFIG_TYPE */
namespace cfg {
/* PRODUCTION_OBJECT_READER */
}
/* PRODUCTION_CONFIG */
/* PRODUCTION_QUOTING */

class ShotWindow : public QWidget {
public:
    bool m_operationBusy = false;
    bool closed = false;
    QString toast;
    /** 【上传对照】【正文边界】提供本次空正文提交，无参数、无返回值 */
    void commitTextEditor() {}
    /** 【上传对照】【选区资格】返回本次固定有效选区，无参数 */
    bool hasUsableSelection() const { return true; }
    /** 【上传对照】【图片暂存】创建本次私有 PNG 输入，参数沿用生产导出开关，返回路径 */
    QString saveSelectionToTempFile(bool) {
        QTemporaryFile file;
        file.setAutoRemove(false);
        if (!file.open()) { return QString(); }
        file.write("isolated-upload-image");
        path = file.fileName();
        return path;
    }
    /** 【上传对照】【光标边界】保留生产忙碌守卫调用，无参数、无返回值 */
    void updateCursor() {}
    /** 【上传对照】【提示接收】记录 text 对应原版提示，无返回值 */
    void showToast(const QString &text) { toast = text; }
    /** 【上传对照】【通知边界】模拟未提供通知服务，生产函数应进入 Toast 回退，返回失败 */
    bool sendDesktopNotification(const QString &, const QString &, int) { return false; }
    /** 【上传对照】【辅助入口】返回本次明确不存在的辅助程序，参数用于保持生产接口 */
    QString helperProgramPath(const QString &) { return QStringLiteral("/absent/isolated-upload-helper"); }
    void uploadSelection();
protected:
    /** 【上传对照】【关闭事实】记录生产延迟关闭，event 为本次关闭事件，无返回值 */
    void closeEvent(QCloseEvent *event) override { closed = true; event->accept(); }
};
/* PRODUCTION_UPLOAD */

/** 【上传对照】【命令入口】在独立 Qt 进程执行配置与上传场景，argc、argv 指定接收端，返回执行状态 */
int main(int argc, char **argv) {
    QApplication app(argc, argv);
    app.setQuitOnLastWindowClosed(false);
    qunsetenv("MARK_SHOT_UPLOAD_COMMAND");
    qunsetenv("MARK_SHOT_UPLOAD_TIMEOUT_MS");
    QJsonArray configs;
    const QJsonArray cases = QJsonDocument::fromJson(QByteArray(R"([
        {}, {"upload":{}}, {"imageUpload":{"path":"  receiver {image}  ","timeoutMs":1}},
        {"uploader":{"helper":"receiver","variables":{" TOKEN ":"value","invalid":7}}},
        {"imageHost":{"command":"","path":"ignored"}},
        {"upload":{"command":false,"path":"fallback","timeoutMs":2500}},
        {"upload":{},"imageUpload":{"command":"ignored"}},
        {"upload":{"environment":{"A":"value"},"env":{}}},
        {"upload":{"timeoutMs":2000.0}}, {"upload":{"timeoutMs":2000.5}},
        {"upload":{"timeoutMs":2147483648}}, {"upload":{"timeoutMs":-2.0}}
    ])")).array();
    for (const QJsonValue &root : cases) {
        configuration = root.toObject();
        const UploadConfig config = uploadConfig();
        QJsonObject env;
        for (const QString &key : {QStringLiteral("TOKEN"), QStringLiteral("A")}) {
            if (config.env.contains(key)) { env.insert(key, config.env.value(key)); }
        }
        configs.append(QJsonObject{{"input", root}, {"command", config.command}, {"timeout_ms", config.timeoutMs}, {"env",env}});
    }
    QJsonArray results;
    const QString receiver = QString::fromLocal8Bit(argv[1]);
    for (const QString &kind : {QStringLiteral("json"),QStringLiteral("plain"),QStringLiteral("empty"),QStringLiteral("failed"),QStringLiteral("copy-failed"),QStringLiteral("timeout"),QStringLiteral("json-local"),QStringLiteral("plain-prefix")}) {
        const QString command = QStringLiteral("/usr/bin/python3 ") + shellQuote(receiver) + QLatin1Char(' ') + shellQuote(kind) + QStringLiteral(" {image}");
        configuration = QJsonObject{{"upload",QJsonObject{{"command",command},{"timeoutMs",1000}}}};
        copied.clear();
        copyAllowed = kind != QStringLiteral("copy-failed");
        ShotWindow window;
        window.show();
        window.uploadSelection();
        QEventLoop loop;
        QTimer::singleShot(180, &loop, &QEventLoop::quit);
        loop.exec();
        results.append(QJsonObject{{"kind",kind},{"url",copied},{"toast",window.toast},{"closed",window.closed},{"busy",window.m_operationBusy},{"input_removed",!QFile::exists(path)}});
    }
    const QJsonObject result{{"qt",QString::fromLatin1(qVersion())},{"configs",configs},{"results",results}};
    QFile output(QString::fromLocal8Bit(argv[2]));
    if (!output.open(QIODevice::WriteOnly)) { return 1; }
    output.write(QJsonDocument(result).toJson());
    return 0;
}
