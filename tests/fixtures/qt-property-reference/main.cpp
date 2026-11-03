// 【属性对照】【实际窗口】使用原版 ShotWindow 对象取得控件树和缓冲，不创建截图会话、不读写用户配置

#include "shot_window.h"
#include "support/isolated_config.h"
#include "app_config_store.h"
#include <QApplication>
#include <QFile>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLabel>
#include <QLayout>
#include <QListWidget>
#include <QMouseEvent>
#include <QPushButton>
#include <QSlider>
#include <QStyleOptionSlider>
#include <QTimer>
#include <QEventLoop>
#include <QTemporaryDir>
#include <QTextEdit>
#include <QSpinBox>
#include <QLineEdit>
#include <QKeyEvent>
#include <QScrollBar>
#include "size_inputs.h"
#include "axis_events.h"
#include "text_editing.h"
#include "text_menu.h"
#include "text_format_history.h"
#include "text_caret.h"
#include "text_input_method.h"
#include "line_edit_input_method.h"

/** 【属性对照】【控件记录】递归读取 root 的控件属性与相对几何；返回真实控件树，不使用手写位置。 */
QJsonArray children(QWidget *root)
{
    QJsonArray result;
    for (QWidget *widget : root->findChildren<QWidget *>(QString(), Qt::FindDirectChildrenOnly)) {
        const QRect rect = widget->geometry();
        QJsonObject item{{"class", widget->metaObject()->className()}, {"name", widget->objectName()},
            {"geometry", QJsonArray{rect.x(), rect.y(), rect.width(), rect.height()}},
            {"visible", widget->isVisible()}, {"tooltip", widget->toolTip()}, {"accessible", widget->accessibleName()}};
        if (auto *label = qobject_cast<QLabel *>(widget)) item["text"] = label->text();
        if (auto *slider = qobject_cast<QSlider *>(widget)) {
            item["range"] = QJsonArray{slider->minimum(), slider->maximum()};
            item["value"] = slider->value();
            QStyleOptionSlider option;
            option.initFrom(slider);
            option.styleObject = slider;
            option.state |= QStyle::State_Horizontal;
            option.orientation = slider->orientation();
            option.minimum = slider->minimum(); option.maximum = slider->maximum();
            option.sliderPosition = slider->sliderPosition(); option.sliderValue = slider->value();
            for (const auto control : {QStyle::SC_SliderGroove, QStyle::SC_SliderHandle}) {
                const QRect part = slider->style()->subControlRect(QStyle::CC_Slider, &option, control, slider);
                item[control == QStyle::SC_SliderGroove ? "groove" : "handle"] = QJsonArray{part.x(),part.y(),part.width(),part.height()};
            }
        }
        if (auto *list = qobject_cast<QListWidget *>(widget)) {
            QJsonArray fonts;
            for (int row = 0; row < list->count(); ++row) {
                const auto *entry = list->item(row);
                fonts.append(QJsonObject{{"text",entry->text()},{"family",entry->data(Qt::UserRole).toString()},
                    {"height",list->sizeHintForRow(row)},{"selected",entry->isSelected()}});
            }
            item["fonts"] = fonts;
        }
        if (auto *bar = qobject_cast<QScrollBar *>(widget)) {
            item["range"] = QJsonArray{bar->minimum(),bar->maximum()};
            item["value"] = bar->value(); item["page"] = bar->pageStep();
            QStyleOptionSlider option;
            option.initFrom(bar); option.styleObject = bar;
            option.orientation = bar->orientation(); option.minimum = bar->minimum(); option.maximum = bar->maximum();
            option.sliderPosition = bar->value(); option.sliderValue = bar->value(); option.pageStep = bar->pageStep();
            for (const auto control : {QStyle::SC_ScrollBarGroove,QStyle::SC_ScrollBarSlider,QStyle::SC_ScrollBarSubLine,QStyle::SC_ScrollBarAddLine}) {
                const QRect part = bar->style()->subControlRect(QStyle::CC_ScrollBar,&option,control,bar);
                item[QString::number(control)] = QJsonArray{part.x(),part.y(),part.width(),part.height()};
            }
        }
        item["children"] = children(widget);
        result.append(item);
    }
    return result;
}

/** 【属性对照】【缓冲保存】保存 root 的真实控件缓冲和几何到 prefix；返回成功状态，不调整控件尺寸。 */
bool save(QWidget *root, const QString &prefix)
{
    if (!root) return false;
    root->grab();
    if (root->layout()) root->layout()->activate();
    QApplication::processEvents();
    const QRect rect = root->geometry();
    QJsonObject result{{"geometry", QJsonArray{rect.x(),rect.y(),rect.width(),rect.height()}}, {"children", children(root)}};
    QFile output(prefix + ".json");
    if (!output.open(QIODevice::WriteOnly)) return false;
    output.write(QJsonDocument(result).toJson());
    return root->grab().save(prefix + ".png");
}

/** 【属性对照】【程序入口】argv 提供输出前缀；创建隔离配置后打开原版文字工具，返回探针状态，不运行实际桌面捕获。 */
int main(int argc, char **argv)
{
    QTemporaryDir isolated;
    const QString realHome = qEnvironmentVariable("HOME");
    if (!markshot::test::isolateConfig(isolated)) return 2;
    // 1. 【属性对照】【字体环境】配置目录已经隔离，字体匹配继续读取实际系统与用户字库
    qputenv("HOME", realHome.toUtf8());
    qputenv("QT_FONT_DPI", "96");
    QApplication app(argc, argv);
    app.setApplicationName("mark-shot-property-reference");
    if (argc < 2 || argc > 3) return 3;
    const bool sizes = argc == 3 && QString::fromUtf8(argv[2]) == QStringLiteral("sizes");
    const bool axis = argc == 3 && QString::fromUtf8(argv[2]) == QStringLiteral("axis");
    const bool editing = argc == 3 && QString::fromUtf8(argv[2]) == QStringLiteral("editing");
    const bool menus = argc == 3 && QString::fromUtf8(argv[2]) == QStringLiteral("menus");
    const bool formats = argc == 3 && QString::fromUtf8(argv[2]) == QStringLiteral("formats");
    const bool caret = argc == 3 && QString::fromUtf8(argv[2]) == QStringLiteral("caret");
    const bool inputMethod = argc == 3 && QString::fromUtf8(argv[2]) == QStringLiteral("input-method");
    const bool lineInputMethod = argc == 3 && QString::fromUtf8(argv[2]) == QStringLiteral("line-input-method");
    if (!markshot::writeAppConfigRoot(QJsonObject{{"ui", QJsonObject{{"theme", argc == 3 && !sizes && !axis && !editing && !menus && !formats && !caret && !inputMethod && !lineInputMethod ? QString::fromUtf8(argv[2]) : QStringLiteral("dark")}}}})) return 8;
    const QString prefix = QString::fromUtf8(argv[1]);
    if (axis) {
        AxisReferenceWindow window(prefix);
        window.show();
        QTimer::singleShot(12000, &app, &QApplication::quit);
        return app.exec();
    }
    QImage source(1067,667,QImage::Format_ARGB32_Premultiplied);
    source.fill(QColor(30,34,38));
    if (lineInputMethod) {
        QFile output(prefix + "-line-input-method.json");
        if (!output.open(QIODevice::WriteOnly)) return 16;
        output.write(QJsonDocument(TextAnnotationEditingTest::lineInputMethod(source, prefix)).toJson());
        return 0;
    }
    if (inputMethod) {
        QFile output(prefix + "-input-method.json");
        if (!output.open(QIODevice::WriteOnly)) return 15;
        output.write(QJsonDocument(TextAnnotationEditingTest::inputMethod(source, prefix)).toJson());
        return 0;
    }
    if (caret) {
        QFile output(prefix + "-caret.json");
        if (!output.open(QIODevice::WriteOnly)) return 14;
        output.write(QJsonDocument(TextAnnotationEditingTest::caret(source,prefix)).toJson(QJsonDocument::Compact));
        return 0;
    }
    if (formats) {
        QFile output(prefix + "-formats.json");
        if (!output.open(QIODevice::WriteOnly)) return 13;
        output.write(QJsonDocument(TextAnnotationEditingTest::formats(source)).toJson());
        return 0;
    }
    if (menus) {
        QFile output(prefix + "-menus.json");
        if (!output.open(QIODevice::WriteOnly)) return 12;
        output.write(QJsonDocument(TextAnnotationEditingTest::menus(source,prefix)).toJson());
        return 0;
    }
    if (editing) {
        QFile output(prefix + "-editing.json");
        if (!output.open(QIODevice::WriteOnly)) return 11;
        output.write(QJsonDocument(TextAnnotationEditingTest::editing(source)).toJson());
        return 0;
    }
    if (sizes) {
        QFile output(prefix + "-sizes.json");
        if (!output.open(QIODevice::WriteOnly)) return 10;
        output.write(QJsonDocument(TextAnnotationEditingTest::sizes(source)).toJson());
        return 0;
    }
    ShotWindow window(source, "reference", QRect(0,0,1067,667), {}, false);
    window.setDefaultTools(ShotWindow::Tool::Text, ShotWindow::Tool::Text);
    window.startFullscreenAnnotation();
    window.resize(1067,667);
    window.show();
    QEventLoop settled;
    QTimer::singleShot(40, &settled, &QEventLoop::quit);
    settled.exec();
    window.grab();
    QApplication::processEvents();
    auto *properties = window.findChild<QWidget *>("annotationPropertyPanel");
    if (!save(properties, prefix + "-properties")) return 4;
    for (auto *button : window.findChildren<QPushButton *>()) {
        if (button->accessibleName() != "Text font") continue;
        button->click();
        QApplication::processEvents();
        auto *panel = window.findChild<QWidget *>("propertyFontPanel");
        if (!save(panel, prefix + "-font")) return 5;
        if (auto *spin = panel->findChild<QSpinBox *>()) {
            spin->setFocus();
            spin->selectAll();
            auto *line = spin->findChild<QLineEdit *>();
            QJsonArray trace;
            for (const auto &text : {QStringLiteral("4"), QStringLiteral("2"), QStringLiteral("0")}) {
                QKeyEvent event(QEvent::KeyPress, Qt::Key_0 + text.toInt(), Qt::NoModifier, text);
                QApplication::sendEvent(line, &event);
                trace.append(QJsonObject{{"input",text},{"text",line->text()},{"value",spin->value()}});
            }
            QFile traceFile(prefix + "-spin-input.json");
            if (traceFile.open(QIODevice::WriteOnly)) traceFile.write(QJsonDocument(trace).toJson());
        }
        if (!save(properties, prefix + "-properties-open")) return 7;
        window.grab().save(prefix + "-window.png");
        panel->hide();
        TextAnnotationEditingTest::select(window);
        if (!save(properties, prefix + "-selected-properties")) return 9;
        return 0;
    }
    return 6;
}
