#include "shortcuts/global_shortcut_keymap.h"

#include <QtTest/QtTest>

using markshot::shortcuts::X11KeyBinding;
using markshot::shortcuts::x11BindingForSequence;
using markshot::shortcuts::x11KeysymForQtKey;

class GlobalShortcutKeymapTest : public QObject {
    Q_OBJECT

private slots:
    void lettersMapToLowercaseKeysyms()
    {
        // X11 抓取的是未叠加 Shift 的 keysym，字母应落在 ASCII 小写区间
        QCOMPARE(x11KeysymForQtKey(Qt::Key_A), 0x0061u);
        QCOMPARE(x11KeysymForQtKey(Qt::Key_S), 0x0073u);
        QCOMPARE(x11KeysymForQtKey(Qt::Key_Z), 0x007au);
    }

    void digitsMapToAsciiKeysyms()
    {
        QCOMPARE(x11KeysymForQtKey(Qt::Key_0), 0x0030u);
        QCOMPARE(x11KeysymForQtKey(Qt::Key_9), 0x0039u);
    }

    void functionKeysAreContiguous()
    {
        QCOMPARE(x11KeysymForQtKey(Qt::Key_F1), 0xffbeu);
        QCOMPARE(x11KeysymForQtKey(Qt::Key_F5), 0xffbeu + 4);
        QCOMPARE(x11KeysymForQtKey(Qt::Key_F12), 0xffbeu + 11);
    }

    void namedKeysMapToKnownKeysyms()
    {
        QCOMPARE(x11KeysymForQtKey(Qt::Key_Print), 0xff61u);
        QCOMPARE(x11KeysymForQtKey(Qt::Key_Escape), 0xff1bu);
        QCOMPARE(x11KeysymForQtKey(Qt::Key_Space), 0x0020u);
        QCOMPARE(x11KeysymForQtKey(Qt::Key_Comma), 0x002cu);
    }

    void unsupportedKeyReturnsZero()
    {
        QCOMPARE(x11KeysymForQtKey(Qt::Key_Massyo), 0u);
        QCOMPARE(x11KeysymForQtKey(Qt::Key_unknown), 0u);
    }

    void sequenceCombinesModifierMasks()
    {
        // Ctrl+Alt+S：ControlMask(1<<2) | Mod1Mask(1<<3) = 0x0c
        const X11KeyBinding binding = x11BindingForSequence(QKeySequence(QStringLiteral("Ctrl+Alt+S")));
        QVERIFY(binding.valid);
        QCOMPARE(binding.keysym, 0x0073u);
        QCOMPARE(binding.modifiers, static_cast<std::uint16_t>((1 << 2) | (1 << 3)));
    }

    void metaModifierMapsToMod4()
    {
        const X11KeyBinding binding = x11BindingForSequence(QKeySequence(QStringLiteral("Meta+Shift+P")));
        QVERIFY(binding.valid);
        QCOMPARE(binding.keysym, 0x0070u);
        QCOMPARE(binding.modifiers, static_cast<std::uint16_t>((1 << 0) | (1 << 6)));
    }

    void sequenceWithoutModifierIsRejected()
    {
        // 无修饰键的组合会抢占普通输入，必须拒绝
        QVERIFY(!x11BindingForSequence(QKeySequence(QStringLiteral("S"))).valid);
        QVERIFY(!x11BindingForSequence(QKeySequence(QStringLiteral("F5"))).valid);
    }

    void emptySequenceIsRejected()
    {
        QVERIFY(!x11BindingForSequence(QKeySequence()).valid);
    }

    void unmappableKeyIsRejected()
    {
        QVERIFY(!x11BindingForSequence(QKeySequence(QStringLiteral("Ctrl+Massyo"))).valid);
    }
};

QTEST_MAIN(GlobalShortcutKeymapTest)
#include "global_shortcut_keymap_test.moc"
