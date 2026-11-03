#include "pinned_window/pinned_focus_chrome.h"

#include <QtTest/QtTest>

#include <limits>

using markshot::shot::PinnedFocusChromeStyle;
using markshot::shot::pinnedFocusChromeStyle;
using markshot::shot::pinnedShowsFocusChrome;

class PinnedFocusChromeTest : public QObject {
    Q_OBJECT

private slots:
    /**
     * 验证聚焦时边框保持配置颜色和透明度。
     * @return 无返回值。
     */
    void focusedKeepsConfiguredBorderColor()
    {
        const QColor configured(94, 234, 212);
        const PinnedFocusChromeStyle style = pinnedFocusChromeStyle(true, configured, 0.4);

        QCOMPARE(style.borderColor, configured);
        QCOMPARE(style.borderColor.alpha(), 255);
    }

    /**
     * 验证失焦时只降低边框透明度，颜色通道不变。
     * @return 无返回值。
     */
    void inactiveDimsBorderAlphaOnly()
    {
        const QColor configured(94, 234, 212);
        const PinnedFocusChromeStyle style = pinnedFocusChromeStyle(false, configured, 0.4);

        QCOMPARE(style.borderColor.red(), 94);
        QCOMPARE(style.borderColor.green(), 234);
        QCOMPARE(style.borderColor.blue(), 212);
        QCOMPARE(style.borderColor.alpha(), qRound(255 * 0.4));
    }

    /**
     * 验证失焦透明度乘在配置颜色自身的透明度上。
     * @return 无返回值。
     */
    void inactiveMultipliesConfiguredAlpha()
    {
        const QColor configured(10, 20, 30, 128);
        const PinnedFocusChromeStyle style = pinnedFocusChromeStyle(false, configured, 0.5);

        QCOMPARE(style.borderColor.alpha(), 64);
    }

    /**
     * 验证失焦透明度超出范围时截断，非有限值回退到默认值。
     * @return 无返回值。
     */
    void inactiveOpacityIsClamped()
    {
        const QColor configured(255, 0, 0);

        QCOMPARE(pinnedFocusChromeStyle(false, configured, 2.0).borderColor.alpha(), 255);
        QCOMPARE(pinnedFocusChromeStyle(false, configured, -1.0).borderColor.alpha(), 0);
        const double nan = std::numeric_limits<double>::quiet_NaN();
        QCOMPARE(pinnedFocusChromeStyle(false, configured, nan).borderColor.alpha(),
                 qRound(255 * markshot::shot::kPinnedInactiveBorderOpacityDefault));
    }

    /**
     * 验证默认失焦透明度让边框变淡但仍可见。
     * @return 无返回值。
     */
    void defaultInactiveOpacityStaysVisible()
    {
        const PinnedFocusChromeStyle style =
            pinnedFocusChromeStyle(false, QColor(94, 234, 212), markshot::shot::kPinnedInactiveBorderOpacityDefault);

        QVERIFY(style.borderColor.alpha() > 0);
        QVERIFY(style.borderColor.alpha() < 255);
    }

    /**
     * 验证无效的配置颜色在两种状态下都保持无效。
     * @return 无返回值。
     */
    void invalidBorderColorStaysInvalid()
    {
        QVERIFY(!pinnedFocusChromeStyle(true, QColor(), 0.4).borderColor.isValid());
        QVERIFY(!pinnedFocusChromeStyle(false, QColor(), 0.4).borderColor.isValid());
    }

    /**
     * 验证聚焦阴影比失焦阴影更深更宽，失焦时阴影仍然存在。
     * @return 无返回值。
     */
    void focusedShadowIsDeeperThanInactive()
    {
        const PinnedFocusChromeStyle focused = pinnedFocusChromeStyle(true, QColor(Qt::black), 0.4);
        const PinnedFocusChromeStyle inactive = pinnedFocusChromeStyle(false, QColor(Qt::black), 0.4);

        QVERIFY(inactive.shadowLayers > 0);
        QVERIFY(inactive.shadowLayerAlpha > 0);
        QVERIFY(focused.shadowLayers > inactive.shadowLayers);
        QVERIFY(focused.shadowLayerAlpha > inactive.shadowLayerAlpha);
        QVERIFY(focused.shadowOffsetY > inactive.shadowOffsetY);
    }

    /**
     * 验证阴影参数与边框配置无关。
     * @return 无返回值。
     */
    void shadowDoesNotDependOnBorderConfig()
    {
        const PinnedFocusChromeStyle a = pinnedFocusChromeStyle(false, QColor(Qt::red), 0.0);
        const PinnedFocusChromeStyle b = pinnedFocusChromeStyle(false, QColor(), 1.0);

        QCOMPARE(a.shadowLayers, b.shadowLayers);
        QCOMPARE(a.shadowLayerAlpha, b.shadowLayerAlpha);
        QCOMPARE(a.shadowOffsetY, b.shadowOffsetY);
    }

    /**
     * 验证普通窗口只按活动状态判断，layer-shell 贴图悬停也视为聚焦。
     * @return 无返回值。
     */
    void hoverCountsOnlyWhenAllowed()
    {
        QVERIFY(pinnedShowsFocusChrome(true, false, false));
        QVERIFY(!pinnedShowsFocusChrome(false, true, false));
        QVERIFY(!pinnedShowsFocusChrome(false, false, false));
        QVERIFY(pinnedShowsFocusChrome(false, true, true));
        QVERIFY(pinnedShowsFocusChrome(true, false, true));
        QVERIFY(!pinnedShowsFocusChrome(false, false, true));
    }
};

QTEST_APPLESS_MAIN(PinnedFocusChromeTest)

#include "pinned_focus_chrome_test.moc"
