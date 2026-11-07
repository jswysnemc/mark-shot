#include "selection_translate/selection_translate_placement.h"
#include "selection_translate/translate_language_popup.h"
#include "selection_translate/translate_result_bar.h"
#include "ui/i18n.h"
#include "ui/theme.h"

#include <QApplication>
#include <QFile>
#include <QFontInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QKeyEvent>
#include <QMouseEvent>
#include <QPushButton>

namespace markshot::i18n {
/** 【框选翻译对照】【固定语言】按 source 返回英文标签，本探针固定英文，生产控件与布局直接链接原版 */
QString translate(const QString &source) { return source; }
}

/** 【框选翻译对照】【矩形编码】按 rect 返回整数矩形，无坐标换算 */
QJsonArray box(const QRect &rect) { return {rect.x(), rect.y(), rect.width(), rect.height()}; }

/** 【框选翻译对照】【事件发送】向 widget 发送 type、button 和 point 对应事件，不通过内部私有方法改变状态 */
void mouse(QWidget &widget, QEvent::Type type, Qt::MouseButton button, QPointF point)
{
    QMouseEvent event(type, point, widget.mapToGlobal(point.toPoint()), button,
                     type == QEvent::MouseButtonPress ? button : Qt::NoButton, Qt::NoModifier);
    QApplication::sendEvent(&widget, &event);
}

/** 【框选翻译对照】【独立采集】按 argv 输出生产控件尺寸、输入及面板避让结果，不修改原仓库 */
int main(int argc, char **argv)
{
    QApplication app(argc, argv);
    app.setStyle("Fusion");
    namespace translate = markshot::selection_translate;
    QJsonObject output{{"qt", QT_VERSION_STR}, {"font", QFontInfo(markshot::theme::uiFont(10)).family()}};
    const QStringList names{"Simplified Chinese", "Traditional Chinese", "English", "Japanese", "Korean", "French", "German", "Spanish", "Russian"};
    QVector<markshot::TranslationLanguageOption> options;
    for (const QString &name : names) { options.append({name, name}); }
    translate::TranslateLanguagePopup popup(nullptr);
    popup.setLanguages(options, "English");
    output["popup_size"] = QJsonArray{popup.sizeHint().width(), popup.sizeHint().height()};
    popup.setLanguages(options, "Esperanto");
    output["custom_popup_size"] = QJsonArray{popup.sizeHint().width(), popup.sizeHint().height()};
    QJsonArray chosen;
    QObject::connect(&popup, &translate::TranslateLanguagePopup::languageChosen, [&chosen](const QString &value) { chosen.append(value); });
    popup.setLanguages(options, "English");
    popup.popup(QRect(QPoint(20, 20), popup.sizeHint()));
    mouse(popup, QEvent::MouseButtonPress, Qt::RightButton, QPointF(20, 24 + 3 * 26 + 13));
    mouse(popup, QEvent::MouseButtonRelease, Qt::RightButton, QPointF(20, 24 + 3 * 26 + 13));
    output["right_kept_popup"] = popup.isVisible() && chosen.isEmpty();
    QKeyEvent key(QEvent::KeyPress, Qt::Key_Return, Qt::NoModifier);
    QApplication::sendEvent(&popup, &key);
    output["chosen"] = chosen;

    QJsonArray bars;
    for (const QString &label : {QString("English"), QString("Traditional Chinese"), QString("Esperanto"), QString("Japanese"), QString("French")}) {
        translate::TranslateResultBar bar(markshot::ui::toolbarPalette(markshot::ui::UiThemeMode::Light), nullptr);
        bar.setLanguageLabel(label);
        bar.show();
        app.processEvents();
        QJsonArray buttons;
        for (const QString &name : {QString("translateLanguageButton"), QString("translateOriginalButton"), QString("translateTranslatedButton"), QString("translateCopyButton"), QString("translateExitButton")}) {
            const auto *button = bar.findChild<QPushButton *>(name);
            buttons.append(QJsonObject{{"name", name}, {"rect", box(button->geometry())}});
        }
        bars.append(QJsonObject{{"label", label}, {"size", QJsonArray{bar.width(), bar.height()}}, {"buttons", buttons}, {"text_advance", bar.fontMetrics().horizontalAdvance(label)}, {"text_size", bar.fontMetrics().size(Qt::TextShowMnemonic, label).width()}});
    }
    output["bars"] = bars;
    QJsonArray languages;
    const QRect bounds(0, 0, 800, 600);
    for (const QRect &anchor : {QRect(20, 20, 30, 30), QRect(740, 20, 30, 30), QRect(20, 540, 30, 30), QRect(740, 540, 30, 30)}) {
        for (bool vertical : {false, true}) {
            const QSize size(174, 262);
            languages.append(QJsonObject{{"anchor", box(anchor)}, {"bounds", box(bounds)}, {"size", QJsonArray{size.width(), size.height()}}, {"vertical", vertical}, {"result", box(translate::languagePopupGeometry(anchor, vertical, size, bounds))}});
        }
    }
    output["languages"] = languages;
    QJsonArray positions;
    for (const QRect &region : {QRect(100, 100, 400, 200), QRect(0, 0, 800, 600), QRect(10, 10, 50, 50), QRect(600, 400, 200, 200), QRect(200, 20, 400, 200)}) {
        for (const QVector<QRect> &obstacles : {QVector<QRect>{}, QVector<QRect>{QRect(250, 60, 250, 30)}, QVector<QRect>{QRect(200, 50, 350, 50), QRect(200, 300, 350, 50)}, QVector<QRect>{QRect(400, 50, 100, 500), QRect(0, 0, 180, 600)}}) {
            QJsonArray blocks;
            for (const QRect &obstacle : obstacles) { blocks.append(box(obstacle)); }
            const QSize size(310, 30);
            positions.append(QJsonObject{{"region", box(region)}, {"bounds", box(bounds)}, {"size", QJsonArray{size.width(), size.height()}}, {"obstacles", blocks}, {"result", box(translate::resultBarGeometry(region, size, bounds, obstacles))}});
        }
    }
    output["positions"] = positions;
    QFile file(argv[1]);
    if (!file.open(QIODevice::WriteOnly)) { return 2; }
    file.write(QJsonDocument(output).toJson());
    return 0;
}
