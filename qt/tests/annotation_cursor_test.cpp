#include "capture_cross_cursor.h"
#include "shot_window.h"
#include "ui/interaction_cursor.h"

#include <QApplication>
#include <QDir>
#include <QEnterEvent>
#include <QEvent>
#include <QMouseEvent>
#include <QPushButton>
#include <QTemporaryDir>
#include <QTextEdit>
#include <QWheelEvent>
#include <QtTest>

#include <memory>

namespace {

const QColor kImageColor(91, 117, 143);

/// @brief 根据截图中独特的底色定位实际图像区域，返回图像在窗口内的边界
/// @param window 正在测试的实际标注窗口
QRect imageBounds(ShotWindow &window)
{
    const QImage image = window.grab().toImage();
    QRect result;
    for (int y = 0; y < image.height(); ++y) {
        for (int x = 0; x < image.width(); ++x) {
            if (image.pixelColor(x, y) == kImageColor) {
                result |= QRect(x, y, 1, 1);
            }
        }
    }
    return result;
}

/// @brief 查找真实工具按钮
/// @param window 标注窗口
/// @param action 工具按钮的动作名称
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

/// @brief 通过真实工具按钮切换工具
/// @param window 标注窗口
/// @param action 工具按钮的动作名称
/// @return 是否找到并触发指定入口
bool chooseTool(ShotWindow &window, const char *action)
{
    if (QPushButton *button = toolButton(window, action)) {
        button->click();
        QCoreApplication::processEvents();
        return true;
    }
    return false;
}

/// @brief 发送包含明确按键状态的真实 Qt 鼠标事件，无返回值
/// @param window 标注窗口
/// @param type 鼠标事件类型
/// @param position 窗口内位置
/// @param button 本次变化的按键
/// @param buttons 事件发生后的完整按键状态
void pointerEvent(ShotWindow &window, QEvent::Type type, QPoint position,
                  Qt::MouseButton button = Qt::NoButton, Qt::MouseButtons buttons = Qt::NoButton)
{
    const QPoint global = window.mapToGlobal(position);
    // 1. 【标注测试】【指针事件】仅空闲移动同步系统位置，避免测试产生额外的无按键移动
    if (type == QEvent::MouseMove && buttons == Qt::NoButton) {
        QCursor::setPos(global);
        QCoreApplication::processEvents();
    }
    QMouseEvent event(type, QPointF(position), QPointF(global), button, buttons, Qt::NoModifier);
    QCoreApplication::sendEvent(&window, &event);
    if (type == QEvent::MouseButtonRelease && buttons == Qt::NoButton) {
        QCursor::setPos(global);
    }
    QCoreApplication::processEvents();
}

/// @brief 向实际标注窗口发送一格滚轮事件，无返回值
/// @param window 标注窗口
/// @param position 滚轮所在的窗口内位置
/// @param buttons 同时按住的鼠标按键
void wheelEvent(ShotWindow &window, QPoint position, Qt::MouseButtons buttons = Qt::NoButton)
{
    if (buttons == Qt::NoButton) {
        QCursor::setPos(window.mapToGlobal(position));
        QCoreApplication::processEvents();
    }
    QWheelEvent event(QPointF(position), QPointF(window.mapToGlobal(position)), {}, QPoint(0, 120),
                      buttons, Qt::NoModifier, Qt::NoScrollPhase, false);
    QCoreApplication::sendEvent(&window, &event);
    QCoreApplication::processEvents();
}

/// @brief 通过完整按下、移动、释放手势创建一笔标注，无返回值
/// @param window 标注窗口
/// @param start 窗口内起点
/// @param end 窗口内终点
void drawStroke(ShotWindow &window, QPoint start, QPoint end)
{
    pointerEvent(window, QEvent::MouseButtonPress, start, Qt::LeftButton, Qt::LeftButton);
    pointerEvent(window, QEvent::MouseMove, end, Qt::NoButton, Qt::LeftButton);
    pointerEvent(window, QEvent::MouseButtonRelease, end, Qt::LeftButton);
}

}  // namespace

/// @brief 通过实际标注窗口检查完整光标状态切换，避免只测试静态映射
class AnnotationCursorTest final : public QObject {
    Q_OBJECT

private slots:
    /// @brief 为每项检查创建独立标注画布，无参数和返回值
    void init()
    {
        QImage image(640, 400, QImage::Format_RGB32);
        image.fill(kImageColor);
        m_window = std::make_unique<ShotWindow>(image, QString(), QRect(0, 0, 640, 400),
                                                QVector<markshot::WindowInfo>{}, false);
        m_window->setAttribute(Qt::WA_DeleteOnClose, false);
        // 窗口高度留足余量：全屏工具栏决定最小宽度，加宽后图片按宽度缩放，上下仍留出画布外的区域
        m_window->resize(960, 800);
        m_window->startFullscreenAnnotation();
        m_window->setImageNavigationEnabled(true);
        m_window->show();
        QVERIFY(QTest::qWaitForWindowExposed(m_window.get()));
        QCoreApplication::processEvents();
        m_image = imageBounds(*m_window);
        QVERIFY2(m_image.width() > 300 && m_image.height() > 150, "The production canvas did not display the image");
        m_inside = m_image.center();
        pointerEvent(*m_window, QEvent::MouseMove, m_inside);
    }

    /// @brief 清理测试窗口及延迟销毁事件，无参数和返回值
    void cleanup()
    {
        m_window.reset();
        QCoreApplication::sendPostedEvents(nullptr, QEvent::DeferredDelete);
    }

    /// @brief 列出所有绘制工具及其预期光标，无参数和返回值
    void drawingCursorFollowsCanvas_data()
    {
        QTest::addColumn<QString>("action");
        QTest::addColumn<int>("cursorShape");
        for (const char *action : {"Pen", "Line", "Rect", "Ellipse", "Arrow", "Highlighter",
                                   "Mosaic", "Number", "Magnifier", "Laser", "Marker"}) {
            QTest::newRow(action) << QString::fromLatin1(action) << int(Qt::BitmapCursor);
        }
        QTest::newRow("Text") << QStringLiteral("Text") << int(Qt::IBeamCursor);
    }

    /// @brief 验证各工具光标随画布边界双向切换，无参数和返回值
    void drawingCursorFollowsCanvas()
    {
        QFETCH(QString, action);
        QFETCH(int, cursorShape);
        QVERIFY(chooseTool(*m_window, action.toUtf8().constData()));
        pointerEvent(*m_window, QEvent::MouseMove, m_inside);
        QCOMPARE(int(m_window->cursor().shape()), cursorShape);
        const QPoint outside(3, m_window->height() - 3);
        QVERIFY(!m_image.contains(outside));
        pointerEvent(*m_window, QEvent::MouseMove, outside);
        QCOMPARE(m_window->cursor().shape(), Qt::ArrowCursor);
        pointerEvent(*m_window, QEvent::MouseMove, m_inside);
        QCOMPARE(int(m_window->cursor().shape()), cursorShape);
    }

    /// @brief 验证绘制拖出画布时保留定位光标，正常释放仍提交笔迹，无参数和返回值
    void drawingOutsideCanvasKeepsCrosshairUntilRelease()
    {
        QVERIFY(chooseTool(*m_window, "Pen"));
        const QPoint outside(m_inside.x(), m_window->height() - 3);
        QVERIFY(!m_image.contains(outside));
        pointerEvent(*m_window, QEvent::MouseButtonPress, m_inside, Qt::LeftButton, Qt::LeftButton);
        pointerEvent(*m_window, QEvent::MouseMove, outside, Qt::NoButton, Qt::LeftButton);
        QCOMPARE(m_window->cursor().shape(), Qt::BitmapCursor);
        pointerEvent(*m_window, QEvent::MouseButtonRelease, outside, Qt::LeftButton);
        QCOMPARE(m_window->cursor().shape(), Qt::ArrowCursor);
        QVERIFY(m_window->grab().toImage().pixelColor(m_inside + QPoint(0, 60)) != kImageColor);
    }

    /// @brief 验证空白处框选使用统一十字，结束后恢复箭头，无参数和返回值
    void selectionBoxUsesUnifiedCrosshair()
    {
        QVERIFY(chooseTool(*m_window, "Select"));
        pointerEvent(*m_window, QEvent::MouseMove, m_inside);
        QCOMPARE(m_window->cursor().shape(), Qt::ArrowCursor);
        pointerEvent(*m_window, QEvent::MouseButtonPress, m_inside, Qt::LeftButton, Qt::LeftButton);
        QCOMPARE(m_window->cursor().shape(), Qt::BitmapCursor);
        pointerEvent(*m_window, QEvent::MouseMove, m_inside + QPoint(60, 40), Qt::NoButton, Qt::LeftButton);
        QCOMPARE(m_window->cursor().shape(), Qt::BitmapCursor);
        pointerEvent(*m_window, QEvent::MouseButtonRelease, m_inside + QPoint(60, 40), Qt::LeftButton);
        QCOMPARE(m_window->cursor().shape(), Qt::ArrowCursor);
    }

    /// @brief 验证图像缩放比例提示不隐藏鼠标，无参数和返回值
    void zoomKeepsPointerVisible()
    {
        QVERIFY(chooseTool(*m_window, "Select"));
        pointerEvent(*m_window, QEvent::MouseMove, m_inside);
        wheelEvent(*m_window, m_inside);
        QVERIFY(m_window->cursor().shape() != Qt::BlankCursor);
    }

    /// @brief 验证画笔预览无需再次移动鼠标即可结束，无参数和返回值
    void brushPreviewExpiresWithoutMotion()
    {
        m_window->setImageNavigationEnabled(false);
        QVERIFY(chooseTool(*m_window, "Pen"));
        pointerEvent(*m_window, QEvent::MouseMove, m_inside);
        wheelEvent(*m_window, m_inside);
        QCOMPARE(m_window->cursor().shape(), Qt::BlankCursor);
        QTRY_COMPARE_WITH_TIMEOUT(m_window->cursor().shape(), Qt::BitmapCursor, 1500);
    }

    /// @brief 验证离开窗口时清除预览，重新进入后立即恢复工具光标，无参数和返回值
    void leavingCanvasClearsPreview()
    {
        m_window->setImageNavigationEnabled(false);
        QVERIFY(chooseTool(*m_window, "Pen"));
        wheelEvent(*m_window, m_inside);
        QCOMPARE(m_window->cursor().shape(), Qt::BlankCursor);
        QEvent leave(QEvent::Leave);
        QCoreApplication::sendEvent(m_window.get(), &leave);
        QCOMPARE(m_window->cursor().shape(), Qt::ArrowCursor);
        QEnterEvent enter(m_inside, m_inside, m_window->mapToGlobal(m_inside));
        QCoreApplication::sendEvent(m_window.get(), &enter);
        QCOMPARE(m_window->cursor().shape(), Qt::BitmapCursor);
    }

    /// @brief 验证绘制期间滚轮不会切换为尺寸预览或改变图像位置，无参数和返回值
    void wheelDoesNotInterruptDrawing()
    {
        QVERIFY(chooseTool(*m_window, "Pen"));
        pointerEvent(*m_window, QEvent::MouseButtonPress, m_inside - QPoint(80, 0), Qt::LeftButton, Qt::LeftButton);
        pointerEvent(*m_window, QEvent::MouseMove, m_inside + QPoint(80, 0), Qt::NoButton, Qt::LeftButton);
        wheelEvent(*m_window, m_inside + QPoint(80, 0), Qt::LeftButton);
        QCOMPARE(m_window->cursor().shape(), Qt::BitmapCursor);
        QCOMPARE(imageBounds(*m_window), m_image);
        pointerEvent(*m_window, QEvent::MouseButtonRelease, m_inside + QPoint(80, 0), Qt::LeftButton);
        QVERIFY(m_window->grab().toImage().pixelColor(m_inside) != kImageColor);
    }

    /// @brief 验证中键释放事件缺失时不会继续平移图片或保留闭合手型，无参数和返回值
    void lostPanButtonRestoresCursor()
    {
        QVERIFY(chooseTool(*m_window, "Select"));
        pointerEvent(*m_window, QEvent::MouseButtonPress, m_inside, Qt::MiddleButton, Qt::MiddleButton);
        QCOMPARE(m_window->cursor().shape(), Qt::ClosedHandCursor);
        pointerEvent(*m_window, QEvent::MouseMove, m_inside + QPoint(30, 20));
        QCOMPARE(m_window->cursor().shape(), Qt::ArrowCursor);
        QCOMPARE(imageBounds(*m_window), m_image);
    }

    /// @brief 验证中键平移只由中键释放结束，无参数和返回值
    void panIgnoresUnrelatedButtonRelease()
    {
        QVERIFY(chooseTool(*m_window, "Select"));
        pointerEvent(*m_window, QEvent::MouseButtonPress, m_inside, Qt::MiddleButton, Qt::MiddleButton);
        pointerEvent(*m_window, QEvent::MouseButtonRelease, m_inside, Qt::LeftButton, Qt::MiddleButton);
        QCOMPARE(m_window->cursor().shape(), Qt::ClosedHandCursor);
        wheelEvent(*m_window, m_inside, Qt::MiddleButton);
        QCOMPARE(m_window->cursor().shape(), Qt::ClosedHandCursor);
        pointerEvent(*m_window, QEvent::MouseButtonRelease, m_inside, Qt::MiddleButton);
        QCOMPARE(m_window->cursor().shape(), Qt::ArrowCursor);
    }

    /// @brief 验证平移接管绘制时撤去草稿，并独占后续按键直到中键释放，无参数和返回值
    void panTakesOverDraftWithoutStartingAnother()
    {
        QVERIFY(chooseTool(*m_window, "Pen"));
        const QPoint start = m_inside - QPoint(80, 0);
        const QPoint end = m_inside + QPoint(80, 0);
        pointerEvent(*m_window, QEvent::MouseButtonPress, start, Qt::LeftButton, Qt::LeftButton);
        pointerEvent(*m_window, QEvent::MouseMove, end, Qt::NoButton, Qt::LeftButton);
        QVERIFY(m_window->grab().toImage().pixelColor(m_inside) != kImageColor);
        pointerEvent(*m_window, QEvent::MouseButtonPress, end, Qt::MiddleButton, Qt::LeftButton | Qt::MiddleButton);
        QCOMPARE(m_window->cursor().shape(), Qt::ClosedHandCursor);
        QCOMPARE(m_window->grab().toImage().pixelColor(m_inside), kImageColor);
        pointerEvent(*m_window, QEvent::MouseButtonRelease, end, Qt::LeftButton, Qt::MiddleButton);
        pointerEvent(*m_window, QEvent::MouseButtonPress, start, Qt::LeftButton, Qt::LeftButton | Qt::MiddleButton);
        pointerEvent(*m_window, QEvent::MouseButtonRelease, start, Qt::LeftButton, Qt::MiddleButton);
        pointerEvent(*m_window, QEvent::MouseButtonRelease, start, Qt::MiddleButton);
        pointerEvent(*m_window, QEvent::MouseMove, end);
        QCOMPARE(m_window->cursor().shape(), Qt::BitmapCursor);
        QCOMPARE(m_window->grab().toImage().pixelColor(m_inside), kImageColor);
    }

    /// @brief 验证切换工具会撤去未提交的绘制草稿，无参数和返回值
    void switchingToolDiscardsActiveDraft()
    {
        QVERIFY(chooseTool(*m_window, "Pen"));
        const QPoint start = m_inside - QPoint(80, 0);
        pointerEvent(*m_window, QEvent::MouseButtonPress, start, Qt::LeftButton, Qt::LeftButton);
        pointerEvent(*m_window, QEvent::MouseMove, m_inside + QPoint(80, 0), Qt::NoButton, Qt::LeftButton);
        QVERIFY(m_window->grab().toImage().pixelColor(m_inside) != kImageColor);
        QVERIFY(chooseTool(*m_window, "Ellipse"));
        QCOMPARE(m_window->grab().toImage().pixelColor(m_inside), kImageColor);
        pointerEvent(*m_window, QEvent::MouseButtonRelease, m_inside + QPoint(80, 0), Qt::LeftButton);
        QCOMPARE(m_window->grab().toImage().pixelColor(m_inside), kImageColor);
    }

    /// @brief 列出需要恢复指针状态的中断事件，无参数和返回值
    void interruptedDrawingDiscardsDraft_data()
    {
        QTest::addColumn<int>("interruption");
        QTest::newRow("ungrab") << int(QEvent::UngrabMouse);
        QTest::newRow("deactivate") << int(QEvent::WindowDeactivate);
        QTest::newRow("hide") << int(QEvent::Hide);
        QTest::newRow("missing-release") << int(QEvent::MouseMove);
    }

    /// @brief 验证抓取中断撤去草稿，后续笔迹仍可正常完成，无参数和返回值
    void interruptedDrawingDiscardsDraft()
    {
        QFETCH(int, interruption);
        QVERIFY(chooseTool(*m_window, "Pen"));
        const QPoint start = m_inside - QPoint(80, 0);
        const QPoint end = m_inside + QPoint(80, 0);
        pointerEvent(*m_window, QEvent::MouseButtonPress, start, Qt::LeftButton, Qt::LeftButton);
        pointerEvent(*m_window, QEvent::MouseMove, end, Qt::NoButton, Qt::LeftButton);
        QVERIFY(m_window->grab().toImage().pixelColor(m_inside) != kImageColor);
        if (interruption == QEvent::Hide) {
            m_window->hide();
            m_window->show();
        } else if (interruption == QEvent::MouseMove) {
            pointerEvent(*m_window, QEvent::MouseMove, end);
        } else {
            QEvent event{QEvent::Type(interruption)};
            QCoreApplication::sendEvent(m_window.get(), &event);
        }
        QCoreApplication::processEvents();
        QCOMPARE(m_window->grab().toImage().pixelColor(m_inside), kImageColor);
        pointerEvent(*m_window, QEvent::MouseButtonRelease, end, Qt::LeftButton);
        QCOMPARE(m_window->grab().toImage().pixelColor(m_inside), kImageColor);
        drawStroke(*m_window, start, end);
        QVERIFY(m_window->grab().toImage().pixelColor(m_inside) != kImageColor);
    }

    /// @brief 验证正常释放之前到达的取消抓取事件不会吞掉已完成笔迹，无参数和返回值
    void ungrabBeforeReleaseStillCommitsStroke()
    {
        QVERIFY(chooseTool(*m_window, "Pen"));
        pointerEvent(*m_window, QEvent::MouseButtonPress, m_inside - QPoint(80, 0), Qt::LeftButton, Qt::LeftButton);
        pointerEvent(*m_window, QEvent::MouseMove, m_inside + QPoint(80, 0), Qt::NoButton, Qt::LeftButton);
        QEvent ungrab(QEvent::UngrabMouse);
        QCoreApplication::sendEvent(m_window.get(), &ungrab);
        pointerEvent(*m_window, QEvent::MouseButtonRelease, m_inside + QPoint(80, 0), Qt::LeftButton);
        QVERIFY(m_window->grab().toImage().pixelColor(m_inside) != kImageColor);
        QCOMPARE(m_window->cursor().shape(), Qt::BitmapCursor);
    }

    /// @brief 验证延迟的抓取清理不会取消紧接着开始的新操作，无参数和返回值
    void oldUngrabDoesNotCancelNewStroke()
    {
        QVERIFY(chooseTool(*m_window, "Pen"));
        pointerEvent(*m_window, QEvent::MouseButtonPress, m_inside, Qt::LeftButton, Qt::LeftButton);
        QEvent ungrab(QEvent::UngrabMouse);
        QCoreApplication::sendEvent(m_window.get(), &ungrab);
        drawStroke(*m_window, m_inside - QPoint(80, 0), m_inside + QPoint(80, 0));
        QVERIFY(m_window->grab().toImage().pixelColor(m_inside) != kImageColor);
    }

    /// @brief 验证悬停命中不会污染移动状态，抓取中断后标注停止移动，无参数和返回值
    void annotationMoveRestoresHoverAfterInterruption()
    {
        QVERIFY(chooseTool(*m_window, "Rect"));
        drawStroke(*m_window, m_inside - QPoint(90, 60), m_inside + QPoint(90, 60));
        QVERIFY(chooseTool(*m_window, "Select"));
        pointerEvent(*m_window, QEvent::MouseMove, m_inside);
        QCOMPARE(m_window->cursor().shape(), Qt::OpenHandCursor);
        const QImage before = m_window->grab().toImage();
        pointerEvent(*m_window, QEvent::MouseMove, m_inside + QPoint(10, 10));
        QCOMPARE(m_window->grab().toImage(), before);
        pointerEvent(*m_window, QEvent::MouseButtonPress, m_inside, Qt::LeftButton, Qt::LeftButton);
        QCOMPARE(m_window->cursor().shape(), Qt::ClosedHandCursor);
        const QPoint moved = m_inside + QPoint(40, 20);
        pointerEvent(*m_window, QEvent::MouseMove, moved, Qt::NoButton, Qt::LeftButton);
        QCOMPARE(m_window->cursor().shape(), Qt::ClosedHandCursor);
        const QImage movedImage = m_window->grab().toImage();
        pointerEvent(*m_window, QEvent::MouseMove, moved + QPoint(10, 10));
        QCOMPARE(m_window->cursor().shape(), Qt::OpenHandCursor);
        QCOMPARE(m_window->grab().toImage(), movedImage);
        pointerEvent(*m_window, QEvent::MouseMove, m_inside - QPoint(200, 0));
        QCOMPARE(m_window->cursor().shape(), Qt::ArrowCursor);
    }

    /// @brief 列出矩形各方向的缩放目标，无参数和返回值
    void annotationResizeKeepsDirection_data()
    {
        QTest::addColumn<QPoint>("offset");
        QTest::addColumn<int>("cursorShape");
        QTest::newRow("left") << QPoint(-90, 0) << int(Qt::SizeHorCursor);
        QTest::newRow("right") << QPoint(90, 0) << int(Qt::SizeHorCursor);
        QTest::newRow("top") << QPoint(0, -60) << int(Qt::SizeVerCursor);
        QTest::newRow("bottom") << QPoint(0, 60) << int(Qt::SizeVerCursor);
        QTest::newRow("top-left") << QPoint(-90, -60) << int(Qt::SizeFDiagCursor);
        QTest::newRow("bottom-right") << QPoint(90, 60) << int(Qt::SizeFDiagCursor);
        QTest::newRow("top-right") << QPoint(90, -60) << int(Qt::SizeBDiagCursor);
        QTest::newRow("bottom-left") << QPoint(-90, 60) << int(Qt::SizeBDiagCursor);
    }

    /// @brief 验证边框缩放从悬停到释放保持方向，离开标注后恢复箭头，无参数和返回值
    void annotationResizeKeepsDirection()
    {
        QFETCH(QPoint, offset);
        QFETCH(int, cursorShape);
        QVERIFY(chooseTool(*m_window, "Rect"));
        drawStroke(*m_window, m_inside - QPoint(90, 60), m_inside + QPoint(90, 60));
        QVERIFY(chooseTool(*m_window, "Select"));
        drawStroke(*m_window, m_inside, m_inside);
        const QPoint handle = m_inside + offset;
        pointerEvent(*m_window, QEvent::MouseMove, handle);
        QCOMPARE(int(m_window->cursor().shape()), cursorShape);
        pointerEvent(*m_window, QEvent::MouseButtonPress, handle, Qt::LeftButton, Qt::LeftButton);
        QCOMPARE(int(m_window->cursor().shape()), cursorShape);
        pointerEvent(*m_window, QEvent::MouseMove, handle + offset / 3, Qt::NoButton, Qt::LeftButton);
        QCOMPARE(int(m_window->cursor().shape()), cursorShape);
        pointerEvent(*m_window, QEvent::MouseButtonRelease, handle + offset / 3, Qt::LeftButton);
        QCOMPARE(int(m_window->cursor().shape()), cursorShape);
        pointerEvent(*m_window, QEvent::MouseMove, m_inside - QPoint(200, 0));
        QCOMPARE(m_window->cursor().shape(), Qt::ArrowCursor);
    }

    /// @brief 验证标注旋转后缩放光标跟随画面方向，旋转操作保留抓取语义，无参数和返回值
    void rotatedAnnotationResizeFollowsVisibleDirection()
    {
        QVERIFY(chooseTool(*m_window, "Rect"));
        drawStroke(*m_window, m_inside - QPoint(90, 60), m_inside + QPoint(90, 60));
        QVERIFY(chooseTool(*m_window, "Select"));
        drawStroke(*m_window, m_inside, m_inside);
        const QPoint rotationHandle = m_inside - QPoint(0, 86);
        pointerEvent(*m_window, QEvent::MouseMove, rotationHandle);
        QCOMPARE(m_window->cursor().shape(), Qt::OpenHandCursor);
        pointerEvent(*m_window, QEvent::MouseButtonPress, rotationHandle, Qt::LeftButton, Qt::LeftButton);
        QCOMPARE(m_window->cursor().shape(), Qt::ClosedHandCursor);
        const QPoint rotatedHandle = m_inside + QPoint(86, 0);
        pointerEvent(*m_window, QEvent::MouseMove, rotatedHandle, Qt::NoButton, Qt::LeftButton);
        pointerEvent(*m_window, QEvent::MouseButtonRelease, rotatedHandle, Qt::LeftButton);
        QCOMPARE(m_window->cursor().shape(), Qt::OpenHandCursor);
        const QPoint visibleTop = m_inside - QPoint(0, 90);
        pointerEvent(*m_window, QEvent::MouseMove, visibleTop);
        QCOMPARE(m_window->cursor().shape(), Qt::SizeVerCursor);
        pointerEvent(*m_window, QEvent::MouseButtonPress, visibleTop, Qt::LeftButton, Qt::LeftButton);
        QCOMPARE(m_window->cursor().shape(), Qt::SizeVerCursor);
        pointerEvent(*m_window, QEvent::MouseButtonRelease, visibleTop, Qt::LeftButton);
        const QPoint visibleRight = m_inside + QPoint(60, 0);
        pointerEvent(*m_window, QEvent::MouseMove, visibleRight);
        QCOMPARE(m_window->cursor().shape(), Qt::SizeHorCursor);
    }

    /// @brief 验证线段端点使用四向光标，调整结束后恢复悬停状态，无参数和返回值
    void lineControlUsesPositionCursor()
    {
        QVERIFY(chooseTool(*m_window, "Line"));
        const QPoint start = m_inside - QPoint(90, 40);
        const QPoint end = m_inside + QPoint(90, 40);
        drawStroke(*m_window, start, end);
        QVERIFY(chooseTool(*m_window, "Select"));
        drawStroke(*m_window, m_inside, m_inside);
        pointerEvent(*m_window, QEvent::MouseMove, end);
        QCOMPARE(m_window->cursor().shape(), Qt::SizeAllCursor);
        pointerEvent(*m_window, QEvent::MouseButtonPress, end, Qt::LeftButton, Qt::LeftButton);
        pointerEvent(*m_window, QEvent::MouseMove, end + QPoint(30, 20), Qt::NoButton, Qt::LeftButton);
        QCOMPARE(m_window->cursor().shape(), Qt::SizeAllCursor);
        pointerEvent(*m_window, QEvent::MouseButtonRelease, end + QPoint(30, 20), Qt::LeftButton);
        QCOMPARE(m_window->cursor().shape(), Qt::SizeAllCursor);
    }

    /// @brief 验证画布外删除按钮的手型与实际删除操作一致，无参数和返回值
    void deleteButtonOutsideImageRemainsClickable()
    {
        QVERIFY(chooseTool(*m_window, "Rect"));
        const QPoint topRight(m_inside.x() + 60, m_image.top() + 4);
        const QPoint bottomLeft = topRight + QPoint(-150, 100);
        drawStroke(*m_window, QPoint(bottomLeft.x(), topRight.y()), QPoint(topRight.x(), bottomLeft.y()));
        QVERIFY(chooseTool(*m_window, "Select"));
        const QPoint center = (topRight + bottomLeft) / 2;
        drawStroke(*m_window, center, center);
        QPoint deletePoint;
        for (int y = m_image.top() - 35; y < m_image.top() && deletePoint.isNull(); y += 3) {
            for (int x = topRight.x() + 1; x < topRight.x() + 35; x += 3) {
                pointerEvent(*m_window, QEvent::MouseMove, QPoint(x, y));
                if (m_window->cursor().shape() == Qt::PointingHandCursor) {
                    deletePoint = QPoint(x, y);
                    break;
                }
            }
        }
        QVERIFY2(!deletePoint.isNull(), "The painted delete button should be clickable beyond the image boundary");
        drawStroke(*m_window, deletePoint, deletePoint);
        pointerEvent(*m_window, QEvent::MouseMove, center);
        QCOMPARE(m_window->cursor().shape(), Qt::ArrowCursor);
    }

    /// @brief 验证工具按钮和握柄保留自身语义，拖动结束后返回画布光标，无参数和返回值
    void toolbarControlsKeepSemanticCursors()
    {
        QVERIFY(chooseTool(*m_window, "Pen"));
        QPushButton *button = toolButton(*m_window, "Pen");
        QVERIFY(button);
        QCOMPARE(button->cursor().shape(), Qt::PointingHandCursor);
        // The top-level overlay can receive the first motion after a toolbar
        // appears. It must still expose the cursor of the child under the
        // event-local pointer position.
        const QPoint buttonCenter = button->mapTo(m_window.get(), button->rect().center());
        pointerEvent(*m_window, QEvent::MouseMove, buttonCenter);
        QCOMPARE(m_window->cursor().shape(), Qt::PointingHandCursor);
        auto *grip = m_window->findChild<QWidget *>(QStringLiteral("toolbarGrip"));
        QVERIFY(grip && grip->isVisible());
        QEnterEvent enter(grip->rect().center(), grip->rect().center(), grip->mapToGlobal(grip->rect().center()));
        QCoreApplication::sendEvent(grip, &enter);
        QCOMPARE(grip->cursor().shape(), Qt::OpenHandCursor);
        QTest::mousePress(grip, Qt::LeftButton);
        QCOMPARE(grip->cursor().shape(), Qt::ClosedHandCursor);
        QTest::mouseRelease(grip, Qt::LeftButton);
        QCOMPARE(grip->cursor().shape(), Qt::OpenHandCursor);
        pointerEvent(*m_window, QEvent::MouseMove, m_inside);
        QCOMPARE(m_window->cursor().shape(), Qt::BitmapCursor);
    }

    /// @brief 验证文字编辑退出后恢复当前绘制工具光标，无参数和返回值
    void textEditorReturnsToDrawingCursor()
    {
        QVERIFY(chooseTool(*m_window, "Text"));
        pointerEvent(*m_window, QEvent::MouseMove, m_inside);
        QCOMPARE(m_window->cursor().shape(), Qt::IBeamCursor);
        pointerEvent(*m_window, QEvent::MouseButtonPress, m_inside, Qt::LeftButton, Qt::LeftButton);
        pointerEvent(*m_window, QEvent::MouseButtonRelease, m_inside, Qt::LeftButton);
        auto *editor = m_window->findChild<QTextEdit *>(QStringLiteral("textEditor"));
        QVERIFY(editor && editor->isVisible());
        QTest::keyClick(editor, Qt::Key_Escape);
        QVERIFY(!editor->isVisible());
        QVERIFY(chooseTool(*m_window, "Pen"));
        pointerEvent(*m_window, QEvent::MouseMove, m_inside);
        QCOMPARE(m_window->cursor().shape(), Qt::BitmapCursor);
    }

private:
    std::unique_ptr<ShotWindow> m_window;
    QRect m_image;
    QPoint m_inside;
};

/// @brief 在隔离配置目录运行真实窗口回归，参数来自测试入口，返回检查退出码
/// @param argc 参数数量
/// @param argv 参数内容
int main(int argc, char **argv)
{
    QTemporaryDir isolated;
    if (!isolated.isValid()) {
        return 1;
    }
    for (const auto &entry : {std::pair{"XDG_CONFIG_HOME", "config"},
                              std::pair{"XDG_CACHE_HOME", "cache"},
                              std::pair{"XDG_DATA_HOME", "data"}}) {
        const QString path = isolated.filePath(QLatin1String(entry.second));
        QDir().mkpath(path);
        qputenv(entry.first, path.toUtf8());
    }
    QApplication app(argc, argv);
    QApplication::setApplicationName(QStringLiteral("mark-shot-cursor-test"));
    markshot::ui::installInteractionCursorPolicy(&app);
    AnnotationCursorTest test;
    return QTest::qExec(&test, argc, argv);
}

#include "annotation_cursor_test.moc"
