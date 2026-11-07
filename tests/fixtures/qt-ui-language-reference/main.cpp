/** 【界面语言对照】【真实原版】只读连接原版语言表和 OCR 窗口，使用临时配置及本地翻译命令，不读取用户凭据 */
#include "app_config_store.h"
#include "ocr_result_window/ocr_result_window.h"
#include "ocr_result_window/ocr_text_pane.h"
#include "ocr_table/ocr_table_pane.h"
#include "support/isolated_config.h"
#include "ui/i18n.h"
#include "ui/interface_language_config.h"
#include <QApplication>
#include <QComboBox>
#include <QDir>
#include <QFile>
#include <QJsonArray>
#include <QJsonDocument>
#include <QLabel>
#include <QPushButton>
#include <QTemporaryDir>
#include <QTabBar>
#include <QTextEdit>
#include <QtTest/QTest>
#include <iostream>
#include <stdexcept>

/** 【界面语言对照】【控件读取】读取 widget 相对 window 的矩形及文案，返回 JSON，不改变业务状态 */
static QJsonObject widgetState(QWidget &window, QWidget *widget)
{
    const QPoint point = widget->mapTo(&window, QPoint());
    QJsonObject value{{"rect", QJsonArray{point.x(), point.y(), widget->width(), widget->height()}},
        {"visible", widget->isVisible()}};
    if (auto *button = qobject_cast<QPushButton *>(widget)) value["text"] = button->text();
    if (auto *label = qobject_cast<QLabel *>(widget)) value["text"] = label->text();
    if (auto *editor = qobject_cast<QTextEdit *>(widget)) value["text"] = editor->toPlainText();
    return value;
}

/** 【界面语言对照】【窗口采集】读取 window 的真实复制、译文及目标语言控件，返回 JSON，不替换原版组件 */
static QJsonObject snapshot(markshot::shot::OcrResultWindow &window)
{
    QJsonObject widgets;
    for (const QString &name : {"ocrTranslationActions", "ocrLanguageCombo", "ocrTranslateButton",
        "ocrSourcePane", "ocrTranslationPane", "ocrTranslationEditor", "ocrEditor", "ocrTableCopyButton"}) {
        widgets[name] = widgetState(window, window.findChild<QWidget *>(name));
    }
    auto *title = window.findChild<QWidget *>("ocrTitleBar");
    widgets["source_copy"] = widgetState(window, title->findChild<QPushButton *>("ocrCopyButton"));
    auto *pane = window.findChild<markshot::shot::OcrTextPane *>("ocrTranslationPane");
    for (const QString &name : {"ocrCopyButton", "ocrPaneNotice", "ocrPaneHeader"}) {
        widgets["translation:" + name] = widgetState(window, pane->findChild<QWidget *>(name));
    }
    for (auto *label : window.findChild<QWidget *>("ocrTranslationActions")->findChildren<QLabel *>()) {
        if (label->text() == MS_TR("Target Language")) widgets["caption"] = widgetState(window, label);
    }
    for (auto *label : window.findChild<QWidget *>("ocrTablePane")->findChildren<QLabel *>(QString(), Qt::FindDirectChildrenOnly)) {
        if (label->text() == MS_TR("Table")) widgets["table:title"] = widgetState(window, label);
        if (label->property("role").toString() == "muted") widgets["table:summary"] = widgetState(window, label);
    }
    return widgets;
}

/** 【界面语言对照】【完整序列】按 width 创建真实窗口并记录复制反馈与翻译提示，返回状态，不访问外部服务 */
static QJsonArray scenario(int width)
{
    // 1. 【界面语言对照】【隔离环境】创建私有配置及真实表格数据，不读取外部翻译设置
    if (!markshot::writeAppConfigRoot({{"ui", QJsonObject{{"theme", "dark"}}},
        {"translation", QJsonObject{{"command", "sleep 5; printf '%s' '{\"tokens\":[{\"text\":\"business translation\"}]}' < {input}"},
            {"targetLanguage", "English"}, {"timeoutMs", 10000}}}})) throw std::runtime_error("private configuration failed");
    QImage image(2, 2, QImage::Format_ARGB32); image.fill(Qt::transparent);
    QVector<markshot::ocr::Token> tokens;
    for (int row = 0; row < 3; ++row) {
        for (int column = 0; column < 3; ++column) {
            tokens.append({QString::number(row * 3 + column), QRectF(38 + column * 220, 32 + row * 66, 30, 24), row, column, 1.0});
        }
    }
    markshot::shot::OcrResultWindow window("business Copy %1", nullptr, image, tokens);
    window.setAttribute(Qt::WA_DeleteOnClose, false);
    window.resize(width, 520); window.show(); QTest::qWait(25);
    QJsonArray states;
    const auto record = [&](const QString &name) { states.append(QJsonObject{{"name", name}, {"widgets", snapshot(window)}}); };
    record("initial");
    // 2. 【界面语言对照】【复制与翻译】记录实际反馈和本地任务取消后的语言几何
    auto *source = window.findChild<markshot::shot::OcrTextPane *>("ocrSourcePane");
    source->showCopyFeedback(true); QTest::qWait(10); record("copied");
    source->showCopyFeedback(false); QTest::qWait(10); record("copy-failed");
    window.findChild<QPushButton *>("ocrTranslationToggle")->click(); QTest::qWait(25); record("busy");
    window.findChild<QPushButton *>("ocrTranslateButton")->click(); QTest::qWait(25); record("canceled");
    // 3. 【界面语言对照】【表格反馈】切换真实页签并核对表格失败标签宽度
    window.findChild<QTabBar *>("ocrViewTabs")->setCurrentIndex(1); QTest::qWait(15); record("table");
    window.findChild<markshot::ocr_table::OcrTablePane *>("ocrTablePane")->showCopyFeedback(false); QTest::qWait(15); record("table-copy-failed");
    window.hide();
    return states;
}

/** 【界面语言对照】【入口】argc、argv 指定私有目录和文案键文件，返回状态；原版源码与用户环境保持不变 */
int main(int argc, char **argv)
{
    if (argc != 3) return 2;
    QTemporaryDir home;
    if (!markshot::test::isolateConfig(home)) return 3;
    for (const char *name : {"DISPLAY", "WAYLAND_DISPLAY", "NIRI_SOCKET", "SWAYSOCK", "DBUS_SESSION_BUS_ADDRESS"}) qunsetenv(name);
    qputenv("QT_QPA_PLATFORM", "offscreen");
    QApplication app(argc, argv);
    QApplication::setQuitOnLastWindowClosed(false);
    QFile input(QString::fromLocal8Bit(argv[2]));
    if (!input.open(QIODevice::ReadOnly)) return 4;
    const QJsonArray keys = QJsonDocument::fromJson(input.readAll()).array();
    QJsonArray languages;
    for (const auto language : {markshot::i18n::Language::English, markshot::i18n::Language::Chinese}) {
        markshot::i18n::setLanguage(language);
        QJsonObject catalog;
        for (const auto &key : keys) catalog[key.toString()] = markshot::i18n::translate(key.toString());
        QJsonArray cases;
        for (const int width : {380, 480, 619, 620, 760}) {
            cases.append(QJsonObject{{"width", width}, {"states", scenario(width)}});
        }
        languages.append(QJsonObject{{"name", language == markshot::i18n::Language::English ? "en" : "zh"},
            {"catalog", catalog}, {"cases", cases}});
    }
    const QString output = QString::fromLocal8Bit(argv[1]);
    QDir().mkpath(output);
    QFile result(output + "/result.json");
    if (!result.open(QIODevice::WriteOnly)) return 5;
    result.write(QJsonDocument(QJsonObject{{"languages", languages}}).toJson(QJsonDocument::Compact));
    return 0;
}
