// 【标注多选界面对照】【原生控件】按原版setup控件尺寸构造群组面板，直接编译原版完整样式与配色文件
#include "ui/toolbar_panel_style.h"
#include <QApplication>
#include <QHBoxLayout>
#include <QLabel>
#include <QSlider>
#include <QPushButton>
#include <QFile>
#include <QJsonArray>
#include <QJsonObject>
#include <QJsonDocument>
#include <QFontMetricsF>

/** 【标注多选界面对照】【矩形输出】按widget返回原生布局已经计算的矩形，无替代布局算法 */
QJsonArray rect(const QWidget *widget) { const auto r = widget->geometry(); return {r.x(), r.y(), r.width(), r.height()}; }
/** 【标注多选界面对照】【布局样本】argc与argv提供输出路径，返回零表示全部控件几何写入成功 */
int main(int argc, char **argv) {
    QApplication app(argc, argv);
    app.setFont(QFont(QStringLiteral("Noto Sans")));
    QJsonArray samples;
    for (bool dark : {false, true}) for (const QString &language : {QStringLiteral("en"), QStringLiteral("zh")}) for (int count : {2, 9, 10, 99, 100, 999, 1000, 1000000}) {
        QWidget panel;
        panel.setObjectName(QStringLiteral("annotationPropertyPanel"));
        panel.setStyleSheet(markshot::theme::panelStyleSheet(markshot::ui::toolbarPalette(dark ? markshot::ui::UiThemeMode::Dark : markshot::ui::UiThemeMode::Light), 30, 11));
        auto layout = new QHBoxLayout(&panel); layout->setContentsMargins(8, 6, 8, 6); layout->setSpacing(6);
        const QString text = (language == QStringLiteral("zh") ? QStringLiteral("组合 %1") : QStringLiteral("Group %1")).arg(count);
        auto title = new QLabel(text, &panel);
        title->setObjectName(QStringLiteral("propertyTitle")); title->setAlignment(Qt::AlignCenter); title->setMinimumWidth(58);
        layout->addWidget(title); layout->addSpacing(2);
        QList<QLabel *> glyphs, values;
        QList<QSlider *> sliders;
        for (int index = 0; index < 2; ++index) {
            auto glyph = new QLabel(&panel); glyph->setObjectName(QStringLiteral("propertyGlyph")); glyph->setAlignment(Qt::AlignCenter);
            QPixmap icon(19, 19); icon.fill(Qt::transparent); glyph->setPixmap(icon); layout->addWidget(glyph); glyphs.append(glyph);
            auto value = new QLabel(index == 0 ? QStringLiteral("3") : QStringLiteral("100%"), &panel);
            value->setObjectName(QStringLiteral("propertyValue")); value->setAlignment(Qt::AlignRight | Qt::AlignVCenter); value->setFixedWidth(index == 0 ? 34 : 36);
            auto slider = new QSlider(Qt::Horizontal, &panel); slider->setFixedWidth(index == 0 ? 88 : 76); slider->setRange(index == 0 ? 1 : 0, index == 0 ? 24 : 100);
            layout->addWidget(slider); layout->addWidget(value); layout->addSpacing(2); sliders.append(slider); values.append(value);
        }
        auto button = new QPushButton(&panel); button->setIconSize(QSize(22, 22)); layout->addWidget(button);
        panel.ensurePolished(); panel.adjustSize(); panel.show(); app.processEvents(); layout->activate();
        QJsonArray glyphRects, valueRects, controlRects;
        for (const auto glyph : glyphs) glyphRects.append(rect(glyph));
        for (const auto value : values) valueRects.append(rect(value));
        for (const auto slider : sliders) controlRects.append(rect(slider));
        controlRects.append(rect(button));
        const QFontMetricsF metrics(title->font());
        samples.append(QJsonObject{{"count", count}, {"language", language}, {"text", text}, {"dark", dark}, {"size", QJsonArray{panel.width(), panel.height()}}, {"title", rect(title)}, {"glyphs", glyphRects}, {"values", valueRects}, {"controls", controlRects}, {"title_hint", QJsonArray{title->sizeHint().width(), title->sizeHint().height()}}, {"font", title->font().toString()}, {"advance", metrics.horizontalAdvance(text)}, {"bounding", metrics.boundingRect(QRectF(0, 0, 2000, 2000), Qt::AlignCenter, text).width()}});
    }
    QFile output(QString::fromLocal8Bit(argv[1])); if (!output.open(QIODevice::WriteOnly)) return 3;
    output.write(QJsonDocument(QJsonObject{{"qt", QT_VERSION_STR}, {"samples", samples}}).toJson());
    return 0;
}
