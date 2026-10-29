#include "selection_history.h"
#include "shot_window.h"
#include "support/isolated_config.h"

#include <QApplication>
#include <QKeyEvent>
#include <QPushButton>
#include <QTemporaryDir>
#include <QtTest>

namespace {

/**
 * 向截图窗口发送一次按键。
 * @param window 截图窗口。
 * @param key 按键。
 * @return 无返回值。
 */
void pressKey(ShotWindow &window, Qt::Key key)
{
    QKeyEvent press(QEvent::KeyPress, key, Qt::NoModifier);
    QCoreApplication::sendEvent(&window, &press);
    QTest::qWait(20);
}

/**
 * 创建带一个窗口识别候选的截图窗口。
 * @param image 冻结帧。
 * @return 截图窗口。
 */
std::unique_ptr<ShotWindow> createWindow(const QImage &image)
{
    // 1. 悬停识别候选覆盖整个窗口区域，复现指针总落在某个窗口上的真实场景
    auto window = std::make_unique<ShotWindow>(image, QString(), QRect(0, 0, 800, 600),
                                               QVector<markshot::WindowInfo>{{QRect(40, 40, 700, 500), 1}}, true);
    window->resize(800, 600);
    window->show();
    return window;
}

}  // namespace

class SelectionHistoryRestoreTest final : public QObject {
    Q_OBJECT
private slots:
    /** 写入两条历史，供后续用例按最新在前的顺序浏览，无参数和返回值。 */
    void initTestCase()
    {
        QVERIFY(markshot::rememberSelection(QRect(100, 120, 300, 200)));
        QVERIFY(markshot::rememberSelection(QRect(200, 150, 240, 160)));
    }

    /** 验证逗号恢复上一次选区后直接进入编辑并显示工具栏，点击不会被当作重新框选，无参数和返回值。 */
    void commaRestoresIntoEditing()
    {
        QImage image(800, 600, QImage::Format_ARGB32_Premultiplied);
        image.fill(QColor(90, 110, 130));
        auto window = createWindow(image);
        QVERIFY(QTest::qWaitForWindowExposed(window.get()));
        window->applyHoveredRect(QRect(40, 40, 700, 500));

        pressKey(*window, Qt::Key_Comma);
        QCOMPARE(window->m_mode, ShotWindow::Mode::Editing);
        QCOMPARE(window->normalizedSelection().toRect(), QRect(200, 150, 240, 160));
        QVERIFY(!window->m_hoveredWindowRect.has_value());
        QVERIFY(window->m_controls.m_toolbar->isVisible());
        QVERIFY(window->m_controls.m_actionToolbar->isVisible());
    }

    /** 验证编辑阶段继续按逗号、句号可在历史间切换，无参数和返回值。 */
    void keepsBrowsingWhileEditing()
    {
        QImage image(800, 600, QImage::Format_ARGB32_Premultiplied);
        image.fill(QColor(90, 110, 130));
        auto window = createWindow(image);
        QVERIFY(QTest::qWaitForWindowExposed(window.get()));

        pressKey(*window, Qt::Key_Comma);
        pressKey(*window, Qt::Key_Comma);
        QCOMPARE(window->normalizedSelection().toRect(), QRect(100, 120, 300, 200));
        pressKey(*window, Qt::Key_Period);
        QCOMPARE(window->normalizedSelection().toRect(), QRect(200, 150, 240, 160));
        QCOMPARE(window->m_mode, ShotWindow::Mode::Editing);
    }

    /** 验证已有标注时不再切换选区，避免已画内容偏离选区，无参数和返回值。 */
    void keepsSelectionOnceAnnotated()
    {
        QImage image(800, 600, QImage::Format_ARGB32_Premultiplied);
        image.fill(QColor(90, 110, 130));
        auto window = createWindow(image);
        QVERIFY(QTest::qWaitForWindowExposed(window.get()));

        pressKey(*window, Qt::Key_Comma);
        window->m_annotations.append(markshot::shot::types::Annotation{});
        pressKey(*window, Qt::Key_Comma);
        QCOMPARE(window->normalizedSelection().toRect(), QRect(200, 150, 240, 160));
    }
};

/**
 * 使用隔离配置运行真实窗口回归。
 * @param argc 参数数量。
 * @param argv 参数列表。
 * @return 测试退出码。
 */
int main(int argc, char **argv)
{
    QTemporaryDir home;
    if (!markshot::test::isolateConfig(home)) return 1;
    QApplication app(argc, argv);
    SelectionHistoryRestoreTest test;
    return QTest::qExec(&test, argc, argv);
}

#include "selection_history_restore_test.moc"
