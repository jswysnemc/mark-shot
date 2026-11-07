/** 【原图对照】【真实组件】只读链接原版 OcrSourcePreview，采集真实几何、指针和菜单 */
#include "app_config_store.h"
#include "support/isolated_config.h"
#include "ui/context_menu.h"
#include "ui/i18n.h"

#include <QApplication>
#include <QContextMenuEvent>
#include <QFrame>
#include <QImage>
#include <QJsonArray>
#include <QJsonDocument>
#include <QPointF>
#include <QTemporaryDir>
#include <QTimer>
#include <QWheelEvent>
#include <QtTest/QTest>
#include <iostream>

// 1. 【原图对照】【只读状态】仅在探针头文件中公开状态读取，不修改原版实现、对象或事件处理
#define private public
#include "ocr_result_window/ocr_source_preview.h"
#undef private

using Preview = markshot::shot::OcrSourcePreview;

/** 【原图对照】【几何记录】从 view 读取真实原图矩形、缩放、平移和光标，返回 JSON，不改变组件 */
static QJsonObject snapshot(const Preview &view)
{
    const QRect rect = view.imageRect();
    return {{"rect", QJsonArray{rect.x(), rect.y(), rect.width(), rect.height()}},
        {"zoom", view.m_zoom}, {"pan", QJsonArray{view.m_pan.x(), view.m_pan.y()}},
        {"dragging", view.m_dragging}, {"cursor", int(view.cursor().shape())}};
}

/** 【原图对照】【滚轮输入】给 view 发送 point 处的 angle 与 pixel 位移，无返回值 */
static void wheel(Preview &view, QPoint point, int angle, int pixel = 0)
{
    QWheelEvent event(point, view.mapToGlobal(point), QPoint(0, pixel), QPoint(0, angle),
        Qt::NoButton, Qt::NoModifier, Qt::NoScrollPhase, false);
    QApplication::sendEvent(&view, &event);
}

/** 【原图对照】【事件回放】按 name 向实际 view 发送事件，返回逐事件状态，不启动编辑或贴图 */
static QJsonObject scenario(const QString &name, QSize viewport, QSize imageSize)
{
    QImage image(imageSize, QImage::Format_ARGB32);
    image.fill(QColor("#d31d77"));
    Preview view(image);
    view.resize(viewport);
    view.show();
    QTest::qWait(20);
    QJsonArray events;
    const auto record = [&](QJsonObject event) {
        event["state"] = snapshot(view);
        events.append(event);
    };
    record({{"kind", "initial"}});
    if (name == "wheel" || name == "drag" || name == "limits" || name == "resize") {
        const int count = name == "limits" ? 30 : 3;
        for (int index = 0; index < count; ++index) {
            const QPoint point(103, 77);
            wheel(view, point, 120);
            record({{"kind", "wheel"}, {"point", QJsonArray{point.x(), point.y()}}, {"angle", 120}});
        }
    }
    if (name == "drag") {
        for (const auto button : {Qt::RightButton, Qt::MiddleButton, Qt::LeftButton}) {
            const QPoint point(103, 77);
            QTest::mousePress(&view, button, {}, point);
            record({{"kind", "press"}, {"button", int(button)}, {"point", QJsonArray{103, 77}}});
            if (button == Qt::LeftButton) {
                for (const QPoint destination : {QPoint(180, 120), QPoint(4000, 3000), QPoint(-4000, -3000)}) {
                    QTest::mouseMove(&view, destination);
                    record({{"kind", "move"}, {"point", QJsonArray{destination.x(), destination.y()}}});
                }
            }
            QTest::mouseRelease(&view, button, {}, point);
            record({{"kind", "release"}, {"button", int(button)}, {"point", QJsonArray{103, 77}}});
        }
        QTest::mouseDClick(&view, Qt::LeftButton, {}, QPoint(100, 100));
        record({{"kind", "double"}});
    }
    if (name == "wheel" || name == "limits") {
        const int count = name == "limits" ? 30 : 5;
        for (int index = 0; index < count; ++index) {
            wheel(view, QPoint(275, 161), -120);
            record({{"kind", "wheel"}, {"point", QJsonArray{275, 161}}, {"angle", -120}});
        }
    }
    if (name == "resize") {
        view.resize(380, 340);
        QTest::qWait(20);
        record({{"kind", "resize"}, {"size", QJsonArray{380, 340}}});
    }
    if (name == "pixel-only") {
        wheel(view, QPoint(100, 100), 0, 120);
        record({{"kind", "wheel"}, {"point", QJsonArray{100, 100}}, {"angle", 0}, {"pixel", 120}});
    }
    return {{"name", name}, {"viewport", QJsonArray{viewport.width(), viewport.height()}},
        {"image", QJsonArray{imageSize.width(), imageSize.height()}}, {"events", events}};
}

/** 【原图对照】【菜单采集】打开实际原图右键菜单，记录菜单行和几何后取消，返回 JSON */
static QJsonObject menu()
{
    QImage image(321, 153, QImage::Format_ARGB32);
    image.fill(Qt::darkCyan);
    Preview view(image);
    view.resize(500, 300);
    view.show();
    QJsonObject result;
    QTimer::singleShot(50, [&] {
        auto *menu = qobject_cast<markshot::ui::ContextMenu *>(QApplication::activePopupWidget());
        if (!menu) return;
        QJsonArray rows;
        for (QAction *action : menu->actions()) {
            const QRect rect = menu->actionGeometry(action);
            rows.append(QJsonObject{{"label", action->text()}, {"enabled", action->isEnabled()},
                {"rect", QJsonArray{rect.x(), rect.y(), rect.width(), rect.height()}}});
        }
        result = {{"size", QJsonArray{menu->width(), menu->height()}}, {"rows", rows}};
        menu->dismiss();
    });
    QContextMenuEvent event(QContextMenuEvent::Mouse, QPoint(100, 100), view.mapToGlobal(QPoint(100, 100)));
    QApplication::sendEvent(&view, &event);
    return result;
}

/** 【原图对照】【入口】argc、argv 为程序参数，输出真实组件证据；不连接用户配置或座席 */
int main(int argc, char **argv)
{
    QTemporaryDir home;
    if (!markshot::test::isolateConfig(home)) return 2;
    for (const char *name : {"DISPLAY", "WAYLAND_DISPLAY", "NIRI_SOCKET", "SWAYSOCK", "XDG_SESSION_TYPE", "DBUS_SESSION_BUS_ADDRESS"}) qunsetenv(name);
    QApplication app(argc, argv);
    markshot::i18n::setLanguage(markshot::i18n::Language::English);
    if (!markshot::writeAppConfigRoot({{"ui", QJsonObject{{"theme", "dark"}}}})) return 3;
    QJsonArray scenarios;
    for (const QString &name : {"wheel", "drag", "limits", "resize", "pixel-only", "fit"}) {
        scenarios.append(scenario(name, QSize(500, 300), QSize(321, 153)));
    }
    scenarios.append(scenario("portrait", QSize(383, 341), QSize(153, 321)));
    scenarios.append(scenario("tiny", QSize(11, 13), QSize(321, 153)));
    std::cout << QJsonDocument(QJsonObject{{"qt_version", QT_VERSION_STR},
        {"component", "original OcrSourcePreview"}, {"scenarios", scenarios}, {"menu", menu()}}).toJson().toStdString();
}
