/** 【标题栏对照】【真实窗口】只读链接完整 OcrResultWindow，采集布局、可见性、字体及绘制，不改写原版行为 */
#include "app_config_store.h"
#include "ocr_result_window/ocr_result_window.h"
#include "ocr_result_window/ocr_text_pane.h"
#include "ocr_table/ocr_table_pane.h"
#include "support/isolated_config.h"
#include "ui/i18n.h"

#include <QApplication>
#include <QDir>
#include <QFontMetrics>
#include <QFontInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QLabel>
#include <QPushButton>
#include <QStackedWidget>
#include <QTabBar>
#include <QTemporaryDir>
#include <QTextEdit>
#include <QTimer>
#include <QScreen>
#include <QtTest/QTest>
#include <iostream>
#include <stdexcept>

/** 【标题栏对照】【实际矩形】读取 widget 相对 window 的位置，返回整数矩形，不改变几何 */
static QJsonArray rect(QWidget *window, QWidget *widget)
{
    const QPoint point = widget->mapTo(window, QPoint());
    return {point.x(), point.y(), widget->width(), widget->height()};
}

/** 【标题栏对照】【控件状态】读取 window、widget 的字体、尺寸、可见性和焦点策略，返回实际状态 */
static QJsonObject widgetState(QWidget *window, QWidget *widget)
{
    const QFont font = widget->font();
    QJsonObject state{{"rect", rect(window, widget)}, {"visible", widget->isVisible()},
        {"enabled", widget->isEnabled()}, {"focus_policy", int(widget->focusPolicy())},
        {"font_family", font.family()}, {"point_size", font.pointSizeF()},
        {"pixel_size", font.pixelSize()}, {"font_height", QFontMetrics(font).height()},
        {"resolved_family", QFontInfo(font).family()}, {"resolved_pixels", QFontInfo(font).pixelSize()},
        {"size_hint", QJsonArray{widget->sizeHint().width(), widget->sizeHint().height()}},
        {"minimum_hint", QJsonArray{widget->minimumSizeHint().width(), widget->minimumSizeHint().height()}}};
    if (auto *button = qobject_cast<QPushButton *>(widget)) {
        state["text"] = button->text();
        state["text_width"] = button->fontMetrics().horizontalAdvance(button->text());
        state["icon_size"] = QJsonArray{button->iconSize().width(), button->iconSize().height()};
        state["checked"] = button->isChecked();
        state["down"] = button->isDown();
        state["tooltip"] = button->toolTip();
    }
    return state;
}

/** 【标题栏对照】【调色板】读取 palette 的 role 并返回 RGBA，不以截图片段猜测颜色 */
static QJsonArray rgba(const QPalette &palette, QPalette::ColorRole role)
{
    const QColor color = palette.color(role);
    return {color.red(), color.green(), color.blue(), color.alpha()};
}

/** 【标题栏对照】【完整状态】读取 window 内原版控件和文字视区，返回实际布局，不生成替代窗口 */
static QJsonObject snapshot(markshot::shot::OcrResultWindow &window)
{
    QJsonObject widgets;
    for (const QString &name : {"ocrTitleBar", "ocrViewTabs", "ocrTranslationToggle", "ocrMoreButton",
             "ocrPinButton", "ocrCloseButton", "ocrContentViews", "ocrSourcePane", "ocrEditor",
             "ocrTranslationPane", "ocrTablePane", "ocrTableCopyButton", "ocrSourcePreview", "ocrResizeGrip"}) {
        if (auto *widget = window.findChild<QWidget *>(name)) widgets[name] = widgetState(&window, widget);
    }
    auto *title = window.findChild<QWidget *>("ocrTitleBar");
    auto *copy = title->findChild<QPushButton *>("ocrCopyButton");
    auto *source = window.findChild<QWidget *>("ocrSourcePane");
    auto *undo = source->findChild<QPushButton *>("ocrUndoButton");
    auto *editor = window.findChild<QTextEdit *>("ocrEditor");
    if (!copy || !undo || !editor) throw std::runtime_error("missing original source controls");
    widgets["source_copy"] = widgetState(&window, copy);
    widgets["source_undo"] = widgetState(&window, undo);
    widgets["editor_viewport"] = widgetState(&window, editor->viewport());
    const auto titleLabels = title->findChildren<QLabel *>();
    if (titleLabels.isEmpty()) throw std::runtime_error("missing original title icon");
    widgets["title_icon"] = widgetState(&window, titleLabels.first());
    QJsonArray tabs;
    auto *bar = window.findChild<QTabBar *>("ocrViewTabs");
    const QPoint origin = bar->mapTo(&window, QPoint());
    for (int index = 0; index < bar->count(); ++index) {
        const QRect bounds = bar->tabRect(index).translated(origin);
        tabs.append(QJsonObject{{"text", bar->tabText(index)},
            {"text_width", bar->fontMetrics().horizontalAdvance(bar->tabText(index))},
            {"rect", QJsonArray{bounds.x(), bounds.y(), bounds.width(), bounds.height()}}});
    }
    const QPalette colors = window.palette();
    QWidget *focus = QApplication::focusWidget();
    return {{"size", QJsonArray{window.width(), window.height()}},
        {"minimum_size", QJsonArray{window.minimumWidth(), window.minimumHeight()}},
        {"widgets", widgets}, {"tabs", tabs}, {"tab", bar->currentIndex()},
        {"focus", focus ? focus->objectName() : ""},
        {"colors", QJsonObject{{"window", rgba(colors, QPalette::Window)}, {"base", rgba(colors, QPalette::Base)},
            {"text", rgba(colors, QPalette::Text)}, {"button", rgba(colors, QPalette::Button)},
            {"alternate", rgba(colors, QPalette::AlternateBase)}, {"accent", rgba(colors, QPalette::Highlight)},
            {"on_accent", rgba(colors, QPalette::HighlightedText)}}}};
}

/** 【标题栏对照】【实际表格】返回与原生固定结果帧一致的 token，imageRect、line、index 使用真实推断入口 */
static QVector<markshot::ocr::Token> tableTokens()
{
    QVector<markshot::ocr::Token> tokens;
    const QList<QStringList> rows{{"Item", "Count", "Total"}, {"Paper", "12", "36"}, {"Ink", "2", "24"}};
    for (int row = 0; row < rows.size(); ++row) {
        for (int column = 0; column < rows[row].size(); ++column) {
            const QString text = rows[row][column];
            tokens.append({text, QRectF(38 + column * 220, 32 + row * 66, qMax(26, int(text.size()) * 15), 24), row, column, 1.0});
        }
    }
    return tokens;
}

/** 【标题栏对照】【窗口案例】按 width、table、image、empty 构建原版窗口，output 保存截图，返回逐页及撤销状态 */
static QJsonObject scenario(int width, bool table, bool image, bool empty, const QString &output)
{
    QImage source;
    if (image) {
        source = QImage(600, 210, QImage::Format_ARGB32);
        source.fill(QColor("#e6d8b5"));
    }
    markshot::shot::OcrResultWindow window(empty ? "  " : "alpha beta\ngamma", nullptr, source,
        table ? tableTokens() : QVector<markshot::ocr::Token>{});
    window.setAttribute(Qt::WA_DeleteOnClose, false);
    window.resize(width, 520);
    window.show();
    window.activateWindow();
    QTest::qWait(35);
    QJsonArray states;
    states.append(snapshot(window));
    auto *bar = window.findChild<QTabBar *>("ocrViewTabs");
    for (int index = 1; index < bar->count(); ++index) {
        bar->setCurrentIndex(index);
        QTest::qWait(15);
        states.append(snapshot(window));
    }
    bar->setCurrentIndex(0);
    QTest::qWait(15);
    auto *editor = window.findChild<QTextEdit *>("ocrEditor");
    editor->moveCursor(QTextCursor::End);
    editor->insertPlainText("!");
    QTest::qWait(15);
    const QJsonObject withUndo = snapshot(window);
    if (table && image && !empty && (width == 380 || width == 500 || width == 760)) {
        const QString language = markshot::i18n::language() == markshot::i18n::Language::English ? "en" : "zh";
        window.grab().save(output + "/" + language + "-" + QString::number(width) + ".png");
    }
    return {{"requested_width", width}, {"table", table}, {"image", image}, {"empty", empty},
        {"states", states}, {"with_undo", withUndo}};
}

/** 【标题栏对照】【反馈期限】调用实际分区反馈入口并观察计时、切页及编辑，返回状态，不触发识别或翻译任务 */
static QJsonObject feedbackSequence()
{
    markshot::i18n::setLanguage(markshot::i18n::Language::English);
    markshot::shot::OcrResultWindow window("alpha", nullptr, {}, tableTokens());
    window.setAttribute(Qt::WA_DeleteOnClose, false);
    window.resize(760, 520);
    window.show();
    QTest::qWait(40);
    auto *source = window.findChild<markshot::shot::OcrTextPane *>("ocrSourcePane");
    auto *table = window.findChild<markshot::ocr_table::OcrTablePane *>("ocrTablePane");
    auto *bar = window.findChild<QTabBar *>("ocrViewTabs");
    if (!source || !table || !bar) throw std::runtime_error("missing feedback owner");
    source->showCopyFeedback(true);
    table->showCopyFeedback(true);
    QTest::qWait(20);
    const int sourceInterval = source->findChild<QTimer *>(QString(), Qt::FindDirectChildrenOnly)->interval();
    const int tableInterval = table->findChild<QTimer *>(QString(), Qt::FindDirectChildrenOnly)->interval();
    QJsonArray states;
    const auto record = [&](const QString &name) { states.append(QJsonObject{{"name", name}, {"state", snapshot(window)}}); };
    record("both-started");
    bar->setCurrentIndex(1);
    QTest::qWait(20);
    record("page-changed");
    source->editor()->insertPlainText("!");
    QTest::qWait(20);
    record("source-edited");
    QTest::qWait(300);
    source->showCopyFeedback(false);
    QTest::qWait(20);
    record("source-renewed-failure");
    QTest::qWait(1500);
    record("table-expired-source-renewed");
    QTest::qWait(700);
    record("both-expired");
    return {{"source_interval_ms", sourceInterval}, {"table_interval_ms", tableInterval}, {"states", states}};
}

/** 【标题栏对照】【入口】argc、argv 指定截图目录，返回状态；原版源码、对象与用户座席均保持只读隔离 */
int main(int argc, char **argv)
{
    const bool wayland = argc > 2 && QByteArray(argv[2]) == "--wayland";
    const QByteArray runtime = qgetenv("XDG_RUNTIME_DIR");
    if (wayland && (qgetenv("MARK_SHOT_PRIVATE_QT_REFERENCE") != runtime || !QFile::exists(QString::fromUtf8(runtime) + "/runtime.json"))) return 4;
    QTemporaryDir home;
    if (!markshot::test::isolateConfig(home)) return 2;
    if (wayland) {
        qputenv("XDG_RUNTIME_DIR", runtime);
        qputenv("QT_QPA_PLATFORM", "wayland");
    } else {
        for (const char *name : {"DISPLAY", "WAYLAND_DISPLAY", "NIRI_SOCKET", "SWAYSOCK", "XDG_SESSION_TYPE", "DBUS_SESSION_BUS_ADDRESS"}) qunsetenv(name);
    }
    QApplication app(argc, argv);
    QApplication::setDesktopFileName("mark-shot-qt-titlebar");
    QApplication::setQuitOnLastWindowClosed(false);
    if (!markshot::writeAppConfigRoot({{"ui", QJsonObject{{"theme", "dark"}}}})) return 3;
    const QString output = argc > 1 ? QString::fromLocal8Bit(argv[1]) : home.path();
    QDir().mkpath(output);
    QJsonArray languages;
    for (const auto language : {markshot::i18n::Language::English, markshot::i18n::Language::Chinese}) {
        markshot::i18n::setLanguage(language);
        QJsonArray cases;
        for (const int width : {380, 430, 480, 499, 500, 619, 620, 760, 1000}) {
            cases.append(scenario(width, true, true, false, output));
            cases.append(scenario(width, false, true, false, output));
            cases.append(scenario(width, false, false, false, output));
            cases.append(scenario(width, true, true, true, output));
        }
        languages.append(QJsonObject{{"name", language == markshot::i18n::Language::English ? "en" : "zh"}, {"cases", cases}});
    }
    QScreen *screen = QApplication::primaryScreen();
    const QJsonObject platform{{"name", QApplication::platformName()}, {"logical_dpi", screen->logicalDotsPerInch()},
        {"dpr", screen->devicePixelRatio()}};
    std::cout << QJsonDocument(QJsonObject{{"qt_version", QT_VERSION_STR}, {"platform", platform}, {"languages", languages},
        {"feedback", feedbackSequence()}}).toJson(QJsonDocument::Compact).toStdString();
}
