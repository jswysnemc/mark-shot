#include "shot_window.h"
#include "ruler/angle_painter.h"
#include "ruler/ruler_geometry.h"
#include "ruler/ruler_painter.h"
#include "support/isolated_config.h"

#include <QApplication>
#include <QKeyEvent>
#include <QMouseEvent>
#include <QPainter>
#include <QTemporaryDir>
#include <QtTest>

#include <cmath>
#include <memory>

namespace {

using markshot::shot::types::RulerMode;
using markshot::shot::types::Tool;

/**
 * 向截图窗口发送一次鼠标事件。
 * @param window 截图窗口。
 * @param type 事件类型。
 * @param position 窗口坐标。
 * @param buttons 事件发生后仍按住的按键。
 * @param modifiers 按住的修饰键。
 * @return 无返回值。
 */
void sendMouse(ShotWindow &window, QEvent::Type type, QPointF position, Qt::MouseButtons buttons,
               Qt::KeyboardModifiers modifiers = Qt::NoModifier)
{
    const Qt::MouseButton button = type == QEvent::MouseMove ? Qt::NoButton : Qt::LeftButton;
    QMouseEvent event(type, position, window.mapToGlobal(position), button, buttons, modifiers);
    QCoreApplication::sendEvent(&window, &event);
}

/**
 * 用左键从起点拖到终点，中途发送一次移动事件。
 * @param window 截图窗口。
 * @param from 按下位置，窗口坐标。
 * @param to 松开位置，窗口坐标。
 * @param modifiers 拖动全程按住的修饰键。
 * @return 无返回值。
 */
void drag(ShotWindow &window, QPointF from, QPointF to, Qt::KeyboardModifiers modifiers = Qt::NoModifier)
{
    sendMouse(window, QEvent::MouseButtonPress, from, Qt::LeftButton, modifiers);
    sendMouse(window, QEvent::MouseMove, (from + to) / 2.0, Qt::LeftButton, modifiers);
    sendMouse(window, QEvent::MouseMove, to, Qt::LeftButton, modifiers);
    sendMouse(window, QEvent::MouseButtonRelease, to, Qt::NoButton, modifiers);
}

/**
 * 在同一位置按下并松开左键。
 * @param window 截图窗口。
 * @param position 窗口坐标。
 * @param modifiers 按住的修饰键。
 * @return 无返回值。
 */
void click(ShotWindow &window, QPointF position, Qt::KeyboardModifiers modifiers = Qt::NoModifier)
{
    sendMouse(window, QEvent::MouseButtonPress, position, Qt::LeftButton, modifiers);
    sendMouse(window, QEvent::MouseButtonRelease, position, Qt::NoButton, modifiers);
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

/**
 * 比较两个点是否在浮点误差内重合。
 * @param actual 实际位置。
 * @param expected 期望位置。
 * @return 两点距离不超过 0.01 时返回 true。
 */
bool samePoint(QPointF actual, QPointF expected)
{
    return QLineF(actual, expected).length() <= 0.01;
}

/**
 * 计算绕轴心转过指定角度后的点，与 QPainter::rotate 同向。
 * @param center 轴心。
 * @param radius 到轴心的距离。
 * @param degrees 方向角度，向右为 0、屏幕上顺时针为正。
 * @return 点的位置。
 */
QPointF pointAt(QPointF center, qreal radius, qreal degrees)
{
    const qreal radians = degrees * M_PI / 180.0;
    return center + QPointF(std::cos(radians), std::sin(radians)) * radius;
}

/**
 * 统计一块区域中不透明度超过一半的像素，用来判断刻度数字是否画在这里。
 * @param image 绘制结果，透明背景。
 * @param area 待统计区域。
 * @return 像素数量。
 */
int opaquePixels(const QImage &image, const QRect &area)
{
    int count = 0;
    const QRect clipped = area.intersected(image.rect());
    for (int y = clipped.top(); y <= clipped.bottom(); ++y) {
        for (int x = clipped.left(); x <= clipped.right(); ++x) {
            count += image.pixelColor(x, y).alpha() > 128 ? 1 : 0;
        }
    }
    return count;
}

}  // namespace

/// @brief 检查尺子和其他标注的旋转把手、Shift 吸附、角度尺选中框和刻度数字靠边换侧
class RulerRotationTest final : public QObject {
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
     * 按编号读取标注。
     * @param window 截图窗口。
     * @param id 标注编号。
     * @return 标注，不存在时返回默认值。
     */
    static ShotWindow::Annotation annotation(const ShotWindow &window, int id)
    {
        const ShotWindow::Annotation *found = window.annotationById(id);
        return found ? *found : ShotWindow::Annotation{};
    }

    /**
     * 在窗口中直接放入一个矩形标注。
     * @param window 截图窗口。
     * @param rect 矩形范围，图像像素坐标。
     * @return 新标注的编号。
     */
    static int addRectangle(ShotWindow &window, const QRectF &rect)
    {
        ShotWindow::Annotation annotation;
        annotation.id = window.m_nextAnnotationId++;
        annotation.tool = Tool::Rectangle;
        annotation.rect = rect;
        window.m_annotations.append(annotation);
        return annotation.id;
    }

    /**
     * 把一把长度尺的测量框和刻度画到 800x600 的透明图片上。
     * @param rect 测量范围，图像和绘制坐标一致。
     * @param bounds 刻度数字允许出现的范围。
     * @return 绘制结果。
     */
    static QImage renderScale(const QRectF &rect, const QRectF &bounds)
    {
        QImage image(800, 600, QImage::Format_ARGB32_Premultiplied);
        image.fill(Qt::transparent);
        QPainter painter(&image);
        painter.setRenderHint(QPainter::Antialiasing, true);
        markshot::ruler::drawRulerScale(painter, {rect, rect, 1.0, QColor(220, 38, 38), 0.0, bounds});
        return image;
    }

private slots:
    /** 验证选中的长度尺显示旋转把手，拖动把手后测量框绕中心旋转，无参数和返回值。 */
    void rotationHandleRotatesLengthRuler()
    {
        auto window = createRulerWindow();
        QVERIFY(window);
        drag(*window, QPoint(100, 100), QPoint(300, 260));
        click(*window, QPoint(200, 100));
        QCOMPARE(window->selectedAnnotationIds().size(), 1);
        const int id = window->selectedAnnotationIds().first();

        // 1. 把手在上边刻度数字之外：数字伸出 30 像素，再留 20 像素连线；拖到中心右侧转过 90 度
        const QPointF handle = window->annotationRotationHandlePoint(annotation(*window, id), false);
        QVERIFY(samePoint(handle, QPointF(200, 50)));
        drag(*window, handle, QPointF(340, 180));
        QVERIFY(std::abs(annotation(*window, id).rotationDegrees - 90.0) < 0.01);
        QCOMPARE(annotation(*window, id).rect, QRectF(100, 100, 200, 160));
    }

    /** 验证按住 Shift 拖动长度尺的旋转把手时角度吸附到 45 度倍数，无参数和返回值。 */
    void shiftSnapsLengthRulerRotation()
    {
        auto window = createRulerWindow();
        QVERIFY(window);
        drag(*window, QPoint(100, 100), QPoint(300, 260));
        click(*window, QPoint(200, 100));
        const int id = window->selectedAnnotationIds().first();

        // 1. 指针转过约 50 度，吸附后为 45 度
        const QPointF handle = window->annotationRotationHandlePoint(annotation(*window, id), false);
        drag(*window, handle, pointAt(QPointF(200, 180), 100, -40), Qt::ShiftModifier);
        QCOMPARE(annotation(*window, id).rotationDegrees, 45.0);
    }

    /** 验证旋转后的长度尺拖角时，对角在屏幕上保持不动，无参数和返回值。 */
    void rotatedRulerResizeKeepsOppositeCorner()
    {
        auto window = createRulerWindow();
        QVERIFY(window);
        drag(*window, QPoint(100, 100), QPoint(300, 260));
        click(*window, QPoint(200, 100));
        const int id = window->selectedAnnotationIds().first();
        window->annotationById(id)->rotationDegrees = 90.0;

        // 1. 旋转 90 度后右下角显示在 (120, 280)，左上角显示在 (280, 80)
        drag(*window, QPointF(120, 280), QPointF(100, 300));
        const ShotWindow::Annotation resized = annotation(*window, id);
        QCOMPARE(resized.rect, QRectF(80, 100, 220, 180));
        const QPointF fixedCorner =
            window->rotatedPoint(resized.rect.topLeft(), resized.rect.center(), resized.rotationDegrees);
        QVERIFY(samePoint(fixedCorner, QPointF(280, 80)));
    }

    /** 验证按住 Shift 拖动其他标注的旋转把手时同样吸附到 45 度倍数，无参数和返回值。 */
    void shiftSnapsAnnotationRotation()
    {
        auto window = createRulerWindow();
        QVERIFY(window);
        const int id = addRectangle(*window, QRectF(300, 200, 200, 100));
        window->setTool(Tool::Select);
        window->setSelectedAnnotations({id});

        // 1. 指针转过约 80 度，吸附后为 90 度
        const QPointF handle = window->annotationRotationHandlePoint(annotation(*window, id), false);
        drag(*window, handle, pointAt(QPointF(400, 250), 100, -10), Qt::ShiftModifier);
        QCOMPARE(annotation(*window, id).rotationDegrees, 90.0);
    }

    /** 验证多选时按住 Shift 拖动旋转把手，整组转过的角度吸附到 45 度倍数，无参数和返回值。 */
    void shiftSnapsGroupRotation()
    {
        auto window = createRulerWindow();
        QVERIFY(window);
        const int first = addRectangle(*window, QRectF(200, 150, 100, 60));
        const int second = addRectangle(*window, QRectF(400, 250, 100, 60));
        window->setTool(Tool::Select);
        window->setSelectedAnnotations({first, second});

        // 1. 组中心 (350, 230)，指针转过约 80 度，吸附后整组转 90 度
        const QPointF handle = window->selectionRotationHandlePoint(window->selectedAnnotationsBounds(), false);
        drag(*window, handle, pointAt(QPointF(350, 230), 100, -10), Qt::ShiftModifier);
        QCOMPARE(annotation(*window, first).rotationDegrees, 90.0);
        QCOMPARE(annotation(*window, second).rotationDegrees, 90.0);
        QVERIFY(samePoint(annotation(*window, first).rect.center(), QPointF(400, 130)));
    }

    /** 验证选中的角度尺画出三点外接矩形的虚线选中框，无参数和返回值。 */
    void angleRulerHasSelectionFrame()
    {
        auto window = createRulerWindow();
        QVERIFY(window);
        pressKey(*window, Qt::Key_U);
        drag(*window, QPoint(300, 300), QPoint(400, 300));
        sendMouse(*window, QEvent::MouseMove, QPoint(300, 200), Qt::NoButton);
        click(*window, QPoint(300, 200));
        click(*window, QPoint(350, 300));
        QCOMPARE(window->selectedAnnotationIds().size(), 1);
        sendMouse(*window, QEvent::MouseMove, QPoint(700, 550), Qt::NoButton);

        // 1. 外接矩形上边 y=200 外扩 6 像素后，x=320 到 x=380 之间是橙色虚线，选中框的线段占一部分像素
        const QImage frame = window->grab().toImage();
        int framePixels = 0;
        for (int x = 320; x <= 380; ++x) {
            const QColor color = frame.pixelColor(x, 194);
            framePixels += color.red() > 200 && color.green() > 110 && color.green() < 180 && color.blue() < 110 ? 1 : 0;
        }
        QVERIFY2(framePixels > 15, qPrintable(QString::number(framePixels)));
    }

    /** 验证拖动角度尺的旋转把手时三个定点绕外接矩形中心转动，夹角不变，无参数和返回值。 */
    void rotationHandleRotatesAngleRuler()
    {
        auto window = createRulerWindow();
        QVERIFY(window);
        pressKey(*window, Qt::Key_U);
        drag(*window, QPoint(300, 300), QPoint(400, 300));
        sendMouse(*window, QEvent::MouseMove, QPoint(300, 200), Qt::NoButton);
        click(*window, QPoint(300, 200));
        click(*window, QPoint(350, 300));
        const int id = window->selectedAnnotationIds().first();

        // 1. 外接矩形中心 (350, 250)，把手在 (350, 174)，拖到中心右侧转过 90 度
        const QPointF handle = window->annotationRotationHandlePoint(annotation(*window, id), false);
        QVERIFY(samePoint(handle, QPointF(350, 174)));
        drag(*window, handle, QPointF(450, 250));
        const QVector<QPointF> points = annotation(*window, id).points;
        QCOMPARE(points.size(), 3);
        // 2. 顺时针转 90 度后第一条边从朝右变为朝下，第二条边从朝上变为朝右
        QVERIFY(samePoint(points.at(0), QPointF(300, 200)));
        QVERIFY(samePoint(points.at(1), QPointF(300, 300)));
        QVERIFY(samePoint(points.at(2), QPointF(400, 200)));
        QVERIFY(std::abs(markshot::ruler::angleDegrees(points.at(0), points.at(1), points.at(2)) - 90.0) < 0.01);
        QCOMPARE(annotation(*window, id).rotationDegrees, 0.0);
    }

    /** 验证按住 Shift 旋转角度尺时第一条边吸附到 45 度倍数，无参数和返回值。 */
    void shiftSnapsAngleRulerRotation()
    {
        auto window = createRulerWindow();
        QVERIFY(window);
        pressKey(*window, Qt::Key_U);
        drag(*window, QPoint(300, 300), QPoint(400, 300));
        sendMouse(*window, QEvent::MouseMove, QPoint(300, 200), Qt::NoButton);
        click(*window, QPoint(300, 200));
        click(*window, QPoint(350, 300));
        const int id = window->selectedAnnotationIds().first();

        // 1. 第一条边原本朝右，指针转过约 30 度，吸附后这条边朝向 45 度
        const QPointF handle = window->annotationRotationHandlePoint(annotation(*window, id), false);
        drag(*window, handle, pointAt(QPointF(350, 250), 100, -60), Qt::ShiftModifier);
        QVERIFY(std::abs(markshot::ruler::angleFirstArmDegrees(annotation(*window, id)) - 45.0) < 0.01);
    }

    /** 验证画角度尺时按住 Shift，两条边从顶点出发吸附到 45 度倍数，无参数和返回值。 */
    void shiftSnapsAngleDraftArms()
    {
        auto window = createRulerWindow();
        QVERIFY(window);
        pressKey(*window, Qt::Key_U);

        // 1. 第一条边略向下偏，吸附成水平；第二条边略向右偏，吸附成竖直
        drag(*window, QPoint(300, 300), QPoint(400, 310), Qt::ShiftModifier);
        QVERIFY(window->m_draft.has_value());
        QVERIFY(samePoint(window->m_draft->points.at(1), QPointF(400, 300)));
        sendMouse(*window, QEvent::MouseMove, QPoint(310, 200), Qt::NoButton, Qt::ShiftModifier);
        QVERIFY(samePoint(window->m_draft->points.last(), QPointF(300, 200)));
        click(*window, QPoint(310, 200), Qt::ShiftModifier);

        const ShotWindow::Annotation angle = window->m_annotations.last();
        QCOMPARE(angle.rulerMode, RulerMode::Angle);
        QVERIFY(std::abs(markshot::ruler::angleDegrees(angle.points.at(0), angle.points.at(1), angle.points.at(2))
                         - 90.0) < 0.01);
    }

    /** 验证按住 Shift 拖动角度尺的边端点时，这条边从顶点出发吸附到 45 度倍数，无参数和返回值。 */
    void shiftSnapsAngleArmDrag()
    {
        auto window = createRulerWindow();
        QVERIFY(window);
        pressKey(*window, Qt::Key_U);
        drag(*window, QPoint(300, 300), QPoint(400, 300));
        sendMouse(*window, QEvent::MouseMove, QPoint(300, 200), Qt::NoButton);
        click(*window, QPoint(300, 200));
        const int id = window->m_annotations.last().id;

        // 1. 端点拖到约 39 度方向，吸附成 45 度并投影到这条方向上
        drag(*window, QPoint(400, 300), QPoint(400, 380), Qt::ShiftModifier);
        QVERIFY(samePoint(annotation(*window, id).points.at(1), QPointF(390, 390)));
        QVERIFY(samePoint(annotation(*window, id).points.at(0), QPointF(300, 300)));
    }

    /** 验证尺子靠近上边和左边时，刻度数字换到下方和右侧，不跑出允许范围，无参数和返回值。 */
    void tickNumbersFlipAwayFromEdges()
    {
        // 1. 默认位置在 y<0 和 x<0，换侧后数字画在测量框下方和右侧；统计区域避开向外伸出的刻度线
        const QImage image = renderScale(QRectF(20, 20, 200, 100), QRectF(0, 0, 800, 600));
        QVERIFY(opaquePixels(image, QRect(30, 136, 180, 12)) > 20);
        QVERIFY(opaquePixels(image, QRect(236, 30, 26, 80)) > 20);

        // 2. 远离边缘时数字仍在上方和左侧
        const QImage centered = renderScale(QRectF(300, 300, 200, 100), QRectF(0, 0, 800, 600));
        QVERIFY(opaquePixels(centered, QRect(310, 272, 180, 12)) > 20);
        QCOMPARE(opaquePixels(centered, QRect(310, 416, 180, 12)), 0);
    }

    /** 验证尺子几乎占满范围时，刻度数字移进范围内，仍然看得见，无参数和返回值。 */
    void tickNumbersClampInsideBounds()
    {
        // 1. 左右两侧都放不下纵向数字，数字条带贴着范围左边，落在测量框内；统计区域避开边框和刻度线
        const QImage image = renderScale(QRectF(5, 5, 790, 590), QRectF(0, 0, 800, 600));
        QVERIFY(opaquePixels(image, QRect(9, 40, 17, 520)) > 40);
        // 2. 横向数字条带贴着范围上边，落在 y 0 到 16 之间
        QVERIFY(opaquePixels(image, QRect(40, 9, 720, 8)) > 40);
    }
};

/**
 * 使用隔离配置运行尺子旋转与刻度换侧回归。
 * @param argc 参数数量。
 * @param argv 参数列表。
 * @return 测试退出码。
 */
int main(int argc, char **argv)
{
    QTemporaryDir home;
    if (!markshot::test::isolateConfig(home)) return 1;
    QApplication app(argc, argv);
    RulerRotationTest test;
    return QTest::qExec(&test, argc, argv);
}

#include "ruler_rotation_test.moc"
