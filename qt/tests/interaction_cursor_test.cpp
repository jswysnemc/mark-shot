#include "ui/interaction_cursor.h"

#include <QComboBox>
#include <QLineEdit>
#include <QListWidget>
#include <QPushButton>
#include <QSlider>
#include <QTabBar>
#include <QTextEdit>
#include <QVBoxLayout>
#include <QtTest/QtTest>

class InteractionCursorTest final : public QObject {
    Q_OBJECT

private slots:
    /// @brief 为实际 Qt 控件安装全应用光标策略
    /// @return 无返回值
    void initTestCase()
    {
        markshot::ui::installInteractionCursorPolicy(qApp);
        markshot::ui::installInteractionCursorPolicy(qApp);
    }

    /// @brief 验证按钮、可编辑下拉框与文本编辑器不继承画布光标
    /// @return 无返回值
    void separatesActionAndTextCursors()
    {
        // 1. 将标准控件放入使用十字光标的父窗口
        QWidget window;
        window.setCursor(Qt::CrossCursor);
        auto *layout = new QVBoxLayout(&window);
        auto *button = new QPushButton(QStringLiteral("Action"), &window);
        auto *combo = new QComboBox(&window);
        combo->setEditable(true);
        combo->addItem(QStringLiteral("Editable text"));
        auto *editor = new QTextEdit(&window);
        layout->addWidget(button);
        layout->addWidget(combo);
        layout->addWidget(editor);
        window.show();
        QVERIFY(QTest::qWaitForWindowExposed(&window));
        // 2. 核对可操作区域、文本区和禁用状态各自的语义
        QCOMPARE(button->cursor().shape(), Qt::PointingHandCursor);
        QCOMPARE(combo->cursor().shape(), Qt::PointingHandCursor);
        QCOMPARE(combo->lineEdit()->cursor().shape(), Qt::IBeamCursor);
        QCOMPARE(editor->viewport()->cursor().shape(), Qt::IBeamCursor);
        button->setEnabled(false);
        QCOMPARE(button->cursor().shape(), Qt::ArrowCursor);
        button->setEnabled(true);
        QCOMPARE(button->cursor().shape(), Qt::PointingHandCursor);
        editor->setEnabled(false);
        QCOMPARE(editor->viewport()->cursor().shape(), Qt::ArrowCursor);
        // 3. 策略只作用于控件，不覆盖画布或安装全应用等待光标
        QCOMPARE(window.cursor().shape(), Qt::CrossCursor);
        QVERIFY(QApplication::overrideCursor() == nullptr);
    }

    /// @brief 验证滑块按下、非主键释放、隐藏和禁用时的捕获语义
    /// @return 无返回值
    void restoresSliderCursorAfterInteraction()
    {
        // 1. 开始拖动后使用闭合手型
        QSlider slider(Qt::Horizontal);
        slider.resize(260, 32);
        slider.show();
        QVERIFY(QTest::qWaitForWindowExposed(&slider));
        QCOMPARE(slider.cursor().shape(), Qt::OpenHandCursor);
        QTest::mousePress(&slider, Qt::LeftButton);
        QCOMPARE(slider.cursor().shape(), Qt::ClosedHandCursor);
        // 2. 释放其他按键不会结束左键拖动，释放左键后恢复手型
        QTest::mousePress(&slider, Qt::RightButton);
        QTest::mouseRelease(&slider, Qt::RightButton);
        QCOMPARE(slider.cursor().shape(), Qt::ClosedHandCursor);
        QTest::mouseRelease(&slider, Qt::LeftButton);
        QCOMPARE(slider.cursor().shape(), Qt::OpenHandCursor);
        // 3. 隐藏或禁用控件后不遗留拖动光标
        QTest::mousePress(&slider, Qt::LeftButton);
        slider.hide();
        QCOMPARE(slider.cursor().shape(), Qt::OpenHandCursor);
        QTest::mouseRelease(&slider, Qt::LeftButton);
        slider.show();
        slider.setEnabled(false);
        QCOMPARE(slider.cursor().shape(), Qt::ArrowCursor);
    }

    /// @brief 验证业务握柄保留自己的拖动光标，禁用后显示箭头
    /// @return 无返回值
    void respectsCustomDragHandles()
    {
        // 1. 业务握柄保留已经设置的抓取光标
        QWidget handle;
        handle.setProperty("dragHandle", true);
        handle.setCursor(Qt::ClosedHandCursor);
        handle.show();
        QVERIFY(QTest::qWaitForWindowExposed(&handle));
        QCOMPARE(handle.cursor().shape(), Qt::ClosedHandCursor);
        // 2. 禁用时使用箭头，重新启用时恢复可拖动状态
        handle.setEnabled(false);
        QCOMPARE(handle.cursor().shape(), Qt::ArrowCursor);
        handle.setEnabled(true);
        QCOMPARE(handle.cursor().shape(), Qt::OpenHandCursor);
    }

    /// @brief 验证标记了内边距输入范围的文本编辑器整框显示插入光标，禁用后恢复箭头
    /// @return 无返回值
    void textCursorCoversMarkedEditorPadding()
    {
        // 1. 普通文本编辑器的边框区域仍是箭头，标记后整框都是插入光标
        QWidget window;
        auto *layout = new QVBoxLayout(&window);
        auto *plain = new QTextEdit(&window);
        auto *inlineEditor = new QTextEdit(&window);
        inlineEditor->setProperty("textCursorInPadding", true);
        layout->addWidget(plain);
        layout->addWidget(inlineEditor);
        window.show();
        QVERIFY(QTest::qWaitForWindowExposed(&window));
        QCOMPARE(plain->cursor().shape(), Qt::ArrowCursor);
        QCOMPARE(inlineEditor->cursor().shape(), Qt::IBeamCursor);
        QCOMPARE(inlineEditor->viewport()->cursor().shape(), Qt::IBeamCursor);
        // 2. 禁用后与其他控件一致显示箭头
        inlineEditor->setEnabled(false);
        QCOMPARE(inlineEditor->cursor().shape(), Qt::ArrowCursor);
    }

    /// @brief 验证标签页和导航列表仅在可操作项目上显示手型
    /// @return 无返回值
    void distinguishesActiveItemsFromEmptySpace()
    {
        // 1. 核对启用、禁用标签页和标签栏空白
        QTabBar tabs;
        tabs.setExpanding(false);
        tabs.addTab(QStringLiteral("Text"));
        tabs.addTab(QStringLiteral("Image"));
        tabs.setTabEnabled(1, false);
        tabs.resize(420, 40);
        tabs.show();
        QVERIFY(QTest::qWaitForWindowExposed(&tabs));
        // 2. 【界面测试】【原生指针】原生平台异步投递移动事件，等待悬停光标更新
        QTest::mouseMove(&tabs, tabs.tabRect(0).center());
        QTRY_COMPARE(tabs.cursor().shape(), Qt::PointingHandCursor);
        QTest::mouseMove(&tabs, tabs.tabRect(1).center());
        QTRY_COMPARE(tabs.cursor().shape(), Qt::ArrowCursor);
        QTest::mouseMove(&tabs, QPoint(400, 20));
        QTRY_COMPARE(tabs.cursor().shape(), Qt::ArrowCursor);
        tabs.hide();
        // 3. 核对导航项、不可选择项目与列表空白
        QListWidget list;
        list.setProperty("actionList", true);
        list.addItem(QStringLiteral("Settings"));
        list.addItem(QStringLiteral("Separator"));
        list.item(1)->setFlags(Qt::NoItemFlags);
        list.resize(260, 240);
        list.show();
        QVERIFY(QTest::qWaitForWindowExposed(&list));
        QTest::mouseMove(list.viewport(), list.visualItemRect(list.item(0)).center());
        QTRY_COMPARE(list.viewport()->cursor().shape(), Qt::PointingHandCursor);
        QTest::mouseMove(list.viewport(), list.visualItemRect(list.item(1)).center());
        QTRY_COMPARE(list.viewport()->cursor().shape(), Qt::ArrowCursor);
        QTest::mouseMove(list.viewport(), QPoint(100, 200));
        QTRY_COMPARE(list.viewport()->cursor().shape(), Qt::ArrowCursor);
    }
};

QTEST_MAIN(InteractionCursorTest)
#include "interaction_cursor_test.moc"
