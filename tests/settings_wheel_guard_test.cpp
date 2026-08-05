#include "settings/settings_wheel_guard.h"

#include <QApplication>
#include <QDialog>
#include <QLabel>
#include <QScrollArea>
#include <QScrollBar>
#include <QSpinBox>
#include <QVBoxLayout>
#include <QWheelEvent>
#include <QtTest/QtTest>

namespace {

/// @brief 构造一个发送给指定控件的滚轮事件。
/// @param target 接收事件的控件。
/// @param angleDeltaY 纵向滚轮角度增量（每格 120）。
/// @param pixelDeltaY 纵向像素增量（0 表示不携带像素增量）。
/// @param modifiers 事件修饰键。
/// @param inverted 事件是否标记为反向（macOS 自然滚动）。
/// @return 构造好的滚轮事件。
QWheelEvent makeWheelEvent(QWidget *target,
                           int angleDeltaY,
                           int pixelDeltaY = 0,
                           Qt::KeyboardModifiers modifiers = Qt::NoModifier,
                           bool inverted = false)
{
    const QPointF pos = target->rect().center();
    const QPointF globalPos = target->mapToGlobal(pos.toPoint());
    return QWheelEvent(pos,
                       globalPos,
                       QPoint(0, pixelDeltaY),
                       QPoint(0, angleDeltaY),
                       Qt::NoButton,
                       modifiers,
                       Qt::NoScrollPhase,
                       inverted);
}

/// @brief 构造带数值框与滚动区域的测试对话框。
/// @param area 输出的滚动区域。
/// @param spin 输出的数值框。
/// @return 测试对话框。
QDialog *makeDialog(QScrollArea **area, QSpinBox **spin)
{
    auto *dialog = new QDialog;
    auto *scrollArea = new QScrollArea(dialog);
    auto *page = new QWidget;
    auto *layout = new QVBoxLayout(page);
    auto *spinBox = new QSpinBox(page);
    spinBox->setRange(0, 100);
    spinBox->setValue(10);
    layout->addWidget(spinBox);
    for (int i = 0; i < 20; ++i) {
        layout->addWidget(new QLabel(QStringLiteral("row %1").arg(i), page));
    }
    scrollArea->setWidgetResizable(true);
    scrollArea->setWidget(page);
    auto *root = new QVBoxLayout(dialog);
    root->addWidget(scrollArea);

    dialog->resize(400, 200);
    if (area) {
        *area = scrollArea;
    }
    if (spin) {
        *spin = spinBox;
    }
    return dialog;
}

}  // namespace

class SettingsWheelGuardTest : public QObject {
    Q_OBJECT

private slots:
    /**
     * 验证滚轮防护的基础行为：未聚焦的数值框收到滚轮事件后
     * 值不会被篡改（此前 Qt 默认行为会直接改值）。
     */
    void unfocusedSpinBoxIsNotModifiedByWheel()
    {
        QScrollArea *area = nullptr;
        QSpinBox *spin = nullptr;
        std::unique_ptr<QDialog> dialog(makeDialog(&area, &spin));
        Q_UNUSED(area);

        // 安装滚轮防护。
        markshot::settings::installSettingsWheelGuard(dialog.get());

        dialog->show();
        QVERIFY(QTest::qWaitForWindowExposed(dialog.get()));

        // 确保数值框没有键盘焦点。
        QVERIFY(!spin->hasFocus());
        const int before = spin->value();

        // 向数值框直接派发滚轮事件（角度增量路径）。
        QWheelEvent wheel = makeWheelEvent(spin, 120);
        QApplication::sendEvent(spin, &wheel);

        // 未聚焦的数值框内容不应被滚轮改动。
        QCOMPARE(spin->value(), before);
    }

    /**
     * 验证防护把滚轮换算为页面滚动：未聚焦数值框上的滚轮
     * 会滚动外层滚动区域，且方向与 Qt 默认一致（向上滚动）。
     */
    void wheelScrollsPageInExpectedDirection()
    {
        QScrollArea *area = nullptr;
        QSpinBox *spin = nullptr;
        std::unique_ptr<QDialog> dialog(makeDialog(&area, &spin));

        markshot::settings::installSettingsWheelGuard(dialog.get());

        dialog->show();
        QVERIFY(QTest::qWaitForWindowExposed(dialog.get()));

        QScrollBar *bar = area->verticalScrollBar();
        QVERIFY(bar->maximum() > 0);
        bar->setValue(bar->maximum() / 2);
        const int scrollBefore = bar->value();
        QVERIFY(!spin->hasFocus());

        // 向上滚动一格（角度 +120，Qt 约定内容上移、滚动条值减小）。
        QWheelEvent wheel = makeWheelEvent(spin, 120);
        QApplication::sendEvent(spin, &wheel);

        QVERIFY2(bar->value() < scrollBefore,
                 "wheel-up over a guarded control should scroll the page up");
    }

    /**
     * 验证亚格滚轮增量不会被截断：+60（半格）也应按比例滚动页面，
     * 与 Qt 6.11 的 QAccumulator 行为一致。
     */
    void subNotchDeltaScrollsProportionally()
    {
        QScrollArea *area = nullptr;
        QSpinBox *spin = nullptr;
        std::unique_ptr<QDialog> dialog(makeDialog(&area, &spin));

        markshot::settings::installSettingsWheelGuard(dialog.get());

        // 固定每格行数，避免依赖平台默认值（macOS 为 1、Windows/X11 为 3）。
        const int savedLines = QApplication::wheelScrollLines();
        QApplication::setWheelScrollLines(3);
        dialog->show();
        QVERIFY(QTest::qWaitForWindowExposed(dialog.get()));

        QScrollBar *bar = area->verticalScrollBar();
        QVERIFY(bar->maximum() > 0);
        bar->setValue(bar->maximum() / 2);
        const int before = bar->value();
        QVERIFY(!spin->hasFocus());

        // 半格（+60）应使滚动条移动；连续两次累计为一格。
        QWheelEvent half = makeWheelEvent(spin, 60);
        QApplication::sendEvent(spin, &half);
        const int afterHalf = bar->value();
        QVERIFY2(afterHalf < before,
                 "half-notch wheel should still scroll the page proportionally");

        // 两个半格累计为一格：第二次 +60 应继续向同一方向滚动。
        QWheelEvent half2 = makeWheelEvent(spin, 60);
        QApplication::sendEvent(spin, &half2);
        const int afterFull = bar->value();
        QVERIFY2(afterFull < afterHalf,
                 "accumulated half-notches should keep scrolling");

        // 一个整格（+120）应比两个半格滚动得更远（跨事件小数余量不丢步）。
        QWheelEvent full = makeWheelEvent(spin, 120);
        QApplication::sendEvent(spin, &full);
        const int afterFullNotch = bar->value();
        QVERIFY2(afterFullNotch < afterFull,
                 "full notch should scroll further than accumulated halves");

        QApplication::setWheelScrollLines(savedLines);
    }

    /**
     * 验证像素增量路径：未聚焦数值框上的像素滚轮同样滚动页面，
     * 且数值不被篡改。
     */
    void pixelDeltaScrollsPage()
    {
        QScrollArea *area = nullptr;
        QSpinBox *spin = nullptr;
        std::unique_ptr<QDialog> dialog(makeDialog(&area, &spin));

        markshot::settings::installSettingsWheelGuard(dialog.get());

        dialog->show();
        QVERIFY(QTest::qWaitForWindowExposed(dialog.get()));

        QScrollBar *bar = area->verticalScrollBar();
        QVERIFY(bar->maximum() > 0);
        bar->setValue(bar->maximum() / 2);
        const int scrollBefore = bar->value();
        const int spinBefore = spin->value();
        QVERIFY(!spin->hasFocus());

        QWheelEvent wheel = makeWheelEvent(spin, 0, 40);
        QApplication::sendEvent(spin, &wheel);

        QCOMPARE(spin->value(), spinBefore);
        QVERIFY2(bar->value() < scrollBefore,
                 "pixel wheel over a guarded control should scroll the page up");
    }

    /**
     * 验证 Ctrl/Shift + 滚轮走整页滚动路径（与 Qt 原生 scrollByDelta 一致）：
     * 一格的滚动量约等于 pageStep，而不是单步行数。
     */
    void ctrlWheelScrollsByPageStep()
    {
        QScrollArea *area = nullptr;
        QSpinBox *spin = nullptr;
        std::unique_ptr<QDialog> dialog(makeDialog(&area, &spin));

        markshot::settings::installSettingsWheelGuard(dialog.get());

        dialog->show();
        QVERIFY(QTest::qWaitForWindowExposed(dialog.get()));

        QScrollBar *bar = area->verticalScrollBar();
        QVERIFY(bar->maximum() > 0);
        bar->setValue(bar->maximum() / 2);
        const int before = bar->value();
        QVERIFY(!spin->hasFocus());

        QWheelEvent wheel = makeWheelEvent(spin, -120, 0, Qt::ControlModifier);
        QApplication::sendEvent(spin, &wheel);

        // Ctrl+滚轮向下应滚动约一页。
        const int delta = bar->value() - before;
        QVERIFY2(delta > 0, "Ctrl+wheel down should scroll the page down");
        QVERIFY2(delta <= bar->pageStep() && delta >= bar->pageStep() - bar->singleStep(),
                 qPrintable(QStringLiteral("Ctrl+wheel should scroll ~one page, got delta=%1 pageStep=%2")
                                .arg(delta)
                                .arg(bar->pageStep())));
    }

    /**
     * 验证防护对聚焦控件放行：聚焦的数值框仍可通过滚轮调整数值。
     */
    void focusedSpinBoxKeepsWheelAdjustment()
    {
        QScrollArea *area = nullptr;
        QSpinBox *spin = nullptr;
        std::unique_ptr<QDialog> dialog(makeDialog(&area, &spin));

        markshot::settings::installSettingsWheelGuard(dialog.get());

        dialog->show();
        QVERIFY(QTest::qWaitForWindowExposed(dialog.get()));

        spin->setFocus();
        QVERIFY(spin->hasFocus());
        const int before = spin->value();

        QWheelEvent wheel = makeWheelEvent(spin, 120);
        QApplication::sendEvent(spin, &wheel);

        // 聚焦控件保留滚轮调值能力。
        QVERIFY2(spin->value() != before,
                 "focused spin box should still adjust its value on wheel");
    }

    /**
     * 验证 QWheelEvent::inverted()（macOS 自然滚动标记）不会再次反转滚动方向。
     * Qt 的 QScrollBar::wheelEvent 忽略 inverted()（方向已由平台在增量中体现），
     * 防护若再次取反会导致方向双重翻转，这里回归锁定与 Qt 原生一致的行为：
     * inverted=true 与 inverted=false 的滚轮向上滚动方向相同。
     */
    void invertedFlagDoesNotReverseScrollDirection()
    {
        QScrollArea *area = nullptr;
        QSpinBox *spin = nullptr;
        std::unique_ptr<QDialog> dialog(makeDialog(&area, &spin));

        markshot::settings::installSettingsWheelGuard(dialog.get());

        dialog->show();
        QVERIFY(QTest::qWaitForWindowExposed(dialog.get()));

        QScrollBar *bar = area->verticalScrollBar();
        QVERIFY(bar->maximum() > 0);
        QVERIFY(!spin->hasFocus());

        // 普通（非反向）事件：滚轮向上 +120 应使滚动条值减小。
        bar->setValue(bar->maximum() / 2);
        const int plainBefore = bar->value();
        QWheelEvent plain = makeWheelEvent(spin, 120);
        QApplication::sendEvent(spin, &plain);
        const int plainAfter = bar->value();
        QVERIFY2(plainAfter < plainBefore,
                 "wheel-up should scroll the page up");

        // inverted=true（自然滚动）事件：方向必须与普通事件一致。
        bar->setValue(bar->maximum() / 2);
        const int invertedBefore = bar->value();
        QWheelEvent inverted = makeWheelEvent(spin, 120, 0, Qt::NoModifier, true);
        QApplication::sendEvent(spin, &inverted);
        const int invertedAfter = bar->value();
        QVERIFY2(invertedAfter < invertedBefore,
                 "inverted() must not reverse the scroll direction");
    }
};

QTEST_MAIN(SettingsWheelGuardTest)
#include "settings_wheel_guard_test.moc"
