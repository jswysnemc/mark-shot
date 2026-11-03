#include "gui_memory_scenarios.h"

#include "pinned_window/pinned_image_window.h"
#include "region_detection/region_detection_scheduler.h"
#include "settings/settings_dialog.h"
#include "shot_window.h"
#include "ui/i18n.h"

#include <QApplication>
#include <QElapsedTimer>
#include <QMouseEvent>
#include <QPainter>
#include <QPointer>
#include <QPushButton>
#include <QScrollBar>
#include <QStackedWidget>
#include <QThreadPool>
#include <QWheelEvent>
#include <QtTest/QTest>

#include <algorithm>
#include <cmath>

namespace markshot::test {
namespace {

/** 创建尺寸为 size 的固定界面图案，返回不透明原始图像，不访问真实桌面。 */
QImage testImage(QSize size)
{
    QImage image(size, QImage::Format_RGB32);
    image.fill(QColor(225, 230, 237));
    QPainter painter(&image);
    painter.fillRect(0, 0, size.width(), 60, QColor(35, 45, 60));
    for (int y = 100; y < size.height(); y += 150) {
        for (int x = 40; x < size.width(); x += 260) {
            painter.fillRect(x, y, 220, 110, Qt::white);
            painter.fillRect(x + 12, y + 16, 120, 8, QColor(60, 80, 105));
            painter.fillRect(x + 12, y + 42, 190, 6, QColor(170, 180, 195));
        }
    }
    return image;
}

/** 在 window 首次绘制后等待区域检测及其完成通知，失败写入 error，返回是否稳定。 */
bool settle(QWidget *window, QString *error)
{
    if (window && !QTest::qWaitForWindowExposed(window)) {
        *error = QStringLiteral("Window was not exposed");
        return false;
    }
    // 1. 【GUI测量】【任务等待】waitForDone 会清空线程池，不能用它模拟正常窗口间的空闲期
    QElapsedTimer timer;
    timer.start();
    while (QThreadPool::globalInstance()->activeThreadCount() > 0 || region_detection::regionDetectionBusy()) {
        if (timer.elapsed() >= 15000) {
            *error = QStringLiteral("Background image work did not finish");
            return false;
        }
        QTest::qWait(2);
    }
    QTest::qWait(80);
    return true;
}

/** 关闭实际生产窗口并等待延迟销毁，失败写入 error，返回是否释放；不整理工作集。 */
bool closeWindow(QWidget *window, QString *error)
{
    if (!window) return true;
    QPointer<QWidget> guard(window);
    window->close();
    for (int attempt = 0; guard && attempt < 100; ++attempt) QTest::qWait(10);
    if (guard) {
        *error = QStringLiteral("Closed window was retained");
        return false;
    }
    return settle(nullptr, error);
}

/** 向 window 的 point 发送完整鼠标事件，type/button/buttons 描述按键状态，无返回值。 */
void pointerEvent(QWidget *window, QEvent::Type type, QPoint point,
                   Qt::MouseButton button, Qt::MouseButtons buttons)
{
    QMouseEvent event(type, QPointF(point), QPointF(window->mapToGlobal(point)),
                      button, buttons, Qt::NoModifier);
    QCoreApplication::sendEvent(window, &event);
}

/** 通过实际工具按钮选择 name，找不到时返回 false。 */
bool chooseTool(ShotWindow *window, const char *name)
{
    for (QPushButton *button : window->findChildren<QPushButton *>()) {
        if (button->property("action").toString() == QLatin1String(name)) {
            button->click();
            return true;
        }
    }
    return false;
}

/** 在 window 绘制 count 条笔画并提交历史，失败写入 error，返回是否成功。 */
bool annotate(ShotWindow *window, int count, QString *error)
{
    if (!chooseTool(window, "Pen")) {
        *error = QStringLiteral("Pen tool is unavailable");
        return false;
    }
    for (int stroke = 0; stroke < count; ++stroke) {
        const QPoint start(window->width() / 4, window->height() / 3 + stroke * 4);
        pointerEvent(window, QEvent::MouseButtonPress, start, Qt::LeftButton, Qt::LeftButton);
        QPoint point = start;
        for (int step = 1; step <= 24; ++step) {
            point = start + QPoint(step * window->width() / 60, (step % 5) * 3);
            pointerEvent(window, QEvent::MouseMove, point, Qt::NoButton, Qt::LeftButton);
        }
        pointerEvent(window, QEvent::MouseButtonRelease, point, Qt::LeftButton, Qt::NoButton);
        QTest::qWait(5);
    }
    return settle(window, error);
}

/** 向 window 发送 steps 个滚轮刻度，modifiers 决定图片缩放手势，无返回值。 */
void wheel(QWidget *window, qreal steps, Qt::KeyboardModifiers modifiers)
{
    const QPoint center = window->rect().center();
    QWheelEvent event(QPointF(center), QPointF(window->mapToGlobal(center)), QPoint(),
                      QPoint(0, qRound(steps * 120)), Qt::NoButton, modifiers,
                      Qt::NoScrollPhase, false);
    QCoreApplication::sendEvent(window, &event);
    QTest::qWait(50);
}

/** 对 frameSize 源图测量 scenario 截图或编辑流程；viewport/strokes/zoomPasses 控制工作量，checkpoint 采样，失败写入 error。 */
bool captureScenario(const QString &scenario, QSize frameSize, QSize viewport, int strokes, int zoomPasses,
                      const GuiMemoryCheckpoint &checkpoint, QString *error)
{
    QPointer<ShotWindow> window = new ShotWindow(testImage(frameSize), QString(),
                                                QRect(QPoint(), viewport));
    window->resize(viewport);
    if (scenario != QStringLiteral("capture")) {
        window->setImageNavigationEnabled(scenario == QStringLiteral("edit"));
        window->startFullscreenAnnotation();
    }
    checkpoint(QStringLiteral("created"));
    window->show();
    if (!settle(window, error)) return false;
    checkpoint(QStringLiteral("shown"));
    if (scenario == QStringLiteral("font-panel")) {
        if (!chooseTool(window, "Text")) {
            *error = QStringLiteral("Text tool is unavailable");
            return false;
        }
        if (!settle(window, error)) return false;
        checkpoint(QStringLiteral("text_tool"));
        QPushButton *fontButton = nullptr;
        for (auto *button : window->findChildren<QPushButton *>()) {
            if (button->accessibleName() == MS_TR("Text font")) fontButton = button;
        }
        if (!fontButton) {
            *error = QStringLiteral("Font button is unavailable");
            return false;
        }
        for (const QString &phase : {QStringLiteral("font_opened"), QStringLiteral("font_hidden"),
                                     QStringLiteral("font_reopened")}) {
            fontButton->click();
            if (!settle(window, error)) return false;
            const auto *panel = window->findChild<QWidget *>(QStringLiteral("propertyFontPanel"));
            if (!panel || panel->isVisible() != (phase != QStringLiteral("font_hidden"))) {
                *error = QStringLiteral("Font panel visibility did not change");
                return false;
            }
            checkpoint(phase);
        }
    }
    if (scenario == QStringLiteral("annotate")) {
        if (!annotate(window, strokes, error)) return false;
        checkpoint(QStringLiteral("annotated"));
        for (int i = 0; i < std::min(strokes, 10); ++i) QTest::keyClick(window, Qt::Key_Z, Qt::ControlModifier);
        checkpoint(QStringLiteral("undo"));
        for (int i = 0; i < std::min(strokes, 10); ++i) QTest::keyClick(window, Qt::Key_Y, Qt::ControlModifier);
        if (!settle(window, error)) return false;
        checkpoint(QStringLiteral("redo"));
    } else if (scenario == QStringLiteral("edit")) {
        wheel(window, 5, Qt::ControlModifier);
        if (!settle(window, error)) return false;
        bool zoomed = false;
        for (QScrollBar *bar : window->findChildren<QScrollBar *>()) zoomed |= bar->maximum() > 0;
        if (!zoomed) {
            *error = QStringLiteral("Image zoom gesture did not change the viewport");
            return false;
        }
        checkpoint(QStringLiteral("zoomed"));
        for (int pass = 1; pass < zoomPasses; ++pass) {
            wheel(window, 1, Qt::ControlModifier);
            if (!settle(window, error)) return false;
            checkpoint(QStringLiteral("zoomed_%1").arg(pass));
        }
        wheel(window, -(4 + zoomPasses), Qt::ControlModifier);
        if (!settle(window, error)) return false;
        checkpoint(QStringLiteral("zoom_restored"));
    }
    if (!closeWindow(window, error)) return false;
    checkpoint(QStringLiteral("closed"));
    return true;
}

/** 测量 frameSize 图像的贴图及可选编辑流程；viewport/sourceDpr 指定尺寸和原图比例，checkpoint 采样，失败写入 error，返回是否成功。 */
bool pinScenario(QSize frameSize, QSize viewport, bool edit, qreal sourceDpr,
                  const GuiMemoryCheckpoint &checkpoint, QString *error)
{
    QImage image = testImage(frameSize);
    image.setDevicePixelRatio(sourceDpr);
    QPointer<shot::PinnedImageWindow> window = new shot::PinnedImageWindow(std::move(image));
    window->show();
    if (!settle(window, error)) return false;
    // 1. 【GUI测量】【贴图缩放】通过实际滚轮调整显示比例，不更改窗口内部缩放状态
    const qreal ratio = static_cast<qreal>(viewport.width()) / std::max(1, window->width());
    wheel(window, std::log(ratio) / std::log(1.08), Qt::NoModifier);
    if (!settle(window, error)) return false;
    checkpoint(QStringLiteral("pinned"));
    if (edit) {
        QPointer<ShotWindow> editor = window->openImageEditor();
        if (!editor) {
            *error = QStringLiteral("Pinned image editor did not open");
            return false;
        }
        editor->resize(viewport);
        if (!settle(editor, error)) return false;
        checkpoint(QStringLiteral("pin_editor"));
        if (!closeWindow(editor, error)) return false;
        checkpoint(QStringLiteral("editor_closed"));
    }
    if (!closeWindow(window, error)) return false;
    checkpoint(QStringLiteral("closed"));
    return true;
}

/** 打开实际设置入口并逐页访问，checkpoint 采样，失败写入 error，返回是否全部完成。 */
bool settingsScenario(const GuiMemoryCheckpoint &checkpoint, QString *error)
{
    settings::showSettingsDialog();
    QPointer<QWidget> dialog;
    for (QWidget *candidate : QApplication::topLevelWidgets()) {
        if (candidate->objectName() == QStringLiteral("settingsDialog")) dialog = candidate;
    }
    if (!dialog || !settle(dialog, error)) return false;
    checkpoint(QStringLiteral("settings_opened"));
    QStackedWidget *stack = dialog->findChild<QStackedWidget *>();
    if (!stack) {
        *error = QStringLiteral("Settings page stack is unavailable");
        return false;
    }
    for (int index = 0; index < stack->count(); ++index) {
        stack->setCurrentIndex(index);
        if (!settle(dialog, error)) return false;
        checkpoint(QStringLiteral("settings_page_%1").arg(index));
    }
    if (!closeWindow(dialog, error)) return false;
    checkpoint(QStringLiteral("closed"));
    return true;
}

}  // namespace

bool runGuiMemoryScenario(const QString &scenario, QSize frameSize, QSize viewport, int strokes, int zoomPasses,
                          const GuiMemoryCheckpoint &checkpoint, QString *error)
{
    if (scenario == QStringLiteral("settings")) return settingsScenario(checkpoint, error);
    if (scenario == QStringLiteral("pin") || scenario == QStringLiteral("pin-edit")
        || scenario == QStringLiteral("pin-edit-hidpi")) {
        return pinScenario(frameSize, viewport, scenario != QStringLiteral("pin"),
                           scenario == QStringLiteral("pin-edit-hidpi") ? 2.0 : 1.0, checkpoint, error);
    }
    return captureScenario(scenario, frameSize, viewport, strokes, zoomPasses, checkpoint, error);
}

}  // namespace markshot::test
