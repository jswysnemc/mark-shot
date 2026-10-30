#include "shot_window.h"
#include "startup_ruler/ruler_painter.h"
#include "support/isolated_config.h"

#include <QApplication>
#include <QFontMetricsF>
#include <QKeyEvent>
#include <QMouseEvent>
#include <QTemporaryDir>
#include <QtTest>

#include <memory>

namespace {

/**
 * 向截图窗口发送一次鼠标事件。
 * @param window 截图窗口。
 * @param type 事件类型。
 * @param position 窗口坐标。
 * @param buttons 事件发生后仍按住的按键。
 * @return 无返回值。
 */
void sendMouse(ShotWindow &window, QEvent::Type type, QPoint position, Qt::MouseButtons buttons)
{
    const Qt::MouseButton button = type == QEvent::MouseMove ? Qt::NoButton : Qt::LeftButton;
    QMouseEvent event(type, QPointF(position), QPointF(window.mapToGlobal(position)), button, buttons,
                      Qt::NoModifier);
    QCoreApplication::sendEvent(&window, &event);
}

/**
 * 用左键从起点拖到终点，中途发送一次移动事件。
 * @param window 截图窗口。
 * @param from 按下位置，窗口坐标。
 * @param to 松开位置，窗口坐标。
 * @return 无返回值。
 */
void dragRuler(ShotWindow &window, QPoint from, QPoint to)
{
    sendMouse(window, QEvent::MouseButtonPress, from, Qt::LeftButton);
    sendMouse(window, QEvent::MouseMove, (from + to) / 2, Qt::LeftButton);
    sendMouse(window, QEvent::MouseMove, to, Qt::LeftButton);
    sendMouse(window, QEvent::MouseButtonRelease, to, Qt::NoButton);
}

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
}

}  // namespace

/// @brief 检查启动阶段尺子工具可以连续拖出多把尺子
class StartupRulerTest final : public QObject {
    Q_OBJECT

public:
    /**
     * 创建处于选区阶段并打开尺子工具的截图窗口，冻结帧与窗口同为 800x600。
     * @return 截图窗口，窗口未能显示时返回空指针。
     */
    static std::unique_ptr<ShotWindow> createRulerWindow()
    {
        QImage image(800, 600, QImage::Format_ARGB32_Premultiplied);
        image.fill(QColor(255, 255, 255));
        auto window = std::make_unique<ShotWindow>(image, QString(), QRect(0, 0, 800, 600),
                                                   QVector<markshot::WindowInfo>{}, false);
        window->setAttribute(Qt::WA_DeleteOnClose, false);
        window->resize(800, 600);
        window->show();
        if (!QTest::qWaitForWindowExposed(window.get())) {
            return nullptr;
        }
        window->setStartupTool(markshot::shot::types::StartupTool::Ruler);
        return window;
    }

    /**
     * 读取窗口中各把尺子的测量范围。
     * @param window 截图窗口。
     * @return 图像坐标下的测量范围，按拖出先后排列。
     */
    static QVector<QRectF> rulerRects(const ShotWindow &window)
    {
        QVector<QRectF> rects;
        for (const QLineF &line : window.m_startupRulers) {
            rects.append(QRectF(line.p1(), line.p2()).normalized());
        }
        return rects;
    }

private slots:
    /** 验证再拖一把尺子时前一把仍然保留并继续绘制，无参数和返回值。 */
    void keepsEveryDraggedRuler()
    {
        auto window = createRulerWindow();
        QVERIFY(window);
        const QColor blank = window->grab().toImage().pixelColor(150, 140);

        dragRuler(*window, QPoint(100, 100), QPoint(200, 180));
        dragRuler(*window, QPoint(400, 300), QPoint(300, 250));
        QCOMPARE(rulerRects(*window),
                 (QVector<QRectF>{QRectF(100, 100, 100, 80), QRectF(300, 250, 100, 50)}));
        QVERIFY(!window->m_startupRulerDragging);

        // 1. 第一把尺子的测量框内部带有填充色，第二把拖出后仍能看到；取样点避开十字线和悬停标签
        const QImage frame = window->grab().toImage();
        const qreal dpr = frame.devicePixelRatio();
        QVERIFY(frame.pixelColor((QPointF(150, 140) * dpr).toPoint()) != blank);
        QVERIFY(frame.pixelColor((QPointF(390, 256) * dpr).toPoint()) != blank);
    }

    /** 验证只单击不拖动不会新增尺子，已量好的尺子不受影响，无参数和返回值。 */
    void clickWithoutDragAddsNoRuler()
    {
        auto window = createRulerWindow();
        QVERIFY(window);
        dragRuler(*window, QPoint(100, 100), QPoint(200, 180));
        sendMouse(*window, QEvent::MouseButtonPress, QPoint(500, 400), Qt::LeftButton);
        sendMouse(*window, QEvent::MouseButtonRelease, QPoint(500, 400), Qt::NoButton);
        QCOMPARE(rulerRects(*window), (QVector<QRectF>{QRectF(100, 100, 100, 80)}));
    }

    /** 验证 Backspace 和 Delete 逐把撤掉最后拖出的尺子，工具保持打开，无参数和返回值。 */
    void deleteKeysRemoveLastRuler()
    {
        auto window = createRulerWindow();
        QVERIFY(window);
        dragRuler(*window, QPoint(100, 100), QPoint(200, 180));
        dragRuler(*window, QPoint(300, 250), QPoint(400, 300));
        dragRuler(*window, QPoint(500, 100), QPoint(600, 150));

        pressKey(*window, Qt::Key_Backspace);
        QCOMPARE(rulerRects(*window),
                 (QVector<QRectF>{QRectF(100, 100, 100, 80), QRectF(300, 250, 100, 50)}));
        pressKey(*window, Qt::Key_Delete);
        QCOMPARE(rulerRects(*window), (QVector<QRectF>{QRectF(100, 100, 100, 80)}));
        pressKey(*window, Qt::Key_Backspace);
        pressKey(*window, Qt::Key_Backspace);
        QVERIFY(window->m_startupRulers.isEmpty());
        QCOMPARE(window->startupTool(), markshot::shot::types::StartupTool::Ruler);
    }

    /** 验证 Esc 退出尺子工具时清空全部尺子，再次打开从空白开始，无参数和返回值。 */
    void escapeClearsAllRulers()
    {
        auto window = createRulerWindow();
        QVERIFY(window);
        dragRuler(*window, QPoint(100, 100), QPoint(200, 180));
        dragRuler(*window, QPoint(300, 250), QPoint(400, 300));

        pressKey(*window, Qt::Key_Escape);
        QCOMPARE(window->startupTool(), markshot::shot::types::StartupTool::None);
        QVERIFY(window->m_startupRulers.isEmpty());
        window->setStartupTool(markshot::shot::types::StartupTool::Ruler);
        QVERIFY(window->m_startupRulers.isEmpty());
    }

    /** 验证后一把尺子的尺寸说明避开前一把的说明，两段说明不重叠，无参数和返回值。 */
    void infoLabelsAvoidEachOther()
    {
        // 1. 第二把尺子在第一把右侧，说明默认放在下方时会压住第一段说明，应改放到上方
        const QSizeF windowSize(800, 600);
        const QRectF firstRuler(100, 100, 100, 80);
        const QRectF secondRuler(300, 120, 100, 60);
        const QVector<QRectF> labels = markshot::startup_ruler::layoutRulerInfoLabels(
            {{firstRuler, firstRuler}, {secondRuler, secondRuler}}, QFontMetricsF(QApplication::font()), windowSize, {});
        QCOMPARE(labels.size(), 2);
        QVERIFY(labels.at(0).top() > firstRuler.bottom());
        QVERIFY(!labels.at(0).intersects(labels.at(1)));
        QVERIFY(labels.at(1).bottom() < secondRuler.top());
        QVERIFY(QRectF(QPointF(0, 0), windowSize).contains(labels.at(1)));
    }

    /** 验证三把错落的尺子排版后，每段说明既不压住其他尺子的刻度，也不互相重叠，无参数和返回值。 */
    void infoLabelsAvoidOtherRulerScales()
    {
        // 1. 第二把尺子的说明默认放在下方，会压住第三把尺子右上角的刻度
        QVector<markshot::startup_ruler::RulerGeometry> rulers;
        for (const QRectF &rect : {QRectF(100, 120, 200, 120), QRectF(360, 300, 200, 120), QRectF(140, 460, 280, 80)}) {
            rulers.append({rect, rect});
        }
        const QVector<QRectF> labels = markshot::startup_ruler::layoutRulerInfoLabels(
            rulers, QFontMetricsF(QApplication::font()), QSizeF(1200, 800), {});
        QCOMPARE(labels.size(), rulers.size());

        // 2. 逐段检查说明与其他尺子的刻度、与其他说明都不相交
        for (int i = 0; i < labels.size(); ++i) {
            for (int j = 0; j < rulers.size(); ++j) {
                const QRectF scale = markshot::startup_ruler::rulerScaleBounds(rulers.at(j).widgetRect);
                QVERIFY2(!labels.at(i).intersects(scale), qPrintable(QStringLiteral("label %1 ruler %2").arg(i).arg(j)));
                if (j != i) {
                    QVERIFY2(!labels.at(i).intersects(labels.at(j)), qPrintable(QStringLiteral("label %1 label %2").arg(i).arg(j)));
                }
            }
        }
    }
};

/**
 * 使用隔离配置运行尺子工具窗口回归。
 * @param argc 参数数量。
 * @param argv 参数列表。
 * @return 测试退出码。
 */
int main(int argc, char **argv)
{
    QTemporaryDir home;
    if (!markshot::test::isolateConfig(home)) return 1;
    QApplication app(argc, argv);
    StartupRulerTest test;
    return QTest::qExec(&test, argc, argv);
}

#include "startup_ruler_test.moc"
