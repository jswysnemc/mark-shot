/** 【OCR 滚动对照】【真实原版】只读链接原版窗口，记录滚轮、滑块和轨道动作，配置与图形平台保持隔离 */
#include "app_config_store.h"
#include "ocr_result_window/ocr_result_window.h"
#include "settings/application_theme.h"
#include "support/isolated_config.h"
#include "ui/i18n.h"
#include "ui/theme.h"
#include <QApplication>
#include <QDir>
#include <QFile>
#include <QFontMetricsF>
#include <QHeaderView>
#include <QJsonArray>
#include <QJsonDocument>
#include <QLineEdit>
#include <QMenu>
#include <QScrollBar>
#include <QStyle>
#include <QStyleOptionSlider>
#include <QTabBar>
#include <QTableWidget>
#include <QTemporaryDir>
#include <QWheelEvent>
#include <QtTest/QTest>
#include <stdexcept>

/** 【OCR 滚动对照】【矩形记录】按 rect 返回四项逻辑坐标，供原生几何精确比较 */
static QJsonArray rectangle(const QRect &rect)
{
    return {rect.x(), rect.y(), rect.width(), rect.height()};
}

/** 【OCR 滚动对照】【样式参数】按 bar 创建与真实滚动条相同的样式参数，返回选项，不替换原版控件 */
static QStyleOptionSlider option(QScrollBar *bar)
{
    QStyleOptionSlider value;
    value.initFrom(bar);
    value.orientation = bar->orientation();
    if (bar->orientation() == Qt::Horizontal) value.state |= QStyle::State_Horizontal;
    value.minimum = bar->minimum(); value.maximum = bar->maximum();
    value.sliderValue = bar->value(); value.sliderPosition = bar->sliderPosition();
    value.singleStep = bar->singleStep(); value.pageStep = bar->pageStep();
    value.upsideDown = false;
    return value;
}

/** 【OCR 滚动对照】【子控件区域】按 bar、control 返回真实样式的滑块、箭头或轨道矩形，不手工指定尺寸 */
static QRect control(QScrollBar *bar, QStyle::SubControl control)
{
    const auto value = option(bar);
    return bar->style()->subControlRect(QStyle::CC_ScrollBar, &value, control, bar);
}

/** 【OCR 滚动对照】【滚动条记录】按 bar 返回范围、值、子控件和样式身份，不改变滚动位置 */
static QJsonObject scrollbar(QScrollBar *bar)
{
    QJsonObject controls;
    for (const auto &entry : {qMakePair("slider", QStyle::SC_ScrollBarSlider),
        qMakePair("groove", QStyle::SC_ScrollBarGroove), qMakePair("add_line", QStyle::SC_ScrollBarAddLine),
        qMakePair("sub_line", QStyle::SC_ScrollBarSubLine), qMakePair("add_page", QStyle::SC_ScrollBarAddPage),
        qMakePair("sub_page", QStyle::SC_ScrollBarSubPage)}) controls[entry.first] = rectangle(control(bar, entry.second));
    return {{"visible", bar->isVisible()}, {"rect", rectangle(QRect(bar->mapTo(bar->parentWidget()->parentWidget(), QPoint()), bar->size()))},
        {"minimum", bar->minimum()}, {"maximum", bar->maximum()}, {"page", bar->pageStep()}, {"step", bar->singleStep()},
        {"value", bar->value()}, {"controls", controls}, {"down", bar->isSliderDown()}};
}

/** 【OCR 滚动对照】【窗口状态】读取 table 的实际偏移、可见单元格和滚动条，返回完整状态，不依赖原生实现 */
static QJsonObject snapshot(QTableWidget *table)
{
    QJsonArray columns;
    for (int column = 0; column < table->columnCount(); ++column) columns.append(table->columnWidth(column));
    QJsonArray positions;
    for (const auto &index : {qMakePair(0, 0), qMakePair(table->rowCount() - 1, table->columnCount() - 1)}) {
        positions.append(rectangle(table->visualItemRect(table->item(index.first, index.second))));
    }
    auto *editor = table->findChild<QLineEdit *>();
    return {{"columns", columns}, {"viewport", rectangle(table->viewport()->geometry())},
        {"first_text_width", QFontMetricsF(table->font()).horizontalAdvance(table->item(0, 0)->text())},
        {"w_width", QFontMetricsF(table->font()).horizontalAdvance("W")},
        {"offset", QJsonArray{table->horizontalHeader()->offset(), table->verticalHeader()->offset()}},
        {"positions", positions}, {"horizontal", scrollbar(table->horizontalScrollBar())},
        {"vertical", scrollbar(table->verticalScrollBar())}, {"editing", editor && editor->isVisible()},
        {"cell", table->item(0, 0)->text()}, {"current", QJsonArray{table->currentRow(), table->currentColumn()}}};
}

/** 【OCR 滚动对照】【滚轮发送】按 widget、angle、pixel、mods 发送真实 Qt 滚轮事件，无返回值，不直接设定滚动条值 */
static void wheel(QWidget *widget, QPoint angle, QPoint pixel = {}, Qt::KeyboardModifiers mods = Qt::NoModifier)
{
    const QPoint point = widget->rect().center();
    QWheelEvent event(point, widget->mapToGlobal(point), pixel, angle, Qt::NoButton, mods, Qt::NoScrollPhase, false);
    QCoreApplication::sendEvent(widget, &event);
    QApplication::processEvents();
}

/** 【OCR 滚动对照】【场景执行】按 theme、width、kind 构造真实窗口，output 保存像素和完整动作，返回场景记录 */
static QJsonObject scenario(const QString &theme, int width, const QString &kind, const QString &output)
{
    if (!markshot::writeAppConfigRoot({{"ui", QJsonObject{{"theme", theme}}}})) throw std::runtime_error("private configuration failed");
    markshot::settings::applyConfiguredApplicationTheme();
    const int rows = kind == "wide" ? 3 : 40;
    const int columns = kind == "tall" ? 3 : 8;
    QVector<markshot::ocr::Token> tokens;
    for (int row = 0; row < rows; ++row) {
        for (int column = 0; column < columns; ++column) {
            QString text = kind == "tall" ? QString::number(row * columns + column) : "A long column value " + QString::number(row * columns + column);
            if (kind == "oversized" && column == 0) text = QString(180, 'W');
            tokens.append({text, QRectF(38 + column * 3000, 32 + row * 66, 40, 24), row, column, 1.0});
        }
    }
    QImage image(2, 2, QImage::Format_ARGB32); image.fill(Qt::transparent);
    markshot::shot::OcrResultWindow window("business source", nullptr, image, tokens);
    window.setAttribute(Qt::WA_DeleteOnClose, false);
    window.resize(width, 460); window.show(); window.activateWindow(); QTest::qWait(25);
    window.findChild<QTabBar *>("ocrViewTabs")->setCurrentIndex(1); QTest::qWait(20);
    auto *table = window.findChild<QTableWidget *>("ocrTable");
    QJsonArray states;
    const QString prefix = theme + "-" + QString::number(width) + "-" + kind;
    const auto record = [&](const QString &name) {
        QApplication::processEvents();
        states.append(QJsonObject{{"name", name}, {"state", snapshot(table)}});
        // 1. 【OCR 滚动对照】【计时隔离】重复计时场景只记录状态，截图耗时不插入两次等待之间
        if (width == 760 && !name.contains("repeat")) {
            window.grab().save(output + "/" + prefix + "-" + name + ".png");
            if (table->horizontalScrollBar()->isVisible()) table->horizontalScrollBar()->grab().save(output + "/" + prefix + "-" + name + "-horizontal.png");
            if (table->verticalScrollBar()->isVisible()) table->verticalScrollBar()->grab().save(output + "/" + prefix + "-" + name + "-vertical.png");
        }
    };
    const auto reset = [&] { table->horizontalScrollBar()->setValue(0); table->verticalScrollBar()->setValue(0); QApplication::processEvents(); };
    // 1. 【OCR 滚动对照】【滚轮链路】记录完整刻度、小步累积、连续位移、Shift 和 Ctrl，不用滚动值替代实际事件
    record("initial");
    wheel(table->viewport(), {0, -120}); record("wheel-down");
    wheel(table->viewport(), {0, 120}); record("wheel-up");
    reset(); wheel(table->viewport(), {-120, 0}); record("wheel-right");
    reset(); wheel(table->viewport(), {0, -120}, {}, Qt::ShiftModifier); record("shift-wheel");
    reset(); wheel(table->viewport(), {0, -120}, {}, Qt::ControlModifier); record("ctrl-wheel");
    reset(); wheel(table->viewport(), {}, {0, -15}); record("pixel-only");
    reset();
    for (int step = 1; step <= 4; ++step) { wheel(table->viewport(), {0, -20}); record("partial-" + QString::number(step)); }
    // 2. 【OCR 滚动对照】【真实控件】记录左键轨道、箭头、滑块拖动；右键和中键单独比较，保持与左键职责区分
    for (const auto &entry : {qMakePair(QStringLiteral("horizontal"), table->horizontalScrollBar()), qMakePair(QStringLiteral("vertical"), table->verticalScrollBar())}) {
        auto *bar = entry.second;
        if (!bar->isVisible()) continue;
        const QString axis = entry.first;
        reset(); QTest::mouseMove(bar, control(bar, QStyle::SC_ScrollBarSlider).center()); record(axis + "-hover");
        if (control(bar, QStyle::SC_ScrollBarAddLine).isValid()) {
            reset(); QTest::mouseClick(bar, Qt::LeftButton, Qt::NoModifier, control(bar, QStyle::SC_ScrollBarAddLine).center()); record(axis + "-add-line");
            if (width == 760 && kind == "both") {
                reset(); int actions = 0;
                const auto connection = QObject::connect(bar, &QAbstractSlider::actionTriggered, bar, [&actions](int) { ++actions; });
                QTest::mousePress(bar, Qt::LeftButton, Qt::NoModifier, control(bar, QStyle::SC_ScrollBarAddLine).center());
                QTest::qWait(300); record(axis + "-line-before-repeat");
                // 1. 【OCR 滚动对照】【实际重复次数】等待两次真实重复动作，避免操作系统调度误差变成冻结状态差异
                int attempts = 0;
                while (actions < 3 && attempts++ < 200) QTest::qWait(10);
                if (actions != 3) throw std::runtime_error("expected two real line-repeat actions");
                record(axis + "-line-after-repeat");
                QTest::mouseRelease(bar, Qt::LeftButton, Qt::NoModifier, control(bar, QStyle::SC_ScrollBarAddLine).center());
                QObject::disconnect(connection);
            }
        }
        reset(); QTest::mouseClick(bar, Qt::LeftButton, Qt::NoModifier, control(bar, QStyle::SC_ScrollBarAddPage).center()); record(axis + "-add-page");
        if (width == 760 && kind == "both") {
            reset(); const QPoint repeatPoint = control(bar, QStyle::SC_ScrollBarAddPage).center();
            QTest::mousePress(bar, Qt::LeftButton, Qt::NoModifier, repeatPoint);
            QTest::qWait(300); record(axis + "-page-before-repeat");
            QTest::qWait(260); record(axis + "-page-after-repeat");
            QTest::qWait(150); record(axis + "-page-repeat-stopped");
            QTest::mouseRelease(bar, Qt::LeftButton, Qt::NoModifier, repeatPoint);
        }
        reset(); const QPoint start = control(bar, QStyle::SC_ScrollBarSlider).center();
        QTest::mousePress(bar, Qt::LeftButton, Qt::NoModifier, start);
        record(axis + "-slider-down");
        const auto groove = control(bar, QStyle::SC_ScrollBarGroove);
        const QPoint end = bar->orientation() == Qt::Horizontal ? QPoint(groove.right(), start.y()) : QPoint(start.x(), groove.bottom());
        QTest::mouseMove(bar, end); record(axis + "-drag-max");
        QTest::mouseRelease(bar, Qt::LeftButton, Qt::NoModifier, end); record(axis + "-released");
        reset(); QTest::mouseClick(bar, Qt::MiddleButton, Qt::NoModifier, control(bar, QStyle::SC_ScrollBarAddPage).center()); record(axis + "-middle-page");
        reset(); const QRect thumb = control(bar, QStyle::SC_ScrollBarSlider);
        const QPoint middleOnThumb = bar->orientation() == Qt::Horizontal ? QPoint(thumb.right() - 3, thumb.center().y()) : QPoint(thumb.center().x(), thumb.bottom() - 3);
        QTest::mouseClick(bar, Qt::MiddleButton, Qt::NoModifier, middleOnThumb); record(axis + "-middle-slider-offset");
        reset(); QTest::mousePress(bar, Qt::RightButton, Qt::NoModifier, control(bar, QStyle::SC_ScrollBarSlider).center()); record(axis + "-right-down");
        QTest::mouseRelease(bar, Qt::RightButton, Qt::NoModifier, control(bar, QStyle::SC_ScrollBarSlider).center());
        for (auto *menu : window.findChildren<QMenu *>()) menu->close();
        reset(); bar->setValue(bar->maximum()); record(axis + "-maximum");
    }
    window.hide();
    return {{"theme", theme}, {"width", width}, {"kind", kind}, {"states", states}};
}

/** 【OCR 滚动对照】【入口】argc、argv 指定私有输出目录，返回状态；不读取真实桌面或用户配置 */
int main(int argc, char **argv)
{
    if (argc != 2) return 2;
    QTemporaryDir home;
    if (!markshot::test::isolateConfig(home)) return 3;
    for (const char *name : {"DISPLAY", "WAYLAND_DISPLAY", "NIRI_SOCKET", "SWAYSOCK", "DBUS_SESSION_BUS_ADDRESS"}) qunsetenv(name);
    qputenv("QT_QPA_PLATFORM", "offscreen");
    QApplication app(argc, argv);
    QFont font = app.font(); font.setFamily(markshot::theme::uiFontFamily()); app.setFont(font);
    QApplication::setQuitOnLastWindowClosed(false);
    markshot::i18n::setLanguage(markshot::i18n::Language::English);
    const QString output = QString::fromLocal8Bit(argv[1]); QDir().mkpath(output);
    QJsonArray cases;
    for (const QString &theme : {"dark", "light"}) {
        for (const int width : {380, 640, 760}) {
            for (const QString &kind : {"tall", "wide", "both", "oversized"}) cases.append(scenario(theme, width, kind, output));
        }
    }
    QFile file(output + "/result.json");
    if (!file.open(QIODevice::WriteOnly)) return 4;
    file.write(QJsonDocument(QJsonObject{{"style", app.style()->objectName()}, {"wheel_scroll_lines", QApplication::wheelScrollLines()}, {"cases", cases}}).toJson(QJsonDocument::Compact));
    return 0;
}
