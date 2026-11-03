#include "shot_window.h"
#include "support/isolated_config.h"
#include "ui/i18n.h"

#include <QApplication>
#include <QHash>
#include <QLineEdit>
#include <QListWidget>
#include <QPointer>
#include <QPushButton>
#include <QSet>
#include <QSpinBox>
#include <QTemporaryDir>
#include <QTextEdit>
#include <QToolButton>
#include <QtTest/QtTest>

namespace {

/** 【界面测试】【工具操作】通过真实按钮选择 window 的 action 工具，返回是否找到按钮。 */
bool chooseTool(ShotWindow &window, const char *action)
{
    for (auto *button : window.findChildren<QPushButton *>()) {
        if (button->property("action").toString() == QLatin1String(action)) {
            button->click();
            return true;
        }
    }
    return false;
}

/** 【界面测试】【字体入口】按无障碍名称查找 window 的字体按钮，找不到返回空指针。 */
QPushButton *fontButton(ShotWindow &window)
{
    for (auto *button : window.findChildren<QPushButton *>()) {
        if (button->accessibleName() == MS_TR("Text font")) return button;
    }
    return nullptr;
}

/** 【界面测试】【测试图】返回固定颜色的小图，不读取桌面或剪贴板，无参数。 */
QImage testImage()
{
    QImage image(800, 600, QImage::Format_RGB32);
    image.fill(Qt::white);
    return image;
}

}  // namespace

class QtUiMemoryTest final : public QObject {
    Q_OBJECT

private slots:
    /** 【界面测试】【延迟字体】截图、普通标注及默认文字输入都不创建字体列表，无参数和返回值。 */
    void ordinaryToolsDoNotCreateFontPanel()
    {
        ShotWindow window(testImage(), {}, {}, {}, false);
        QVERIFY(!window.findChild<QWidget *>(QStringLiteral("propertyFontPanel")));
        window.resize(800, 600);
        window.startFullscreenAnnotation();
        window.show();
        QVERIFY(QTest::qWaitForWindowExposed(&window));
        QVERIFY(chooseTool(window, "Pen"));
        QVERIFY(chooseTool(window, "Text"));
        QTest::mouseClick(&window, Qt::LeftButton, Qt::NoModifier, QPoint(200, 200));
        auto *editor = window.findChild<QTextEdit *>();
        QVERIFY(editor);
        QTRY_VERIFY(editor->isVisible());
        QTest::keyClicks(editor, "plain text");
        QCOMPARE(editor->toPlainText(), QStringLiteral("plain text"));
        QVERIFY(chooseTool(window, "Pen"));
        QVERIFY(!window.findChild<QWidget *>(QStringLiteral("propertyFontPanel")));
    }

    /** 【界面测试】【字体交互】首次打开可搜索和设置样式，重复打开不增加面板，窗口销毁释放面板，无参数和返回值。 */
    void fontPanelWorksOnFirstUseAndIsReused()
    {
        auto window = std::make_unique<ShotWindow>(testImage(), QString(), QRect(),
                                                  QVector<markshot::WindowInfo>(), false);
        window->resize(800, 600);
        window->startFullscreenAnnotation();
        window->show();
        QVERIFY(QTest::qWaitForWindowExposed(window.get()));
        QVERIFY(chooseTool(*window, "Text"));
        auto *button = fontButton(*window);
        QVERIFY(button);
        button->click();
        QPointer<QWidget> panel = window->findChild<QWidget *>(QStringLiteral("propertyFontPanel"));
        QVERIFY(panel);
        QTRY_VERIFY(panel->isVisible());
        auto *list = panel->findChild<QListWidget *>();
        auto *search = panel->findChild<QLineEdit *>(QString(), Qt::FindDirectChildrenOnly);
        auto *size = panel->findChild<QSpinBox *>();
        QVERIFY(list && list->count() > 0);
        QVERIFY(search && size);
        const QString firstVisibleFamily = list->indexAt(QPoint(5, 5)).data(Qt::UserRole).toString();
        QVERIFY(!firstVisibleFamily.isEmpty());
        QSet<qint64> iconKeys;
        for (int i = 0; i < list->count(); ++i) {
            auto *item = list->item(i);
            if (!item->data(Qt::UserRole).toString().isEmpty()) iconKeys.insert(item->icon().cacheKey());
        }
        QVERIFY(iconKeys.size() <= 2);
        size->setValue(42);
        QHash<QString, bool> expectedStyles;
        for (auto *style : panel->findChildren<QToolButton *>()) {
            if (style->text() == QStringLiteral("B") || style->text() == QStringLiteral("I")) {
                style->click();
                expectedStyles.insert(style->text(), style->isChecked());
            }
        }
        search->setText(QStringLiteral("mark-shot-nonexistent-font-987654"));
        for (int i = 0; i < list->count(); ++i) QVERIFY(list->item(i)->isHidden());
        button->click();
        QVERIFY(!panel->isVisible());
        button->click();
        QVERIFY(panel->isVisible());
        QCOMPARE(search->text(), QString());
        QCOMPARE(list->indexAt(QPoint(5, 5)).data(Qt::UserRole).toString(), firstVisibleFamily);
        QCOMPARE(size->value(), 42);
        QCOMPARE(expectedStyles.size(), 2);
        for (auto *style : panel->findChildren<QToolButton *>()) {
            if (expectedStyles.contains(style->text())) QCOMPARE(style->isChecked(), expectedStyles.value(style->text()));
        }
        QSet<qint64> reopenedKeys;
        for (int i = 0; i < list->count(); ++i) {
            auto *item = list->item(i);
            if (!item->data(Qt::UserRole).toString().isEmpty()) reopenedKeys.insert(item->icon().cacheKey());
        }
        QCOMPARE(reopenedKeys, iconKeys);
        const int rowHeight = std::max(28, qRound(28.0 * list->logicalDpiY() / 96.0));
        QCOMPARE(list->sizeHintForRow(0), rowHeight);
        QCOMPARE(list->sizeHintForRow(list->count() - 1), rowHeight);
        QCOMPARE(window->findChildren<QWidget *>(QStringLiteral("propertyFontPanel")).size(), 1);
        QVERIFY(window->rect().contains(panel->geometry()));
        QListWidgetItem *target = nullptr;
        for (int i = 0; i < list->count(); ++i) {
            if (!list->item(i)->data(Qt::UserRole).toString().isEmpty()) {
                target = list->item(i);
                break;
            }
        }
        QVERIFY(target);
        const QString family = target->data(Qt::UserRole).toString();
        search->setText(family);
        list->scrollToItem(target);
        QCoreApplication::processEvents();
        const QPoint position = list->visualItemRect(target).center();
        QVERIFY(list->viewport()->rect().contains(position));
        QTest::mouseClick(list->viewport(), Qt::LeftButton, Qt::NoModifier, position);
        QTRY_VERIFY(!panel->isVisible());
        button->click();
        QVERIFY(list->currentItem());
        QCOMPARE(list->currentItem()->data(Qt::UserRole).toString(), family);
        QCOMPARE(window->findChildren<QWidget *>(QStringLiteral("propertyFontPanel")).size(), 1);
        window.reset();
        QVERIFY(panel.isNull());
    }
};

/** 【界面测试】【启动】隔离配置后执行 argc/argv 指定的测试，返回测试退出码。 */
int main(int argc, char **argv)
{
    QTemporaryDir home;
    if (!markshot::test::isolateConfig(home)) return 1;
    QApplication app(argc, argv);
    QtUiMemoryTest test;
    return QTest::qExec(&test, argc, argv);
}

#include "qt_ui_memory_test.moc"
