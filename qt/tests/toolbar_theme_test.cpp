#include "app_config_store.h"
#include "shot_window.h"
#include "support/isolated_config.h"
#include "ui/compact_combo_box.h"
#include "ui/compact_toolbar.h"
#include "ui/context_menu.h"
#include "ui/icons.h"
#include "ui/interface_theme_config.h"
#include "ui/toolbar_palette.h"
#include "ui/toolbar_panel_style.h"

#include <QApplication>
#include <QJsonObject>
#include <QPainter>
#include <QPushButton>
#include <QTemporaryDir>
#include <QtTest>

#include <cstdlib>

namespace {

using markshot::ui::CompactComboBox;
using markshot::ui::CompactToolbar;
using markshot::ui::ContextMenu;
using markshot::ui::ToolbarPalette;
using markshot::ui::UiThemeMode;

/**
 * 判断两个颜色的 RGB 通道是否在容差内一致。
 * @param actual 实际颜色。
 * @param expected 期望颜色。
 * @param tolerance 每个通道允许的差值。
 * @return 全部通道在容差内时返回 true。
 */
bool sameRgb(const QColor &actual, const QColor &expected, int tolerance = 3)
{
    return std::abs(actual.red() - expected.red()) <= tolerance
        && std::abs(actual.green() - expected.green()) <= tolerance
        && std::abs(actual.blue() - expected.blue()) <= tolerance;
}

/**
 * 取图标渲染结果中最不透明像素的颜色，即图标前景色。
 * @param icon 待检查图标。
 * @return 最不透明像素的非预乘颜色。
 */
QColor iconInk(const QIcon &icon)
{
    const QImage image = icon.pixmap(QSize(22, 22)).toImage().convertToFormat(QImage::Format_ARGB32);
    QColor best(Qt::transparent);
    for (int y = 0; y < image.height(); ++y) {
        for (int x = 0; x < image.width(); ++x) {
            const QColor color = image.pixelColor(x, y);
            if (color.alpha() > best.alpha()) {
                best = color;
            }
        }
    }
    return best;
}

/**
 * 把工具栏绘制到透明画布，取左侧内边距处的底板颜色。
 * @param toolbar 截图主工具栏。
 * @return 底板像素颜色。
 */
QColor toolbarFill(CompactToolbar *toolbar)
{
    toolbar->adjustSize();
    QImage image(toolbar->size(), QImage::Format_ARGB32_Premultiplied);
    image.fill(Qt::transparent);
    toolbar->render(&image);
    return image.pixelColor(3, image.height() / 2);
}

}  // namespace

class ToolbarThemeTest final : public QObject {
    Q_OBJECT

private slots:
    /** 准备浅色、深色与跟随系统三种界面主题配置，无参数和返回值。 */
    void followsInterfaceTheme_data()
    {
        QTest::addColumn<QString>("theme");
        QTest::newRow("light") << QStringLiteral("light");
        QTest::newRow("dark") << QStringLiteral("dark");
        QTest::newRow("system") << QStringLiteral("system");
    }

    /** 验证截图工具栏、属性面板与下拉面板的配色跟随设置中的界面主题，无参数和返回值。 */
    void followsInterfaceTheme()
    {
        QFETCH(QString, theme);
        // 1. 写入界面主题后创建截图窗口，期望配色按实际明暗解析
        QVERIFY(markshot::writeAppConfigRoot({{QStringLiteral("ui"), QJsonObject{{QStringLiteral("theme"), theme}}}}));
        const UiThemeMode mode = markshot::ui::effectiveUiThemeMode(markshot::ui::uiThemeModeFromString(theme));
        const ToolbarPalette expected = markshot::ui::toolbarPalette(mode);
        QImage image(800, 600, QImage::Format_ARGB32_Premultiplied);
        image.fill(QColor(90, 110, 130));
        ShotWindow window(image, QString(), QRect(0, 0, 800, 600), QVector<markshot::WindowInfo>{}, false);
        window.setAttribute(Qt::WA_DeleteOnClose, false);

        // 2. 主工具栏底板明暗与配色一致，样式表由同一配色生成
        auto *toolbar = window.findChild<CompactToolbar *>(QStringLiteral("shotToolbar"));
        QVERIFY(toolbar);
        const QColor fill = toolbarFill(toolbar);
        QVERIFY2(expected.dark ? fill.lightness() < 60 : fill.lightness() > 200,
                 qPrintable(fill.name(QColor::HexArgb)));
        QCOMPARE(toolbar->styleSheet(), markshot::theme::panelStyleSheet(expected, 30, 11));
        if (theme == QStringLiteral("dark")) {
            QCOMPARE(toolbar->styleSheet(), markshot::theme::panelStyleSheet());
        }

        // 3. 普通工具图标使用配色前景，悬停时切换为强调色底上的前景
        QPushButton *pen = toolbar->button(markshot::ui::actionName(ShotWindow::Action::ToolPen));
        QVERIFY(pen);
        QVERIFY2(sameRgb(iconInk(pen->icon()), expected.text), qPrintable(iconInk(pen->icon()).name()));
        QEvent enter(QEvent::Enter);
        QCoreApplication::sendEvent(pen, &enter);
        QVERIFY2(sameRgb(iconInk(pen->icon()), expected.hoverIcon), qPrintable(iconInk(pen->icon()).name()));
        QEvent leave(QEvent::Leave);
        QCoreApplication::sendEvent(pen, &leave);
        QVERIFY(sameRgb(iconInk(pen->icon()), expected.text));

        // 4. 动作工具栏、属性面板与各下拉面板使用同一配色
        auto *actionToolbar = window.findChild<CompactToolbar *>(QStringLiteral("actionToolbar"));
        QVERIFY(actionToolbar);
        QVERIFY(actionToolbar->styleSheet().startsWith(toolbar->styleSheet()));
        auto *propertyPanel = window.findChild<QWidget *>(QStringLiteral("annotationPropertyPanel"));
        QVERIFY(propertyPanel);
        QCOMPARE(propertyPanel->styleSheet(), toolbar->styleSheet());
        const QString popupFill = markshot::ui::cssColor(expected.popupFill);
        for (const char *name : {"openWithPanel", "extensionPanel", "propertyFontPanel", "propertyColorDialogPanel"}) {
            auto *panel = window.findChild<QWidget *>(QString::fromLatin1(name));
            QVERIFY2(panel, name);
            QVERIFY2(panel->styleSheet().contains(popupFill), name);
        }

        // 5. 属性栏下拉框弹出的自绘菜单与工具栏明暗一致
        const auto combos = propertyPanel->findChildren<CompactComboBox *>();
        QVERIFY(!combos.isEmpty());
        CompactComboBox *combo = combos.first();
        combo->showPopup();
        auto *menu = combo->findChild<ContextMenu *>();
        QVERIFY(menu);
        QCOMPARE(menu->colorScheme(), expected.dark ? ContextMenu::ColorScheme::Dark : ContextMenu::ColorScheme::Light);
        combo->hidePopup();
    }
};

/**
 * 使用隔离配置运行工具栏主题回归。
 * @param argc 参数数量。
 * @param argv 参数列表。
 * @return 测试退出码。
 */
int main(int argc, char **argv)
{
    QTemporaryDir home;
    if (!markshot::test::isolateConfig(home)) return 1;
    QApplication app(argc, argv);
    ToolbarThemeTest test;
    return QTest::qExec(&test, argc, argv);
}

#include "toolbar_theme_test.moc"
