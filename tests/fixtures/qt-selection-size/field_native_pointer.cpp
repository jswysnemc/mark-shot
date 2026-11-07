#include "field_probe.h"
#include "ui/i18n.h"

#include <QApplication>
#include <QClipboard>
#include <QFile>
#include <QFontInfo>
#include <QJsonDocument>
#include <QLocale>
#include <QMouseEvent>
#include <QTimer>

namespace markshot::i18n {
/** 【字段原生对照】【固定语言】按 source 返回英文标签，不读取用户配置 */
QString translate(const QString &source) { return source; }
}

class PointerLog final : public QObject {
public:
    QFile *file = nullptr;
    QFile *states = nullptr;

    /** 【字段原生对照】【平台事件】按 target 和 event 保存原版实际收到的按钮、时间与位置，返回继续分派 */
    bool eventFilter(QObject *target, QEvent *event) override
    {
        QString type;
        switch (event->type()) {
        case QEvent::MouseButtonPress: type = QStringLiteral("press"); break;
        case QEvent::MouseButtonDblClick: type = QStringLiteral("double"); break;
        case QEvent::MouseButtonRelease: type = QStringLiteral("release"); break;
        case QEvent::MouseMove: type = QStringLiteral("move"); break;
        default: return false;
        }
        const auto *mouse = static_cast<QMouseEvent *>(event);
        const QJsonObject value{{"type", type}, {"button", static_cast<int>(mouse->button())},
                                {"buttons", static_cast<int>(mouse->buttons())},
                                {"point", QJsonArray{mouse->position().x(), mouse->position().y()}},
                                {"timestamp", static_cast<qint64>(mouse->timestamp())}};
        file->write(QJsonDocument(value).toJson(QJsonDocument::Compact));
        file->write("\n");
        file->flush();
        // 1. 【字段原生对照】【分派后状态】直接记录控件正文与选择，补充仅锚点改变时输入法未通知的真实状态
        auto *editor = qobject_cast<QLineEdit *>(target);
        if (editor) {
            QTimer::singleShot(0, editor, [this, editor, value]() {
                const QJsonObject record{{"event", value}, {"state", state(*editor)}};
                states->write(QJsonDocument(record).toJson(QJsonDocument::Compact));
                states->write("\n");
                states->flush();
            });
        }
        return false;
    }
};

/** 【字段原生对照】【原生入口】按 argv 输出真实控件几何和事件，交互标记保持 Qt 事件循环，返回文件或应用退出状态 */
int main(int argc, char **argv)
{
    QApplication app(argc, argv);
    app.setStyle("Fusion");
    app.setApplicationName(QStringLiteral("mark-shot-field-reference"));
    app.setDesktopFileName(QStringLiteral("mark-shot-field-reference"));
    QLocale::setDefault(QLocale::c());
    QWidget host;
    host.setWindowTitle(QStringLiteral("Selection Field Reference"));
    aspect::SelectionSizeField field(QStringLiteral("Width"), &host);
    QLineEdit &editor = prepare(host, field, QStringLiteral("+1200"));
    field.move(12, 12);
    field.setValue(300, 1280);
    host.resize(800, 600);
    QApplication::processEvents();

    // 1. 【字段原生对照】【真实几何】仅保存客户区坐标，桌面位置由私有合成器提供
    const QPoint point = editor.mapTo(&host, QPoint());
    const QJsonObject value{{"qt", QString::fromLatin1(qVersion())}, {"font", QFontInfo(field.font()).family()},
                            {"dpi", field.logicalDpiX()}, {"text", editor.text()},
                            {"field_rect", QJsonArray{point.x(), point.y(), editor.width(), editor.height()}},
                            {"primary_supported", QApplication::clipboard()->supportsSelection()}};
    QFile metadata(QString::fromLocal8Bit(argv[1]));
    if (!metadata.open(QIODevice::WriteOnly)) { return 1; }
    metadata.write(QJsonDocument(value).toJson(QJsonDocument::Compact));
    metadata.write("\n");
    metadata.close();
    if (argc < 3 || QString::fromLocal8Bit(argv[2]) != QStringLiteral("--interactive")) { return 0; }

    // 2. 【字段原生对照】【事件追踪】真实输入来自独立 Wayland 会话，不注入直接构造的双击事件
    QFile events(QString::fromLocal8Bit(argv[1]) + QStringLiteral(".events"));
    if (!events.open(QIODevice::WriteOnly)) { return 1; }
    PointerLog log;
    QFile states(QString::fromLocal8Bit(argv[1]) + QStringLiteral(".states"));
    if (!states.open(QIODevice::WriteOnly)) { return 1; }
    log.file = &events;
    log.states = &states;
    editor.installEventFilter(&log);
    return app.exec();
}
