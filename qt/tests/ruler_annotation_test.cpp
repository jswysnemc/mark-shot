#include "shot_window.h"
#include "shortcut_config.h"
#include "ruler/angle_painter.h"
#include "ruler/ruler_painter.h"
#include "support/isolated_config.h"

#include <QApplication>
#include <QKeyEvent>
#include <QMouseEvent>
#include <QTemporaryDir>
#include <QtTest>

#include <memory>

namespace {

using markshot::shot::types::Mode;
using markshot::shot::types::RulerMode;
using markshot::shot::types::StartupTool;
using markshot::shot::types::Tool;

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
void drag(ShotWindow &window, QPoint from, QPoint to)
{
    sendMouse(window, QEvent::MouseButtonPress, from, Qt::LeftButton);
    sendMouse(window, QEvent::MouseMove, (from + to) / 2, Qt::LeftButton);
    sendMouse(window, QEvent::MouseMove, to, Qt::LeftButton);
    sendMouse(window, QEvent::MouseButtonRelease, to, Qt::NoButton);
}

/**
 * 在同一位置按下并松开左键。
 * @param window 截图窗口。
 * @param position 窗口坐标。
 * @return 无返回值。
 */
void click(ShotWindow &window, QPoint position)
{
    sendMouse(window, QEvent::MouseButtonPress, position, Qt::LeftButton);
    sendMouse(window, QEvent::MouseButtonRelease, position, Qt::NoButton);
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

/// @brief 检查尺子作为标注工具的绘制、调整、角度定点、导出和选区阶段入口
class RulerAnnotationTest final : public QObject {
    Q_OBJECT

public:
    /**
     * 创建处于选区阶段的截图窗口并按 R 进入尺子工具，冻结帧与窗口同为 800x600。
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
        pressKey(*window, Qt::Key_R);
        return window;
    }

    /**
     * 读取全部尺子标注。
     * @param window 截图窗口。
     * @return 尺子标注，按绘制先后排列。
     */
    static QVector<ShotWindow::Annotation> rulers(const ShotWindow &window)
    {
        QVector<ShotWindow::Annotation> result;
        for (const ShotWindow::Annotation &annotation : window.m_annotations) {
            if (annotation.tool == Tool::Ruler) {
                result.append(annotation);
            }
        }
        return result;
    }

private slots:
    /** 验证选区阶段按 R 进入整屏标注并选中尺子工具，无参数和返回值。 */
    void rEntersFullscreenRulerTool()
    {
        auto window = createRulerWindow();
        QVERIFY(window);
        QCOMPARE(window->m_mode, Mode::Editing);
        QVERIFY(window->m_fullscreenAnnotation);
        QCOMPARE(window->m_tool, Tool::Ruler);
        QCOMPARE(window->startupTool(), StartupTool::None);
        QVERIFY(window->m_ruler.enteredFromSelection);
    }

    /** 验证拖出的尺子成为标注，尺子内部可以再量一把，单击不留下尺子，并可撤销，无参数和返回值。 */
    void dragCreatesLengthRulers()
    {
        auto window = createRulerWindow();
        QVERIFY(window);
        drag(*window, QPoint(100, 100), QPoint(300, 260));
        drag(*window, QPoint(150, 150), QPoint(200, 180));
        click(*window, QPoint(500, 400));
        const auto drawn = rulers(*window);
        QCOMPARE(drawn.size(), 2);
        QCOMPARE(drawn.at(0).rect, QRectF(100, 100, 200, 160));
        QCOMPARE(drawn.at(1).rect, QRectF(150, 150, 50, 30));
        QCOMPARE(drawn.at(0).rulerMode, RulerMode::Length);

        window->undoAnnotationEdit();
        QCOMPARE(rulers(*window).size(), 1);
    }

    /** 验证平直的尺子也能量出长度并留下标注，无参数和返回值。 */
    void flatRulerMeasuresLength()
    {
        auto window = createRulerWindow();
        QVERIFY(window);
        drag(*window, QPoint(100, 300), QPoint(400, 300));
        const auto drawn = rulers(*window);
        QCOMPARE(drawn.size(), 1);
        QCOMPARE(drawn.first().rect, QRectF(100, 300, 300, 0));
    }

    /** 验证尺子工具下拖角改大小、拖边框整体移动，Delete 删除选中的尺子，无参数和返回值。 */
    void handlesResizeMoveAndDelete()
    {
        auto window = createRulerWindow();
        QVERIFY(window);
        drag(*window, QPoint(100, 100), QPoint(200, 180));

        // 1. 右下角向外拖，左上角保持不动
        drag(*window, QPoint(200, 180), QPoint(260, 220));
        QCOMPARE(rulers(*window).first().rect, QRectF(100, 100, 160, 120));
        QCOMPARE(window->selectedAnnotationIds().size(), 1);

        // 2. 上边框离控制点较远的位置整体移动
        drag(*window, QPoint(130, 100), QPoint(150, 130));
        QCOMPARE(rulers(*window).first().rect, QRectF(120, 130, 160, 120));
        QCOMPARE(window->m_tool, Tool::Ruler);

        pressKey(*window, Qt::Key_Delete);
        QVERIFY(rulers(*window).isEmpty());
    }

    /** 验证选择工具下尺子与其他标注一样可点中内部拖动，无参数和返回值。 */
    void selectToolMovesRuler()
    {
        auto window = createRulerWindow();
        QVERIFY(window);
        drag(*window, QPoint(100, 100), QPoint(300, 260));
        window->setTool(Tool::Select);
        drag(*window, QPoint(200, 180), QPoint(220, 190));
        QCOMPARE(rulers(*window).first().rect, QRectF(120, 110, 200, 160));
    }

    /** 验证角度尺三次定点，松开后的抓取清理不会撤去等待定点的草稿，无参数和返回值。 */
    void angleRulerTakesThreePoints()
    {
        auto window = createRulerWindow();
        QVERIFY(window);
        pressKey(*window, Qt::Key_U);
        QCOMPARE(window->m_ruler.mode, RulerMode::Angle);

        // 1. 按下放顶点，拖出第一条边后松开，草稿等待第二条边
        drag(*window, QPoint(300, 300), QPoint(400, 300));
        QVERIFY(window->m_draft.has_value());
        QCOMPARE(window->m_draft->points.size(), 3);
        QEvent ungrab(QEvent::UngrabMouse);
        QCoreApplication::sendEvent(window.get(), &ungrab);
        QCoreApplication::processEvents();
        QVERIFY(window->m_draft.has_value());

        // 2. 移动指针时第二条边跟随，单击定下后提交
        sendMouse(*window, QEvent::MouseMove, QPoint(300, 200), Qt::NoButton);
        QCOMPARE(window->m_draft->points.last(), QPointF(300, 200));
        click(*window, QPoint(300, 200));
        QVERIFY(!window->m_draft.has_value());
        const auto drawn = rulers(*window);
        QCOMPARE(drawn.size(), 1);
        QCOMPARE(drawn.first().rulerMode, RulerMode::Angle);
        QCOMPARE(drawn.first().points, (QVector<QPointF>{{300, 300}, {400, 300}, {300, 200}}));
        QCOMPARE(markshot::ruler::angleDegrees(drawn.first().points.at(0), drawn.first().points.at(1),
                                               drawn.first().points.at(2)),
                 90.0);

        // 3. 拖动顶点只移动顶点，两条边端点保持原位
        drag(*window, QPoint(300, 300), QPoint(320, 310));
        QCOMPARE(rulers(*window).first().points, (QVector<QPointF>{{320, 310}, {400, 300}, {300, 200}}));
    }

    /** 验证只单击也能逐点放下角度尺的三个点，无参数和返回值。 */
    void angleRulerAcceptsClicks()
    {
        auto window = createRulerWindow();
        QVERIFY(window);
        pressKey(*window, Qt::Key_U);
        click(*window, QPoint(500, 300));
        sendMouse(*window, QEvent::MouseMove, QPoint(600, 300), Qt::NoButton);
        click(*window, QPoint(600, 300));

        // 1. 第二条边落回顶点时长度为零，不提交，继续等待定点
        sendMouse(*window, QEvent::MouseMove, QPoint(500, 300), Qt::NoButton);
        click(*window, QPoint(500, 300));
        QVERIFY(window->m_draft.has_value());
        QVERIFY(rulers(*window).isEmpty());

        sendMouse(*window, QEvent::MouseMove, QPoint(600, 200), Qt::NoButton);
        click(*window, QPoint(600, 200));
        const auto drawn = rulers(*window);
        QCOMPARE(drawn.size(), 1);
        QCOMPARE(drawn.first().points, (QVector<QPointF>{{500, 300}, {600, 300}, {600, 200}}));
    }

    /** 验证 Esc 先撤去角度草稿，再按一次丢弃测量回到选区阶段，窗口不关闭，无参数和返回值。 */
    void escapeReturnsToSelection()
    {
        auto window = createRulerWindow();
        QVERIFY(window);
        drag(*window, QPoint(100, 100), QPoint(200, 180));
        pressKey(*window, Qt::Key_U);
        drag(*window, QPoint(300, 300), QPoint(400, 300));
        QVERIFY(window->m_draft.has_value());

        pressKey(*window, Qt::Key_Escape);
        QVERIFY(!window->m_draft.has_value());
        QCOMPARE(window->m_mode, Mode::Editing);

        pressKey(*window, Qt::Key_Escape);
        QCOMPARE(window->m_mode, Mode::Selecting);
        QVERIFY(window->isVisible());
        QVERIFY(window->m_annotations.isEmpty());
        QVERIFY(!window->m_ruler.enteredFromSelection);
    }

    /** 验证尺子画进导出图片，悬停十字线只在窗口中，无参数和返回值。 */
    void rulerIsExported()
    {
        auto window = createRulerWindow();
        QVERIFY(window);
        drag(*window, QPoint(100, 100), QPoint(300, 260));
        sendMouse(*window, QEvent::MouseMove, QPoint(500, 450), Qt::NoButton);
        QVERIFY(window->m_ruler.hoverValid);
        QCOMPARE(window->m_ruler.hoverImagePoint, QPointF(500, 450));

        // 1. 测量框左边落在 x=100，导出图片中该处带有标注颜色，悬停十字线所在的 x=500 保持空白
        const QImage exported = window->renderedSelection();
        QCOMPARE(exported.size(), QSize(800, 600));
        QVERIFY(exported.pixelColor(100, 180) != QColor(255, 255, 255));
        QCOMPARE(exported.pixelColor(500, 50), QColor(255, 255, 255));
    }

    /** 验证顶层 ruler 仍是选区阶段的尺子快捷键，tools 分组中的 ruler 才是尺子工具快捷键，无参数和返回值。 */
    void rulerShortcutKeysStaySeparate()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        QFile file(dir.filePath(QStringLiteral("config.json")));
        QVERIFY(file.open(QIODevice::WriteOnly));
        file.write(R"({"shortcuts": {"ruler": "X", "tools": {"ruler": "Y"}}})");
        file.close();
        const auto shortcuts = markshot::shortcut::configuredShortcuts(file.fileName());
        QCOMPARE(shortcuts.startupRuler, QKeySequence(Qt::Key_X));
        QCOMPARE(shortcuts.tools.at(markshot::shortcut::toolIndex(Tool::Ruler)), QKeySequence(Qt::Key_Y));
        QCOMPARE(markshot::shortcut::configuredShortcuts({}).tools.at(markshot::shortcut::toolIndex(Tool::Ruler)),
                 QKeySequence(Qt::Key_U));
    }

    /** 验证后一把尺子的尺寸说明避开前一把的说明，两段说明不重叠，无参数和返回值。 */
    void infoLabelsAvoidEachOther()
    {
        // 1. 第二把尺子在第一把右侧，说明默认放在下方时会压住第一段说明，应改放到上方
        const QRectF bounds(0, 0, 800, 600);
        const QRectF firstRuler(100, 100, 100, 80);
        const QRectF secondRuler(300, 120, 100, 60);
        const QVector<QRectF> labels = markshot::ruler::layoutRulerInfoLabels(
            {{firstRuler, firstRuler, 1.0, Qt::red}, {secondRuler, secondRuler, 1.0, Qt::red}}, bounds, {});
        QCOMPARE(labels.size(), 2);
        QVERIFY(labels.at(0).top() > firstRuler.bottom());
        QVERIFY(!labels.at(0).intersects(labels.at(1)));
        QVERIFY(labels.at(1).bottom() < secondRuler.top());
        QVERIFY(bounds.contains(labels.at(1)));
    }

    /** 验证三把错落的尺子排版后，每段说明既不压住其他尺子的刻度，也不互相重叠，无参数和返回值。 */
    void infoLabelsAvoidOtherRulerScales()
    {
        // 1. 第二把尺子的说明默认放在下方，会压住第三把尺子右上角的刻度
        QVector<markshot::ruler::RulerGeometry> geometries;
        for (const QRectF &rect : {QRectF(100, 120, 200, 120), QRectF(360, 300, 200, 120), QRectF(140, 460, 280, 80)}) {
            geometries.append({rect, rect, 1.0, Qt::red});
        }
        const QVector<QRectF> labels =
            markshot::ruler::layoutRulerInfoLabels(geometries, QRectF(0, 0, 1200, 800), {});
        QCOMPARE(labels.size(), geometries.size());

        // 2. 逐段检查说明与其他尺子的刻度、与其他说明都不相交
        for (int i = 0; i < labels.size(); ++i) {
            for (int j = 0; j < geometries.size(); ++j) {
                const QRectF scale = markshot::ruler::rulerScaleBounds(geometries.at(j).paintRect, 1.0);
                QVERIFY2(!labels.at(i).intersects(scale), qPrintable(QStringLiteral("label %1 ruler %2").arg(i).arg(j)));
                if (j != i) {
                    QVERIFY2(!labels.at(i).intersects(labels.at(j)), qPrintable(QStringLiteral("label %1 label %2").arg(i).arg(j)));
                }
            }
        }
    }
};

/**
 * 使用隔离配置运行尺子标注窗口回归。
 * @param argc 参数数量。
 * @param argv 参数列表。
 * @return 测试退出码。
 */
int main(int argc, char **argv)
{
    QTemporaryDir home;
    if (!markshot::test::isolateConfig(home)) return 1;
    QApplication app(argc, argv);
    RulerAnnotationTest test;
    return QTest::qExec(&test, argc, argv);
}

#include "ruler_annotation_test.moc"
