/** 【OCR 表格对照】【真实原版】只读调用原版窗口，采集网格、表头和滚动几何，不读取用户配置 */
#include "app_config_store.h"
#include "ocr_result_window/ocr_result_window.h"
#include "settings/application_theme.h"
#include "support/isolated_config.h"
#include "ui/i18n.h"
#include "ui/theme.h"
#include <QApplication>
#include <QAbstractItemDelegate>
#include <QDir>
#include <QFile>
#include <QFontInfo>
#include <QFontMetricsF>
#include <QHeaderView>
#include <QJsonArray>
#include <QJsonDocument>
#include <QScrollBar>
#include <QStyleOptionViewItem>
#include <QTabBar>
#include <QTableWidget>
#include <QTemporaryDir>
#include <QtTest/QTest>
#include <stdexcept>

/** 【OCR 表格对照】【矩形读取】将 rect 转换为 JSON 四元组，返回逻辑坐标，不改变控件 */
static QJsonArray rectangle(const QRect &rect)
{
    return {rect.x(), rect.y(), rect.width(), rect.height()};
}

/** 【OCR 表格对照】【滚动读取】读取 bar 相对 table 的位置、范围和步长，返回 JSON，不修改滚动值 */
static QJsonObject scrollbar(QTableWidget *table, QScrollBar *bar)
{
    return {{"rect", rectangle(QRect(bar->mapTo(table, QPoint()), bar->size()))},
        {"visible", bar->isVisible()}, {"minimum", bar->minimum()}, {"maximum", bar->maximum()},
        {"page", bar->pageStep()}, {"step", bar->singleStep()}, {"value", bar->value()}};
}

/** 【OCR 表格对照】【状态采集】按 window 读取真实网格与内容列宽，返回 JSON，保留原版选择和渲染 */
static QJsonObject snapshot(markshot::shot::OcrResultWindow &window)
{
    auto *table = window.findChild<QTableWidget *>("ocrTable");
    auto *horizontal = table->horizontalHeader();
    auto *vertical = table->verticalHeader();
    QJsonArray columns, rows, cells;
    for (int column = 0; column < table->columnCount(); ++column) {
        qreal textWidth = 0;
        int boundingWidth = 0, delegateWidth = 0;
        for (int row = 0; row < table->rowCount(); ++row) {
            for (const QString &line : table->item(row, column)->text().split('\n')) {
                textWidth = qMax(textWidth, QFontMetricsF(table->font()).horizontalAdvance(line));
                boundingWidth = qMax(boundingWidth, table->fontMetrics().boundingRect(line).width());
            }
            QStyleOptionViewItem option; option.initFrom(table);
            option.font = table->font(); option.fontMetrics = table->fontMetrics();
            delegateWidth = qMax(delegateWidth, table->itemDelegate()->sizeHint(option, table->model()->index(row, column)).width());
        }
        columns.append(QJsonObject{{"width", table->columnWidth(column)},
            {"content_width", horizontal->sectionSizeHint(column)},
            {"text_width", textWidth},
            {"bounding_width", boundingWidth}, {"delegate_width", delegateWidth},
            {"header", table->model()->headerData(column, Qt::Horizontal).toString()}});
    }
    for (int row = 0; row < table->rowCount(); ++row) {
        rows.append(QJsonObject{{"height", table->rowHeight(row)},
            {"header", table->model()->headerData(row, Qt::Vertical).toString()}});
        if (row < 3) {
            for (int column = 0; column < table->columnCount(); ++column) {
                cells.append(QJsonObject{{"row", row}, {"column", column},
                    {"text", table->item(row, column)->text()},
                    {"rect", rectangle(table->visualItemRect(table->item(row, column)))},
                    {"selected", table->item(row, column)->isSelected()}});
            }
        }
    }
    QJsonObject palette;
    for (const auto &entry : {qMakePair("base", QPalette::Base), qMakePair("text", QPalette::Text),
        qMakePair("alternate", QPalette::AlternateBase), qMakePair("accent", QPalette::Highlight),
        qMakePair("on_accent", QPalette::HighlightedText)}) {
        const QColor value = table->palette().color(entry.second);
        palette[entry.first] = QJsonArray{value.red(), value.green(), value.blue(), value.alpha()};
    }
    return {{"table", rectangle(QRect(table->mapTo(&window, QPoint()), table->size()))},
        {"viewport", rectangle(table->viewport()->geometry())},
        {"horizontal_header", rectangle(horizontal->geometry())},
        {"vertical_header", rectangle(vertical->geometry())}, {"frame_width", table->frameWidth()},
        {"font_pixels", QFontInfo(table->font()).pixelSize()}, {"font_height", table->fontMetrics().height()},
        {"font_family", QFontInfo(table->font()).family()}, {"columns", columns}, {"rows", rows},
        {"header_font_pixels", QFontInfo(horizontal->font()).pixelSize()},
        {"header_font_height", horizontal->fontMetrics().height()},
        {"header_font_family", QFontInfo(horizontal->font()).family()},
        {"minimum_column", horizontal->minimumSectionSize()},
        {"gutter_hint", vertical->sizeHint().width()},
        {"gutter_text_width", QFontMetricsF(vertical->font()).horizontalAdvance(QString::number(table->rowCount()))},
        {"cells", cells}, {"palette", palette}, {"horizontal_scroll", scrollbar(table, table->horizontalScrollBar())},
        {"vertical_scroll", scrollbar(table, table->verticalScrollBar())},
        {"horizontal_mode", int(table->horizontalScrollMode())}, {"vertical_mode", int(table->verticalScrollMode())}};
}

/** 【OCR 表格对照】【完整场景】用 theme、width、kind 创建真实窗口，output 保存截图，返回初始、选择及编辑状态 */
static QJsonObject scenario(const QString &theme, int width, const QString &kind, const QString &output)
{
    if (!markshot::writeAppConfigRoot({{"ui", QJsonObject{{"theme", theme}}}})) {
        throw std::runtime_error("private configuration failed");
    }
    markshot::settings::applyConfiguredApplicationTheme();
    const int rowCount = kind == "tall" ? 40 : 3;
    const int columnCount = kind == "wide" ? 8 : 3;
    QVector<markshot::ocr::Token> tokens;
    QStringList values;
    for (int row = 0; row < rowCount; ++row) {
        for (int column = 0; column < columnCount; ++column) {
            QString text = QString::number(row * columnCount + column);
            if (kind == "content") {
                const QStringList entries{"Item", "Count", "Total", "Paper", "12", "36", "Ink", "2", "24"};
                text = entries.at(row * columnCount + column);
            } else if (kind == "wide") {
                text = "A long column value " + text;
            } else if (kind == "multiline" && row == 1 && column == 0) {
                text = "first\nsecond\nthird";
            }
            tokens.append({text, QRectF(38 + column * 220, 32 + row * 66, 40, 24), row, column, 1.0});
            values.append(text);
        }
    }
    QImage image(2, 2, QImage::Format_ARGB32); image.fill(Qt::transparent);
    markshot::shot::OcrResultWindow window(values.join('\n'), nullptr, image, tokens);
    window.setAttribute(Qt::WA_DeleteOnClose, false);
    window.resize(width, 460); window.show(); window.activateWindow(); QTest::qWait(30);
    window.findChild<QTabBar *>("ocrViewTabs")->setCurrentIndex(1); QTest::qWait(25);
    auto *table = window.findChild<QTableWidget *>("ocrTable");
    QJsonArray states;
    const QString prefix = theme + "-" + QString::number(width) + "-" + kind;
    const auto record = [&](const QString &name) {
        QTest::qWait(10);
        states.append(QJsonObject{{"name", name}, {"state", snapshot(window)}});
        if (width == 760 || width == 380) window.grab().save(output + "/" + prefix + "-" + name + ".png");
    };
    // 1. 【OCR 表格对照】【初始布局】使用真实表格内容与列头，不手工调整原版几何
    record("initial");
    QTest::mouseClick(table->viewport(), Qt::LeftButton, Qt::NoModifier, table->visualItemRect(table->item(1, 0)).center());
    record("selected");
    // 2. 【OCR 表格对照】【编辑稳定】提交较长多行值后读取行列尺寸，确认原版是否自动重新计算
    table->item(1, 0)->setText("Changed value\nsecond line");
    record("changed");
    if (kind == "wide" || kind == "tall") {
        table->horizontalScrollBar()->setValue(table->horizontalScrollBar()->maximum());
        table->verticalScrollBar()->setValue(table->verticalScrollBar()->maximum());
        record("scrolled");
    }
    window.hide();
    return {{"theme", theme}, {"width", width}, {"kind", kind}, {"states", states}};
}

/** 【OCR 表格对照】【入口】argc、argv 指定输出目录，返回执行状态；全部配置与图形会话均隔离 */
int main(int argc, char **argv)
{
    if (argc != 2) return 2;
    QTemporaryDir home;
    if (!markshot::test::isolateConfig(home)) return 3;
    for (const char *name : {"DISPLAY", "WAYLAND_DISPLAY", "NIRI_SOCKET", "SWAYSOCK", "DBUS_SESSION_BUS_ADDRESS"}) qunsetenv(name);
    qputenv("QT_QPA_PLATFORM", "offscreen");
    QApplication app(argc, argv);
    // 1. 【OCR 表格对照】【启动一致】复用原版 main 的全局字体初始化，避免表头退回测试平台默认家族
    QFont applicationFont = app.font(); applicationFont.setFamily(markshot::theme::uiFontFamily()); app.setFont(applicationFont);
    QApplication::setQuitOnLastWindowClosed(false);
    markshot::i18n::setLanguage(markshot::i18n::Language::English);
    const QString output = QString::fromLocal8Bit(argv[1]); QDir().mkpath(output);
    QJsonArray cases;
    for (const QString &theme : {"dark", "light"}) {
        for (const int width : {380, 640, 760}) {
            for (const QString &kind : {"numeric", "content", "multiline", "wide", "tall"}) {
                cases.append(scenario(theme, width, kind, output));
            }
        }
    }
    QFile result(output + "/result.json");
    if (!result.open(QIODevice::WriteOnly)) return 4;
    result.write(QJsonDocument(QJsonObject{{"cases", cases}}).toJson(QJsonDocument::Compact));
    return 0;
}
