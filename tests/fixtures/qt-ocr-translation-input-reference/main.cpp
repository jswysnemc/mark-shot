/** 【OCR 翻译输入对照】【实际控件】只读链接原版窗口，记录三个编辑器的输入法、历史和焦点，不访问真实服务 */
#include "app_config_store.h"
#include "ocr_result_window/ocr_result_window.h"
#include "ocr_result_window/ocr_text_pane.h"
#include "support/isolated_config.h"
#include "ui/i18n.h"
#include <QApplication>
#include <QComboBox>
#include <QDir>
#include <QFile>
#include <QInputMethodEvent>
#include <QJsonArray>
#include <QJsonDocument>
#include <QLabel>
#include <QLineEdit>
#include <QPushButton>
#include <QSaveFile>
#include <QTabBar>
#include <QTemporaryDir>
#include <QTextBlock>
#include <QTextEdit>
#include <QTextLayout>
#include <QTimer>
#include <QtTest/QTest>
#include <iostream>
#include <functional>
#include <stdexcept>

/** 【OCR 翻译输入对照】【字段快照】按 widget 读取实际输入资格、确认正文和 UTF-16 光标，返回 JSON，不重建替代控件 */
static QJsonObject field(QWidget *widget)
{
    const QRect rectangle = widget->inputMethodQuery(Qt::ImCursorRectangle).toRect();
    QJsonObject value{{"enabled", widget->isEnabled()}, {"visible", widget->isVisible()},
        {"input_enabled", widget->inputMethodQuery(Qt::ImEnabled).toBool()},
        {"multiline", bool(widget->inputMethodQuery(Qt::ImHints).toUInt() & Qt::ImhMultiLine)},
        {"surrounding", widget->inputMethodQuery(Qt::ImSurroundingText).toString()},
        {"cursor", widget->inputMethodQuery(Qt::ImCursorPosition).toInt()},
        {"anchor", widget->inputMethodQuery(Qt::ImAnchorPosition).toInt()},
        {"cursor_rect", QJsonArray{rectangle.x(), rectangle.y(), rectangle.width(), rectangle.height()}}};
    if (auto *editor = qobject_cast<QTextEdit *>(widget)) {
        value["text"] = editor->toPlainText();
        value["read_only"] = editor->isReadOnly();
        value["preedit"] = editor->textCursor().block().layout()->preeditAreaText();
        value["undo"] = editor->document()->isUndoAvailable();
        value["redo"] = editor->document()->isRedoAvailable();
    } else if (auto *editor = qobject_cast<QLineEdit *>(widget)) {
        value["text"] = editor->text();
        value["read_only"] = editor->isReadOnly();
        value["undo"] = editor->isUndoAvailable();
        value["redo"] = editor->isRedoAvailable();
    }
    return value;
}

/** 【OCR 翻译输入对照】【窗口快照】按 window 读取真实三个字段、目标保存值和焦点，返回 JSON，不请求外部服务 */
static QJsonObject snapshot(markshot::shot::OcrResultWindow &window)
{
    auto *combo = window.findChild<QComboBox *>("ocrLanguageCombo");
    auto *pane = window.findChild<markshot::shot::OcrTextPane *>("ocrTranslationPane");
    auto *button = window.findChild<QPushButton *>("ocrTranslateButton");
    const auto config = markshot::readAppConfigRoot().value("translation").toObject();
    return {{"source", field(window.findChild<QTextEdit *>("ocrEditor"))},
        {"translation", field(window.findChild<QTextEdit *>("ocrTranslationEditor"))},
        {"language", field(combo->lineEdit())}, {"language_enabled", combo->isEnabled()},
        {"target", combo->currentText()}, {"saved_target", config.value("targetLanguage").toString()},
        {"busy", button->text() == "Cancel"},
        {"notice", pane->findChild<QLabel *>("ocrPaneNotice")->text()},
        {"focus", QApplication::focusWidget() ? QApplication::focusWidget()->objectName() : QString()}};
}

/** 【OCR 翻译输入对照】【完整批次】向 widget 交付 preedit、commit 与 UTF-16 replacementStart/replacementLength，无返回值，事件经过原版分派 */
static void input(QWidget *widget, const QString &preedit, const QString &commit = {}, int start = 0, int length = 0)
{
    QList<QInputMethodEvent::Attribute> attributes;
    attributes.append({QInputMethodEvent::Cursor, int(preedit.size()), 1, {}});
    QInputMethodEvent event(preedit, attributes);
    if (!commit.isEmpty() || length) event.setCommitString(commit, start, length);
    QApplication::sendEvent(widget, &event);
    QApplication::processEvents();
}

/** 【OCR 翻译输入对照】【实际序列】按 width、output 执行原版编辑和焦点序列，返回逐步快照，所有配置和截图写入私有目录 */
static QJsonObject scenario(int width, const QString &output)
{
    const QString command = "sleep 0.6; printf '%s' '{\"tokens\":[{\"text\":\"completed\"}]}' < {input}";
    if (!markshot::writeAppConfigRoot({{"ui", QJsonObject{{"theme", "dark"}}},
        {"translation", QJsonObject{{"command", command}, {"targetLanguage", "English"}, {"timeoutMs", 1500}}}}))
        throw std::runtime_error("private config could not be written");
    QImage image(2, 2, QImage::Format_ARGB32); image.fill(Qt::transparent);
    markshot::shot::OcrResultWindow window("base", nullptr, image, {});
    window.setAttribute(Qt::WA_DeleteOnClose, false);
    window.resize(width, 520); window.show(); window.activateWindow(); QTest::qWait(30);
    auto *source = window.findChild<QTextEdit *>("ocrEditor");
    auto *editor = window.findChild<QTextEdit *>("ocrTranslationEditor");
    auto *combo = window.findChild<QComboBox *>("ocrLanguageCombo");
    auto *language = combo->lineEdit(); language->setObjectName("ocrLanguageEditor");
    auto *toggle = window.findChild<QPushButton *>("ocrTranslationToggle");
    auto *translate = window.findChild<QPushButton *>("ocrTranslateButton");
    auto *tabs = window.findChild<QTabBar *>("ocrViewTabs");
    editor->setPlainText("draft"); toggle->click(); editor->setFocus(); editor->moveCursor(QTextCursor::End);
    QJsonArray states;
    const auto record = [&](const QString &name) { states.append(QJsonObject{{"name", name}, {"state", snapshot(window)}}); };
    record("initial");
    // 1. 【OCR 翻译输入对照】【独立历史】译文逐次确认，普通文本和辅助平面码点分别检查，不修改原文
    input(editor, "ni"); record("translation-preedit");
    input(editor, "hao", "X"); record("translation-commit-x");
    input(editor, {}, "Y"); record("translation-commit-y");
    editor->undo(); record("translation-undo-y"); editor->undo(); record("translation-undo-x");
    QTest::keyClick(editor, Qt::Key_Y, Qt::ControlModifier); record("translation-ctrl-y-no-redo");
    QTest::keyClick(editor, Qt::Key_Z, Qt::ControlModifier | Qt::ShiftModifier); record("translation-redo-x");
    QTest::keyClick(editor, Qt::Key_Z, Qt::ControlModifier | Qt::ShiftModifier); record("translation-redo-y");
    input(editor, {}, QString::fromUtf8("𠮷")); record("translation-supplementary");
    input(editor, {}, QString::fromUtf8("你"), -2, 2); record("translation-replacement");
    // 2. 【OCR 翻译输入对照】【忙碌范围】原文快捷翻译保留候选，只读译文拒绝确认，取消恢复自己的历史
    source->setFocus(); source->moveCursor(QTextCursor::End); input(source, "source-candidate");
    record("source-preedit");
    QTest::keyClick(source, Qt::Key_Return, Qt::ControlModifier); QTest::qWait(10); record("busy-source-preedit");
    input(source, {}, "!"); record("busy-source-commit");
    source->undo(); record("source-undo");
    QTest::keyClick(source, Qt::Key_Y, Qt::ControlModifier); record("source-ctrl-y-no-redo");
    QTest::keyClick(source, Qt::Key_Z, Qt::ControlModifier | Qt::ShiftModifier); record("source-redo");
    editor->setFocus(); QTest::keyClicks(editor, "forbidden"); record("busy-translation-rejected");
    translate->click(); QTest::qWait(10); record("canceled");
    // 3. 【OCR 翻译输入对照】【页面焦点】带候选切换原图再恢复译文，确认正文和编辑历史继续独立
    editor->setFocus(); editor->moveCursor(QTextCursor::End); input(editor, "view-candidate"); record("before-image");
    tabs->setCurrentIndex(1); QTest::qWait(10); record("on-image");
    tabs->setCurrentIndex(0); QTest::qWait(10); record("restored-translation");
    input(editor, {});
    // 4. 【OCR 翻译输入对照】【自由目标】语言字段确认、保存和忙碌禁用分别记录，候选不作为请求目标
    language->setFocus(); language->selectAll(); input(language, "free"); record("language-preedit-selection");
    input(language, {}, "Esperanto"); record("language-commit");
    QTest::keyClick(language, Qt::Key_Return); record("language-return-saved");
    input(language, "target-candidate"); record("language-preedit-before-start");
    QTest::keyClick(language, Qt::Key_Return, Qt::ControlModifier); QTest::qWait(10); record("busy-from-language");
    translate->click(); QTest::qWait(10); record("language-task-canceled");
    language->setFocus(); input(language, {}); record("language-restored");
    if (width == 760) window.grab().save(output + "/language-restored.png");
    return {{"width", width}, {"states", states}};
}

/** 【OCR 翻译输入对照】【平台观察】输入、焦点和按钮事件结束后记录真实控件，window 指定私有窗口，save 指定无副作用的快照保存 */
class Observer final : public QObject {
public:
    Observer(QWidget *window, std::function<void()> save) : QObject(window), m_window(window), m_save(std::move(save)) {}
protected:
    /** 【OCR 翻译输入对照】【观察事件】watched 为实际事件对象，event 为平台事件，返回 false 保留原版分派，保存延后到事件完成 */
    bool eventFilter(QObject *watched, QEvent *event) override
    {
        Q_UNUSED(watched);
        switch (event->type()) {
        case QEvent::InputMethod: case QEvent::FocusIn: case QEvent::FocusOut:
        case QEvent::MouseButtonRelease: case QEvent::KeyRelease:
            QTimer::singleShot(0, m_window, m_save);
            break;
        default: break;
        }
        return false;
    }
private:
    QWidget *m_window;
    std::function<void()> m_save;
};

/** 【OCR 翻译输入对照】【平台入口】按 application、output 建立真实原版窗口，返回事件循环状态；从私有元数据读取本地命令，不读取用户配置 */
static int wayland(QApplication &application, const QString &output)
{
    QFile options(output + "/reference-options.json");
    if (!options.open(QIODevice::ReadOnly)) return 5;
    const auto request = QJsonDocument::fromJson(options.readAll()).object();
    if (!markshot::writeAppConfigRoot({{"ui", QJsonObject{{"theme", "dark"}}},
        {"translation", QJsonObject{{"command", request.value("command").toString()}, {"targetLanguage", "English"}, {"timeoutMs", 10000}}}})) return 6;
    QImage image(2, 2, QImage::Format_ARGB32); image.fill(Qt::transparent);
    markshot::shot::OcrResultWindow window("base", nullptr, image, {});
    window.setAttribute(Qt::WA_DeleteOnClose, false);
    window.resize(760, 460);
    const auto save = [&window, &output] {
        QSaveFile file(output + "/reference-state.json");
        if (file.open(QIODevice::WriteOnly)) {
            file.write(QJsonDocument(snapshot(window)).toJson(QJsonDocument::Compact));
            file.commit();
        }
    };
    Observer observer(&window, save); application.installEventFilter(&observer);
    QObject::connect(&application, &QApplication::focusChanged, &window, [save] { save(); });
    for (const char *name : {"ocrEditor", "ocrTranslationEditor"}) {
        QObject::connect(window.findChild<QTextEdit *>(name), &QTextEdit::textChanged, &window, save);
    }
    QObject::connect(window.findChild<QComboBox *>("ocrLanguageCombo"), &QComboBox::currentTextChanged, &window, save);
    auto *language = window.findChild<QComboBox *>("ocrLanguageCombo")->lineEdit();
    QObject::connect(language, &QLineEdit::selectionChanged, &window, save);
    QObject::connect(language, &QLineEdit::cursorPositionChanged, &window, [save] { save(); });
    window.show(); window.activateWindow(); QTest::qWait(40); save();
    std::cout << "ready\n" << std::flush;
    return application.exec();
}

/** 【OCR 翻译输入对照】【入口】argc、argv 指定输出目录，返回成功状态，原版源码与用户配置始终保持不变 */
int main(int argc, char **argv)
{
    const bool native = argc > 2 && QByteArray(argv[2]) == "--wayland";
    const QByteArray runtime = qgetenv("XDG_RUNTIME_DIR");
    if (native && (qgetenv("MARK_SHOT_PRIVATE_QT_REFERENCE") != runtime || !QFile::exists(QString::fromUtf8(runtime) + "/runtime.json"))) return 4;
    QTemporaryDir home;
    if (!markshot::test::isolateConfig(home)) return 2;
    if (native) { qputenv("XDG_RUNTIME_DIR", runtime); qputenv("QT_QPA_PLATFORM", "wayland"); }
    else {
        for (const char *name : {"DISPLAY", "WAYLAND_DISPLAY", "NIRI_SOCKET", "SWAYSOCK", "XDG_SESSION_TYPE"}) qunsetenv(name);
        qputenv("QT_QPA_PLATFORM", "offscreen");
    }
    qunsetenv("DBUS_SESSION_BUS_ADDRESS");
    QApplication application(argc, argv); QApplication::setQuitOnLastWindowClosed(native);
    QApplication::setDesktopFileName("mark-shot-ocr-result");
    markshot::i18n::setLanguage(markshot::i18n::Language::English);
    const QString output = argc > 1 ? QString::fromLocal8Bit(argv[1]) : home.path(); QDir().mkpath(output);
    if (native) return wayland(application, output);
    QJsonArray cases;
    try { for (int width : {380, 760}) cases.append(scenario(width, output)); }
    catch (const std::exception &error) { std::cerr << error.what(); return 3; }
    std::cout << QJsonDocument(QJsonObject{{"cases", cases}}).toJson(QJsonDocument::Compact).toStdString() << std::flush;
    return 0;
}
