/** 【OCR 滚动对照】【真实协议】在私有 Wayland 中读取原版实际 QWheelEvent 和最终偏移，不使用手工角度转换 */
#include "app_config_store.h"
#include "ocr_result_window/ocr_result_window.h"
#include "settings/application_theme.h"
#include "support/isolated_config.h"
#include "ui/i18n.h"
#include "ui/theme.h"
#include <QApplication>
#include <QDir>
#include <QFile>
#include <QHeaderView>
#include <QJsonArray>
#include <QJsonDocument>
#include <QScrollBar>
#include <QTabBar>
#include <QTableWidget>
#include <QTemporaryDir>
#include <QTimer>
#include <QWheelEvent>
#include <iostream>

class WheelObserver : public QObject {
public:
    /** 【OCR 滚动对照】【观察创建】用 table、output 创建原版事件观察器，返回对象，不改变滚动事件 */
    WheelObserver(QTableWidget *table, QString output) : m_table(table), m_output(std::move(output)) {}

protected:
    /** 【OCR 滚动对照】【协议事件】读取 watched 的 event 后异步记录原版处理结果，返回 false 继续交付实际控件 */
    bool eventFilter(QObject *watched, QEvent *event) override
    {
        if (event->type() == QEvent::Wheel) {
            auto *wheel = static_cast<QWheelEvent *>(event);
            QJsonObject input{{"angle", QJsonArray{wheel->angleDelta().x(), wheel->angleDelta().y()}},
                {"pixel", QJsonArray{wheel->pixelDelta().x(), wheel->pixelDelta().y()}},
                {"phase", int(wheel->phase())}, {"mods", int(wheel->modifiers())},
                {"source", int(wheel->source())}, {"inverted", wheel->inverted()}};
            QTimer::singleShot(0, this, [this, input] {
                QJsonObject state = input;
                state["offset"] = QJsonArray{m_table->horizontalHeader()->offset(), m_table->verticalHeader()->offset()};
                state["values"] = QJsonArray{m_table->horizontalScrollBar()->value(), m_table->verticalScrollBar()->value()};
                m_events.append(state);
                QFile file(m_output + "/events.json");
                if (file.open(QIODevice::WriteOnly)) file.write(QJsonDocument(m_events).toJson(QJsonDocument::Compact));
            });
        }
        return QObject::eventFilter(watched, event);
    }

private:
    QTableWidget *m_table;
    QString m_output;
    QJsonArray m_events;
};

/** 【OCR 滚动对照】【私有入口】argc、argv 指定输出目录，返回状态；保留本次私有 Wayland，仅替换用户配置路径 */
int main(int argc, char **argv)
{
    if (argc != 2) return 2;
    const QByteArray runtime = qgetenv("XDG_RUNTIME_DIR");
    QTemporaryDir home;
    if (!markshot::test::isolateConfig(home)) return 3;
    qputenv("XDG_RUNTIME_DIR", runtime);
    QApplication app(argc, argv);
    QApplication::setDesktopFileName("mark-shot-ocr-result");
    QFont font = app.font(); font.setFamily(markshot::theme::uiFontFamily()); app.setFont(font);
    markshot::writeAppConfigRoot({{"ui", QJsonObject{{"theme", "dark"}}}});
    markshot::settings::applyConfiguredApplicationTheme();
    markshot::i18n::setLanguage(markshot::i18n::Language::English);
    QVector<markshot::ocr::Token> tokens;
    for (int row = 0; row < 40; ++row) {
        for (int column = 0; column < 8; ++column) tokens.append({"A long column value " + QString::number(row * 8 + column), QRectF(38 + column * 3000, 32 + row * 66, 40, 24), row, column, 1.0});
    }
    QImage image(2, 2, QImage::Format_ARGB32); image.fill(Qt::transparent);
    markshot::shot::OcrResultWindow window("business source", nullptr, image, tokens);
    window.setAttribute(Qt::WA_DeleteOnClose, false);
    window.resize(760, 460); window.show();
    window.findChild<QTabBar *>("ocrViewTabs")->setCurrentIndex(1);
    const QString output = QString::fromLocal8Bit(argv[1]); QDir().mkpath(output);
    auto *table = window.findChild<QTableWidget *>("ocrTable");
    WheelObserver observer(table, output);
    table->viewport()->installEventFilter(&observer);
    QTimer::singleShot(100, [] { std::cout << std::endl; });
    return app.exec();
}
