// 【界面对照】【原版渲染】直接编译原版 Qt 控件，仅用于生成几何与像素证据

#include "ui/color_picker.h"
#include "ui/color_history_store.h"
#include "ui/theme.h"
#include "ui/toolbar_panel_style.h"
#include <QApplication>
#include <QDir>
#include <QFile>
#include <QFontInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLineEdit>
#include <QPainter>
#include <QPushButton>
#include <QVBoxLayout>
#include <cmath>

/** 【界面对照】【矩形序列化】rect 为控件本地矩形，返回 x、y、width、height 数组。 */
QJsonArray geometry(const QRect &rect)
{
    return {rect.x(), rect.y(), rect.width(), rect.height()};
}

/** 【界面对照】【环形构造】在 panel 中复用原版配色、按钮约束和位置公式，无返回值。 */
void radial(QWidget &panel)
{
    panel.resize(178, 178);
    panel.setObjectName(QStringLiteral("colorPalette"));
    panel.setStyleSheet(markshot::theme::colorPaletteStyleSheet());
    const QVector<QColor> colors = markshot::theme::paletteColors();
    for (int index = 0; index < colors.size(); ++index) {
        auto *button = new QPushButton(&panel);
        button->setFocusPolicy(Qt::NoFocus);
        button->setStyleSheet(QStringLiteral("background: %1;").arg(colors.at(index).name()));
        const double angle = -M_PI / 2.0 + 2.0 * M_PI * index / colors.size();
        button->setGeometry(qRound(89 + std::cos(angle) * 68 - 15),
                            qRound(89 + std::sin(angle) * 68 - 15), 30, 30);
    }
    auto *preview = new QWidget(&panel);
    preview->setObjectName(QStringLiteral("colorPalettePreview"));
    preview->setGeometry(85, 85, 8, 8);
    preview->setStyleSheet(QStringLiteral("background:#FF4D4D; border:0; border-radius:3px;"));
}

/** 【界面对照】【控件证据】读取 argc/argv 的模式、目录和颜色，输出 PNG 与实际子控件 JSON，返回进程状态。 */
int main(int argc, char **argv)
{
    QApplication app(argc, argv);
    if (argc < 3) return 2;
    const QString mode = QString::fromUtf8(argv[1]);
    const QDir output(QString::fromUtf8(argv[2]));
    QDir().mkpath(output.path());
    QWidget host;
    host.resize(640,480);
    host.setFocusPolicy(Qt::StrongFocus);
    QWidget panel(&host);
    panel.setFont(markshot::theme::uiFont());
    if (mode == QStringLiteral("radial")) {
        radial(panel);
    } else {
        const auto palette = markshot::ui::toolbarPalette(mode == QStringLiteral("light")
            ? markshot::ui::UiThemeMode::Light : markshot::ui::UiThemeMode::Dark);
        panel.setObjectName(QStringLiteral("propertyColorDialogPanel"));
        panel.setStyleSheet(markshot::theme::propertyColorDialogPanelStyleSheet(palette));
        auto *layout = new QVBoxLayout(&panel);
        layout->setContentsMargins(8, 8, 8, 8);
        layout->setSpacing(0);
        auto *picker = new markshot::ui::ColorPicker(&panel);
        picker->setToolbarPalette(palette);
        picker->setColor(argc > 3 ? markshot::ui::colorHistoryColorFromString(QString::fromUtf8(argv[3])).value_or(QColor(255,77,77)) : QColor(255, 77, 77));
        layout->addWidget(picker);
        panel.adjustSize();
    }
    host.show();
    panel.show();
    host.setFocus();
    app.processEvents();
    const double scale = panel.devicePixelRatioF();
    QImage image(qCeil(panel.width() * scale), qCeil(panel.height() * scale), QImage::Format_ARGB32_Premultiplied);
    image.setDevicePixelRatio(scale);
    image.fill(Qt::transparent);
    panel.render(&image, QPoint(), QRegion(), QWidget::DrawChildren);
    if (!image.save(output.filePath(mode + QStringLiteral(".png")))) return 3;
    QJsonArray children;
    for (QWidget *child : panel.findChildren<QWidget *>()) {
        children.append(QJsonObject{{"class", child->metaObject()->className()},
            {"name", child->objectName()}, {"geometry", geometry(QRect(child->mapTo(&panel, QPoint()), child->size()))},
            {"visible", child->isVisible()}, {"enabled", child->isEnabled()},
            {"font", QFontInfo(child->font()).family()}, {"pixel_size", QFontInfo(child->font()).pixelSize()}});
    }
    QJsonObject result{{"panel", geometry(panel.rect())}, {"scale", scale}, {"children", children}};
    if (auto *picker = panel.findChild<markshot::ui::ColorPicker *>()) {
        const QColor color = picker->color();
        result.insert(QStringLiteral("rgba"), QJsonArray{color.red(), color.green(), color.blue(), color.alpha()});
        result.insert(QStringLiteral("hsv"), QJsonArray{color.hsvHue(), color.hsvSaturation(), color.value()});
    }
    QFile file(output.filePath(mode + QStringLiteral(".json")));
    if (!file.open(QIODevice::WriteOnly)) return 4;
    file.write(QJsonDocument(result).toJson());
    return 0;
}
