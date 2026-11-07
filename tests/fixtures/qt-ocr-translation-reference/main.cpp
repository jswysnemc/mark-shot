/** 【OCR 翻译对照】【完整窗口】只读链接原版窗口，使用固定本地命令观察分栏、取消和原文隔离，不读取服务凭据 */
#include "app_config_store.h"
#include "ocr_result_window/ocr_result_window.h"
#include "ocr_result_window/ocr_text_pane.h"
#include "support/isolated_config.h"
#include "ui/i18n.h"
#include <QApplication>
#include <QAbstractItemView>
#include <QComboBox>
#include <QDir>
#include <QFontInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QLabel>
#include <QLineEdit>
#include <QProgressBar>
#include <QPushButton>
#include <QSplitter>
#include <QScrollBar>
#include <QTabBar>
#include <QTemporaryDir>
#include <QTextEdit>
#include <QtTest/QTest>
#include <iostream>
#include <stdexcept>

/** 【OCR 翻译对照】【控件记录】读取 widget 相对 window 的实际几何和状态，返回 JSON，不改变控件布局 */
static QJsonObject widgetState(QWidget &window, QWidget *widget)
{
    const QPoint point = widget->mapTo(&window, QPoint());
    QJsonObject value{{"rect", QJsonArray{point.x(), point.y(), widget->width(), widget->height()}},
        {"visible", widget->isVisible()}, {"enabled", widget->isEnabled()},
        {"font", QFontInfo(widget->font()).family()}, {"pixels", QFontInfo(widget->font()).pixelSize()},
        {"font_height", widget->fontMetrics().height()}, {"focus_policy", int(widget->focusPolicy())},
        {"minimum", QJsonArray{widget->minimumWidth(), widget->minimumHeight()}}};
    if (auto *editor = qobject_cast<QTextEdit *>(widget)) {
        value["text"] = editor->toPlainText(); value["read_only"] = editor->isReadOnly(); value["placeholder"] = editor->placeholderText();
    }
    if (auto *button = qobject_cast<QPushButton *>(widget)) { value["text"] = button->text(); value["checked"] = button->isChecked(); }
    if (auto *label = qobject_cast<QLabel *>(widget)) { value["text"] = label->text(); value["error"] = label->property("error").toBool(); }
    if (auto *combo = qobject_cast<QComboBox *>(widget)) { value["text"] = combo->currentText(); value["index"] = combo->currentIndex(); }
    return value;
}

/** 【OCR 翻译对照】【窗口状态】按 window 读取实际原文、译文、操作行、分栏及页脚，返回 JSON，不创建替代组件 */
static QJsonObject snapshot(markshot::shot::OcrResultWindow &window)
{
    QJsonObject widgets;
    for (const QString &name : {"ocrTitleBar", "ocrTranslationToggle", "ocrTranslationActions", "ocrLanguageCombo",
        "ocrTranslateButton", "ocrContentViews", "ocrTextSplitter", "ocrSourcePane", "ocrTranslationPane",
        "ocrEditor", "ocrTranslationEditor", "ocrStatus", "ocrStatisticsBar", "ocrResizeGrip"}) {
        if (auto *widget = window.findChild<QWidget *>(name)) widgets[name] = widgetState(window, widget);
    }
    auto *splitter = window.findChild<QSplitter *>("ocrTextSplitter");
    auto *source = window.findChild<markshot::shot::OcrTextPane *>("ocrSourcePane");
    auto *translation = window.findChild<markshot::shot::OcrTextPane *>("ocrTranslationPane");
    auto *combo = window.findChild<QComboBox *>("ocrLanguageCombo");
    for (const auto &entry : {qMakePair(QStringLiteral("source"), source), qMakePair(QStringLiteral("translation"), translation)}) {
        for (const QString &name : {"ocrPaneHeader", "ocrUndoButton", "ocrCopyButton", "ocrPaneNotice", "ocrTranslationProgress"}) {
            if (auto *widget = entry.second->findChild<QWidget *>(name)) widgets[entry.first + ":" + name] = widgetState(window, widget);
        }
        widgets[entry.first + ":viewport"] = widgetState(window, entry.second->editor()->viewport());
    }
    widgets["language:line_edit"] = widgetState(window, combo->lineEdit());
    for (auto *label : window.findChild<QWidget *>("ocrTranslationActions")->findChildren<QLabel *>()) {
        if (label->text() == "Target Language") widgets["language:caption"] = widgetState(window, label);
    }
    QJsonArray sizes;
    for (int size : splitter->sizes()) sizes.append(size);
    QJsonArray options;
    for (int index = 0; index < combo->count(); ++index) options.append(QJsonObject{{"label", combo->itemText(index)}, {"value", combo->itemData(index).toString()}});
    return {{"size", QJsonArray{window.width(), window.height()}}, {"widgets", widgets},
        {"orientation", splitter->orientation() == Qt::Horizontal ? "horizontal" : "vertical"}, {"split_sizes", sizes}, {"options", options}};
}

/** 【OCR 翻译对照】【目标确认】向 combo 的真实编辑器交付 value 并确认，无返回值，使用原版持久化槽 */
static void language(QComboBox *combo, const QString &value)
{
    combo->setEditText(value);
    QMetaObject::invokeMethod(combo->lineEdit(), "editingFinished", Qt::DirectConnection);
    QApplication::processEvents();
}

/** 【OCR 翻译对照】【完整序列】按 width 和 output 创建完整原版窗口并执行实际按钮与编辑器序列，返回逐事件记录 */
static QJsonObject scenario(int width, const QString &output)
{
    const QString command = QStringLiteral("sleep 0.35; printf '%s' '{\"tokens\":[{\"text\":\"Translated first\"},{\"text\":\"Translated second\"}]}' < {input}");
    if (!markshot::writeAppConfigRoot({{"ui", QJsonObject{{"theme", "dark"}}},
        {"translation", QJsonObject{{"command", command}, {"targetLanguage", "English"}, {"timeoutMs", 1500}}}})) throw std::runtime_error("private config could not be written");
    QImage image(2, 2, QImage::Format_ARGB32); image.fill(Qt::transparent);
    markshot::shot::OcrResultWindow window("alpha\nbeta", nullptr, image, {});
    window.setAttribute(Qt::WA_DeleteOnClose, false);
    window.resize(width, 520); window.show(); window.activateWindow(); QTest::qWait(35);
    auto *toggle = window.findChild<QPushButton *>("ocrTranslationToggle");
    auto *translate = window.findChild<QPushButton *>("ocrTranslateButton");
    auto *source = window.findChild<QTextEdit *>("ocrEditor");
    auto *editor = window.findChild<QTextEdit *>("ocrTranslationEditor");
    auto *combo = window.findChild<QComboBox *>("ocrLanguageCombo");
    QJsonArray states;
    const auto record = [&](const QString &name) { states.append(QJsonObject{{"name", name}, {"state", snapshot(window)}}); };
    record("initial");
    // 1. 【OCR 翻译对照】【原文隔离】翻译期间原文可编辑，译文编辑暂停，取消保留确认正文
    toggle->click(); QTest::qWait(30); record("busy");
    source->moveCursor(QTextCursor::End); source->insertPlainText("!"); QTest::qWait(10); record("busy-source-edited");
    translate->click(); QTest::qWait(20); record("canceled");
    // 2. 【OCR 翻译对照】【完整结果】再次执行本地固定命令，只有结果任务改变译文
    translate->click();
    int attempts = 0;
    while (translate->text() == "Cancel" && attempts++ < 100) QTest::qWait(20);
    if (editor->toPlainText() != "Translated first\nTranslated second") throw std::runtime_error("original command translation did not complete");
    record("translated");
    if (width == 380 || width == 760) window.grab().save(output + "/translated-" + QString::number(width) + ".png");
    // 3. 【OCR 翻译对照】【过期提示】原文或自由目标变化只更新提示，不自动覆盖已有译文
    source->insertPlainText(" changed"); QTest::qWait(10); record("source-stale");
    language(combo, " Klingon "); QTest::qWait(10); record("language-stale");
    source->setPlainText("alpha\nbeta!"); language(combo, "English"); QTest::qWait(10); record("fresh-again");
    // 4. 【OCR 翻译对照】【译文历史】独立编辑和复制反馈不改变原文历史，收起再展开保留译文
    editor->setFocus(); editor->moveCursor(QTextCursor::End); editor->insertPlainText(" edited"); QTest::qWait(10); record("translation-edited");
    auto *pane = window.findChild<markshot::shot::OcrTextPane *>("ocrTranslationPane");
    pane->showCopyFeedback(true); QApplication::processEvents(); record("translation-copy-feedback");
    toggle->click(); QTest::qWait(10); record("hidden-kept-text");
    toggle->click(); QTest::qWait(10); record("reopened-without-request");
    // 5. 【OCR 翻译对照】【页面焦点】译文编辑后切到原图再返回，读取原版实际恢复的编辑器身份
    editor->setFocus(); QTest::qWait(10);
    auto *tabs = window.findChild<QTabBar *>("ocrViewTabs");
    tabs->setCurrentIndex(1); QTest::qWait(20);
    const bool hiddenOnImage = !editor->isVisible();
    tabs->setCurrentIndex(0); QTest::qWait(20);
    const QString restoredFocus = QApplication::focusWidget() ? QApplication::focusWidget()->objectName() : QString();
    // 6. 【OCR 翻译对照】【语言视区】原版最多显示八项，读取实际行高、列表框和滚动范围，不创建替代控件
    combo->showPopup(); QTest::qWait(20);
    auto *view = combo->view();
    QJsonArray rows;
    for (int index = 0; index < combo->count(); ++index) {
        const QRect rect = view->visualRect(combo->model()->index(index, 0));
        rows.append(QJsonArray{rect.x(), rect.y(), rect.width(), rect.height()});
    }
    const QPoint scrollbarPoint = view->verticalScrollBar()->mapTo(view, QPoint());
    const QJsonObject popup{{"size", QJsonArray{view->window()->width(), view->window()->height()}},
        {"view_size", QJsonArray{view->width(), view->height()}}, {"viewport", QJsonArray{view->viewport()->x(), view->viewport()->y(), view->viewport()->width(), view->viewport()->height()}},
        {"rows", rows}, {"font_pixels", QFontInfo(view->font()).pixelSize()}, {"font_family", QFontInfo(view->font()).family()}, {"font_height", view->fontMetrics().height()}, {"max_visible", combo->maxVisibleItems()},
        {"scrollbar", QJsonArray{scrollbarPoint.x(), scrollbarPoint.y(), view->verticalScrollBar()->width(), view->verticalScrollBar()->height()}},
        {"scroll_max", view->verticalScrollBar()->maximum()}, {"scroll_page", view->verticalScrollBar()->pageStep()}};
    combo->hidePopup();
    return {{"width", width}, {"states", states}, {"extra", QJsonObject{{"hidden_on_image", hiddenOnImage}, {"restored_focus", restoredFocus}, {"language_popup", popup}}}};
}

/** 【OCR 翻译对照】【入口】argc、argv 指定输出目录，返回成功状态；清除真实桌面和座席环境，所有配置写入临时目录 */
int main(int argc, char **argv)
{
    QTemporaryDir home;
    if (!markshot::test::isolateConfig(home)) return 2;
    for (const char *name : {"DISPLAY", "WAYLAND_DISPLAY", "NIRI_SOCKET", "SWAYSOCK", "XDG_SESSION_TYPE", "DBUS_SESSION_BUS_ADDRESS"}) qunsetenv(name);
    qputenv("QT_QPA_PLATFORM", "offscreen");
    QApplication application(argc, argv);
    QApplication::setQuitOnLastWindowClosed(false);
    markshot::i18n::setLanguage(markshot::i18n::Language::English);
    const QString output = argc > 1 ? QString::fromLocal8Bit(argv[1]) : home.path();
    QDir().mkpath(output);
    QJsonArray cases;
    try { for (int width : {380, 480, 500, 619, 620, 760}) cases.append(scenario(width, output)); }
    catch (const std::exception &error) { std::cerr << error.what(); return 3; }
    std::cout << QJsonDocument(QJsonObject{{"cases", cases}}).toJson(QJsonDocument::Compact).toStdString() << std::flush;
    return 0;
}
