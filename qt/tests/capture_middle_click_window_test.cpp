#include "app_config_store.h"
#include "settings/settings_config.h"
#include "settings/settings_page_capture.h"
#include "shot_window.h"
#include "support/isolated_config.h"
#include "ui/i18n.h"

#include <QApplication>
#include <QComboBox>
#include <QMouseEvent>
#include <QPushButton>
#include <QSignalSpy>
#include <QTemporaryDir>
#include <QWheelEvent>
#include <QtTest>

#include <memory>

namespace {

// 窗口与冻结帧同为 1000x700，图像坐标与窗口坐标一致
const QSize kWindowSize(1000, 700);
// 拖出的选区，工具栏跟随选区落在其下方
const QPoint kSelectionStart(300, 200);
const QPoint kSelectionEnd(700, 450);
// 选区内的采样点，滚轮缩放提示画在 kZoomAnchor 附近，不覆盖该点
const QPoint kInside(500, 300);
const QPoint kZoomAnchor(620, 400);
// 冻结图像内、选区外的点
const QPoint kOutside(150, 100);

/// @brief 发送携带完整按键状态的鼠标事件，无返回值
/// @param window 截图窗口
/// @param type 鼠标事件类型
/// @param position 窗口内位置
/// @param button 本次变化的按键
/// @param buttons 事件发生后仍按住的按键
void pointerEvent(ShotWindow &window, QEvent::Type type, QPoint position,
                  Qt::MouseButton button = Qt::NoButton, Qt::MouseButtons buttons = Qt::NoButton)
{
    QMouseEvent event(type, QPointF(position), QPointF(window.mapToGlobal(position)), button, buttons,
                      Qt::NoModifier);
    QCoreApplication::sendEvent(&window, &event);
    QCoreApplication::processEvents();
}

/// @brief 在指定位置原地按下并释放中键，无返回值
/// @param window 截图窗口
/// @param position 窗口内位置
void middleClick(ShotWindow &window, QPoint position)
{
    pointerEvent(window, QEvent::MouseButtonPress, position, Qt::MiddleButton, Qt::MiddleButton);
    pointerEvent(window, QEvent::MouseButtonRelease, position, Qt::MiddleButton);
}

/// @brief 读取窗口当前画面在指定位置的颜色
/// @param window 截图窗口
/// @param position 窗口内位置
/// @return 该位置的像素颜色
QColor shownColor(ShotWindow &window, QPoint position)
{
    const QImage image = window.grab().toImage();
    return image.pixelColor(QPointF(position * image.devicePixelRatio()).toPoint());
}

/// @brief 查找真实工具栏按钮
/// @param window 截图窗口
/// @param action 按钮的动作名称
/// @return 对应按钮，不存在时返回空指针
QPushButton *toolButton(ShotWindow &window, const char *action)
{
    for (QPushButton *button : window.findChildren<QPushButton *>()) {
        if (button->property("action").toString() == QLatin1String(action)) {
            return button;
        }
    }
    return nullptr;
}

}  // namespace

/// 通过真实截图窗口验证中键单击动作（issue #141）：未拖动的中键单击执行配置动作，拖动照常平移画布
class CaptureMiddleClickWindowTest final : public QObject {
    Q_OBJECT

private:
    /**
     * 写入中键单击配置并创建尚未框选的截图窗口。
     * @param action capture.middleClickAction 的配置值。
     * @return 截图窗口；窗口未能显示时返回空。
     */
    static std::unique_ptr<ShotWindow> createWindow(const QString &action)
    {
        // 1. 窗口在构造时读取一次配置，必须先写入
        if (!markshot::writeAppConfigValue({QStringLiteral("capture"), QStringLiteral("middleClickAction")},
                                           action)) {
            return nullptr;
        }

        // 2. 横向渐变底图，画布平移后同一窗口位置的颜色随之变化
        QImage image(kWindowSize, QImage::Format_ARGB32_Premultiplied);
        for (int x = 0; x < image.width(); ++x) {
            const QColor column(x % 256, (x / 4) % 256, 160);
            for (int y = 0; y < image.height(); ++y) {
                image.setPixelColor(x, y, column);
            }
        }
        auto window = std::make_unique<ShotWindow>(image, QString(), QRect(QPoint(0, 0), kWindowSize),
                                                   QVector<markshot::WindowInfo>{}, false);
        window->setAttribute(Qt::WA_DeleteOnClose, false);
        window->resize(kWindowSize);
        window->show();
        if (!QTest::qWaitForWindowExposed(window.get())) {
            return nullptr;
        }
        return window;
    }

    /**
     * 创建窗口并用真实左键拖动完成框选，进入编辑阶段。
     * @param action capture.middleClickAction 的配置值。
     * @return 截图窗口；窗口未能显示时返回空。
     */
    static std::unique_ptr<ShotWindow> createSelectedWindow(const QString &action)
    {
        auto window = createWindow(action);
        if (!window) {
            return nullptr;
        }
        pointerEvent(*window, QEvent::MouseButtonPress, kSelectionStart, Qt::LeftButton, Qt::LeftButton);
        pointerEvent(*window, QEvent::MouseMove, kSelectionEnd, Qt::NoButton, Qt::LeftButton);
        pointerEvent(*window, QEvent::MouseButtonRelease, kSelectionEnd, Qt::LeftButton);
        return window;
    }

    /**
     * 切换到选择工具并在选区内滚动一格放大画布，使中键平移产生可见位移。
     * @param window 截图窗口。
     * @return 找到选择工具按钮时返回 true。
     */
    static bool zoomIn(ShotWindow &window)
    {
        QPushButton *select = toolButton(window, "Select");
        if (!select) {
            return false;
        }
        select->click();
        QCoreApplication::processEvents();
        QWheelEvent event(QPointF(kZoomAnchor), QPointF(window.mapToGlobal(kZoomAnchor)), {}, QPoint(0, 120),
                          Qt::NoButton, Qt::NoModifier, Qt::NoScrollPhase, false);
        QCoreApplication::sendEvent(&window, &event);
        QCoreApplication::processEvents();
        return true;
    }

private slots:
    /** 关闭区域识别，避免渐变底图生成候选区域干扰框选，无参数和返回值。 */
    void initTestCase()
    {
        QVERIFY(markshot::writeAppConfigValue(
            {QStringLiteral("capture"), QStringLiteral("regionDetection"), QStringLiteral("enabled")}, false));
    }

    /** 验证默认配置下中键单击不执行动作，中键拖动仍然平移画布，无参数和返回值。 */
    void defaultKeepsPanAndIgnoresClick()
    {
        auto window = createSelectedWindow(QStringLiteral("default"));
        QVERIFY(window);
        QSignalSpy cancelled(window.get(), &ShotWindow::sessionCancelRequested);
        middleClick(*window, kInside);
        QCOMPARE(cancelled.count(), 0);
        QVERIFY(window->isVisible());

        QVERIFY(zoomIn(*window));
        const QColor before = shownColor(*window, kInside);
        pointerEvent(*window, QEvent::MouseButtonPress, kInside, Qt::MiddleButton, Qt::MiddleButton);
        QCOMPARE(window->cursor().shape(), Qt::ClosedHandCursor);
        pointerEvent(*window, QEvent::MouseMove, kInside - QPoint(40, 0), Qt::NoButton, Qt::MiddleButton);
        pointerEvent(*window, QEvent::MouseButtonRelease, kInside - QPoint(40, 0), Qt::MiddleButton);
        QVERIFY(shownColor(*window, kInside) != before);
        QCOMPARE(cancelled.count(), 0);
    }

    /** 验证选区阶段中键仍切换到全屏标注而不执行配置动作，无参数和返回值。 */
    void selectingPhaseStillEntersFullscreenAnnotation()
    {
        auto window = createWindow(QStringLiteral("cancel"));
        QVERIFY(window);
        QPushButton *pen = toolButton(*window, "Pen");
        QVERIFY(pen && !pen->isVisible());
        QSignalSpy cancelled(window.get(), &ShotWindow::sessionCancelRequested);
        middleClick(*window, kInside);
        QVERIFY(pen->isVisible());
        QCOMPARE(cancelled.count(), 0);
        QVERIFY(window->isVisible());
    }

    /** 验证框选完成后在选区内中键单击执行配置动作，无参数和返回值。 */
    void clickInsideSelectionRunsAction()
    {
        auto window = createSelectedWindow(QStringLiteral("cancel"));
        QVERIFY(window);
        QVERIFY(!window->childAt(kInside));
        QSignalSpy cancelled(window.get(), &ShotWindow::sessionCancelRequested);
        const int jitter = std::max(1, QApplication::startDragDistance() / 2);
        pointerEvent(*window, QEvent::MouseButtonPress, kInside, Qt::MiddleButton, Qt::MiddleButton);
        pointerEvent(*window, QEvent::MouseMove, kInside + QPoint(jitter - 1, 0), Qt::NoButton, Qt::MiddleButton);
        pointerEvent(*window, QEvent::MouseButtonRelease, kInside + QPoint(jitter - 1, 0), Qt::MiddleButton);
        QCOMPARE(cancelled.count(), 1);
        QVERIFY(!window->isVisible());
    }

    /** 验证配置动作后中键拖动仍平移画布且不执行动作，阈值内抖动不移动画面，无参数和返回值。 */
    void dragInsideSelectionStillPans()
    {
        auto window = createSelectedWindow(QStringLiteral("cancel"));
        QVERIFY(window);
        QVERIFY(zoomIn(*window));
        QSignalSpy cancelled(window.get(), &ShotWindow::sessionCancelRequested);
        const QColor before = shownColor(*window, kInside);
        pointerEvent(*window, QEvent::MouseButtonPress, kInside, Qt::MiddleButton, Qt::MiddleButton);
        QCOMPARE(window->cursor().shape(), Qt::ClosedHandCursor);
        pointerEvent(*window, QEvent::MouseMove, kInside + QPoint(1, 0), Qt::NoButton, Qt::MiddleButton);
        QCOMPARE(shownColor(*window, kInside), before);
        const QPoint end = kInside - QPoint(QApplication::startDragDistance() + 40, 0);
        pointerEvent(*window, QEvent::MouseMove, end, Qt::NoButton, Qt::MiddleButton);
        pointerEvent(*window, QEvent::MouseButtonRelease, end, Qt::MiddleButton);
        QVERIFY(shownColor(*window, kInside) != before);
        QCOMPARE(cancelled.count(), 0);
        QVERIFY(window->isVisible());
    }

    /** 验证选区外的中键单击不执行动作，无参数和返回值。 */
    void clickOutsideSelectionIsIgnored()
    {
        auto window = createSelectedWindow(QStringLiteral("cancel"));
        QVERIFY(window);
        QSignalSpy cancelled(window.get(), &ShotWindow::sessionCancelRequested);
        middleClick(*window, kOutside);
        QCOMPARE(cancelled.count(), 0);
        QVERIFY(window->isVisible());
    }

    /** 验证与左键组合或释放丢失的中键不执行动作，之后的中键单击照常生效，无参数和返回值。 */
    void interruptedClickIsIgnored()
    {
        auto window = createSelectedWindow(QStringLiteral("cancel"));
        QVERIFY(window);
        QSignalSpy cancelled(window.get(), &ShotWindow::sessionCancelRequested);

        // 1. 平移期间按下左键，本次中键不再算作单击
        pointerEvent(*window, QEvent::MouseButtonPress, kInside, Qt::MiddleButton, Qt::MiddleButton);
        pointerEvent(*window, QEvent::MouseButtonPress, kInside, Qt::LeftButton, Qt::LeftButton | Qt::MiddleButton);
        pointerEvent(*window, QEvent::MouseButtonRelease, kInside, Qt::LeftButton, Qt::MiddleButton);
        pointerEvent(*window, QEvent::MouseButtonRelease, kInside, Qt::MiddleButton);
        QCOMPARE(cancelled.count(), 0);

        // 2. 中键释放事件丢失后，无按键移动结束平移，迟到的释放不补发动作
        pointerEvent(*window, QEvent::MouseButtonPress, kInside, Qt::MiddleButton, Qt::MiddleButton);
        pointerEvent(*window, QEvent::MouseMove, kInside);
        pointerEvent(*window, QEvent::MouseButtonRelease, kInside, Qt::MiddleButton);
        QCOMPARE(cancelled.count(), 0);
        QVERIFY(window->isVisible());

        // 3. 之后完整的中键单击仍执行动作
        middleClick(*window, kInside);
        QCOMPARE(cancelled.count(), 1);
    }

    /** 验证截图设置页的中键单击下拉框读写 capture.middleClickAction 并提供中文文案，无参数和返回值。 */
    void settingsPageRoundTrip()
    {
        using namespace markshot::settings;
        QVERIFY(markshot::writeAppConfigValue({QStringLiteral("capture"), QStringLiteral("middleClickAction")},
                                              QStringLiteral("default")));
        SettingsConfig settings = readSettingsConfig();
        QCOMPARE(settings.capture.middleClickAction, markshot::CaptureClickAction::None);
        SettingsPageCapture page;
        page.setConfig(settings);
        auto *combo = page.findChild<QComboBox *>(QStringLiteral("captureMiddleClickAction"));
        QVERIFY(combo);
        QCOMPARE(combo->count(), 6);
        QCOMPARE(combo->currentData().toInt(), static_cast<int>(markshot::CaptureClickAction::None));

        // 1. 选择钉到屏幕后保存，配置文件写入规范名称 pin
        combo->setCurrentIndex(combo->findData(static_cast<int>(markshot::CaptureClickAction::Pin)));
        page.updateConfig(&settings);
        QString error;
        QVERIFY2(writeSettingsConfig(settings, &error), qPrintable(error));
        QCOMPARE(readSettingsConfig().capture.middleClickAction, markshot::CaptureClickAction::Pin);
        bool ok = false;
        const QJsonObject root = markshot::readAppConfigRoot(&ok);
        QVERIFY(ok);
        QCOMPARE(root.value(QStringLiteral("capture")).toObject().value(QStringLiteral("middleClickAction")),
                 QJsonValue(QStringLiteral("pin")));

        // 2. 恢复默认后写回 default，双击配置不受影响
        combo->setCurrentIndex(0);
        page.updateConfig(&settings);
        QVERIFY2(writeSettingsConfig(settings, &error), qPrintable(error));
        settings = readSettingsConfig();
        QCOMPARE(settings.capture.middleClickAction, markshot::CaptureClickAction::None);
        QCOMPARE(settings.capture.doubleClickAction, markshot::CaptureClickAction::Copy);

        // 3. 中文界面文案
        markshot::i18n::setLanguage(markshot::i18n::Language::Chinese);
        QCOMPARE(markshot::i18n::translate(QStringLiteral("Middle Click Action")), QStringLiteral("中键单击动作"));
        QCOMPARE(markshot::i18n::translate(QStringLiteral("Default (Pan Canvas)")), QStringLiteral("默认（平移画布）"));
        markshot::i18n::setLanguage(markshot::i18n::Language::English);
    }
};

/**
 * 使用隔离配置运行中键单击窗口回归。
 * @param argc 参数数量。
 * @param argv 参数列表。
 * @return 测试退出码。
 */
int main(int argc, char **argv)
{
    QTemporaryDir home;
    if (!markshot::test::isolateConfig(home)) return 1;
    QApplication app(argc, argv);
    CaptureMiddleClickWindowTest test;
    return QTest::qExec(&test, argc, argv);
}

#include "capture_middle_click_window_test.moc"
