// 【画布导航对照】【实际采集】调用完整生产函数和原生QScrollBar，不用重写端算法生成预期
#include "probe.h"
#include "production.inc"
#include "ui/canvas_scrollbar_style.h"
#include <QFile>
#include <QFileInfo>
#include <QJsonDocument>
#include <QStyleOptionSlider>

using namespace markshot::shot;
QJsonArray styleSamples(const QString &root);

/** 【画布导航对照】【点编码】按value返回原始双精度坐标 */
QJsonArray point(QPointF value) { return {value.x(), value.y()}; }
/** 【画布导航对照】【矩形编码】按value返回左、上、宽、高 */
QJsonArray rect(QRectF value) { return {value.x(), value.y(), value.width(), value.height()}; }
/** 【画布导航对照】【轴编码】按bar返回原生几何、范围、步长和实际显隐 */
QJsonObject axis(QScrollBar *bar) {
    return {{"rect", rect(bar->geometry())}, {"maximum", bar->maximum()}, {"page_step", bar->pageStep()}, {"single_step", bar->singleStep()}, {"value", bar->value()}, {"visible", !bar->isHidden()}};
}
/** 【画布导航对照】【视图编码】按window返回完整当前状态、原图和窗口投影，以及两轴原生结果 */
QJsonObject view(ShotWindow &window) {
    const QPointF probe(173.25, -29.75);
    const auto mapped = window.widgetToImage(probe);
    return {{"zoom", window.m_imageZoom}, {"center", point(window.m_imageCenter)}, {"initialized", window.m_imageCenterInitialized}, {"rect", rect(window.m_frozenImageRect)}, {"size", QJsonArray{window.width(), window.height()}}, {"probe", point(probe)}, {"image_probe", point(mapped)}, {"widget_probe", point(window.imageToWidget(mapped))}, {"horizontal", axis(window.m_controls.m_horizontalImageScrollBar)}, {"vertical", axis(window.m_controls.m_verticalImageScrollBar)}};
}
/** 【画布导航对照】【指针投递】按pointer设置私有offscreen宿主的原生整点指针，并按dragging设置滚动条抓取，无返回值 */
void pointerAt(ShotWindow &window, QPointF pointer, bool horizontal = false, bool vertical = false) {
    QCursor::setPos(window.mapToGlobal(pointer.toPoint()));
    window.m_controls.m_horizontalImageScrollBar->setSliderDown(horizontal);
    window.m_controls.m_verticalImageScrollBar->setSliderDown(vertical);
    window.updateImageScrollBars();
}
/** 【画布导航对照】【几何采集】返回多种原图、窗口、锚点和连续变换的生产数据，无参数 */
QJsonArray geometrySamples() {
    QJsonArray result;
    const QSize images[] = {QSize(640, 480), QSize(400, 1800), QSize(1800, 400), QSize(1, 1), QSize(8192, 128), QSize(853, 533), QSize(32, 20000)};
    const QSize windows[] = {QSize(640, 480), QSize(853, 533), QSize(960, 720), QSize(1000, 700)};
    for (auto image : images) for (auto size : windows) {
        ShotWindow window;
        window.resize(size); window.show(); QApplication::processEvents();
        window.m_frozenFrame = QImage(image, QImage::Format_ARGB32_Premultiplied);
        window.updateFrozenImageRect(); pointerAt(window, QPointF(100, 100));
        const auto initial = view(window);
        QJsonArray steps;
        // 1. 【画布导航对照】【连续求值】真实函数从前一状态继续，覆盖图外锚点、上下限、反向平移和窗口变化
        for (auto value : {std::pair<qreal, QPointF>{1.12, QPointF(137.25, 91.5)}, {4, QPointF(20, 20)}, {0.5, QPointF(-80, 900)}, {1e6, QPointF(size.width(), size.height())}, {1e-6, QPointF(0, 0)}, {8, QPointF(size.width() / 2.0, size.height() / 2.0)}}) {
            window.zoomImageAt(value.first, value.second);
            pointerAt(window, QPointF(size.width() - 32, size.height() - 32));
            steps.append(QJsonObject{{"action", "zoom"}, {"factor", value.first}, {"anchor", point(value.second)}, {"pointer", point(QPointF(size.width() - 32, size.height() - 32))}, {"after", view(window)}});
        }
        const auto start = QPointF(200.25, 180.75);
        window.m_imagePanStartWidget = start;
        window.m_imagePanStartCenter = window.m_imageCenter;
        const auto startCenter = window.m_imageCenter;
        for (auto current : {QPointF(213.625, 192.375), QPointF(3000, -2000), QPointF(-3000, 2000), start}) {
            window.panImageTo(current);
            pointerAt(window, QPointF(size.width() + 1, size.height() + 1), true, false);
            steps.append(QJsonObject{{"action", "pan"}, {"start", point(start)}, {"center", point(startCenter)}, {"current", point(current)}, {"pointer", point(QPointF(size.width() + 1, size.height() + 1))}, {"dragging", QJsonArray{true, false}}, {"after", view(window)}});
        }
        // 2. 【画布导航对照】【整数滚动】采用实际范围的三分之一和三分之二位置，生产函数读回原生滚动条
        const int h = window.m_controls.m_horizontalImageScrollBar->maximum() / 3;
        const int v = window.m_controls.m_verticalImageScrollBar->maximum() * 2 / 3;
        window.m_controls.m_horizontalImageScrollBar->setValue(h);
        window.m_controls.m_verticalImageScrollBar->setValue(v);
        window.setImageCenterFromScrollBars(); pointerAt(window, QPointF(-1, -1));
        steps.append(QJsonObject{{"action", "scroll"}, {"values", QJsonArray{h, v}}, {"pointer", point(QPointF(-1, -1))}, {"after", view(window)}});
        window.m_mode = ShotWindow::Mode::Selecting;
        window.updateFrozenImageRect(); pointerAt(window, QPointF(100, 100));
        steps.append(QJsonObject{{"action", "availability"}, {"available", false}, {"pointer", point(QPointF(100, 100))}, {"after", view(window)}});
        window.m_mode = ShotWindow::Mode::Editing;
        window.updateFrozenImageRect(); pointerAt(window, QPointF(100, 100));
        steps.append(QJsonObject{{"action", "availability"}, {"available", true}, {"pointer", point(QPointF(100, 100))}, {"after", view(window)}});
        window.resize(701, 509); window.updateFrozenImageRect(); pointerAt(window, QPointF(700, 508));
        steps.append(QJsonObject{{"action", "resize"}, {"size", QJsonArray{701, 509}}, {"pointer", point(QPointF(700, 508))}, {"after", view(window)}});
        window.resetImageZoom(); pointerAt(window, QPointF(700, 508));
        steps.append(QJsonObject{{"action", "reset"}, {"pointer", point(QPointF(700, 508))}, {"after", view(window)}});
        result.append(QJsonObject{{"image", QJsonArray{image.width(), image.height()}}, {"size", QJsonArray{size.width(), size.height()}}, {"initial", initial}, {"steps", steps}});
    }
    return result;
}

/** 【画布导航对照】【滚轮采集】所有分派使用完整wheelEvent，记录真实缩放或宽度setter调用，不执行其他业务 */
QJsonArray wheelSamples() {
    const std::pair<ShotWindow::Tool, const char *> tools[] = {{ShotWindow::Tool::Move, "selection"}, {ShotWindow::Tool::Select, "pick"}, {ShotWindow::Tool::Pen, "pen"}, {ShotWindow::Tool::Line, "line"}, {ShotWindow::Tool::Highlighter, "highlighter"}, {ShotWindow::Tool::Rectangle, "rectangle"}, {ShotWindow::Tool::Ellipse, "circle"}, {ShotWindow::Tool::Marker, "marker"}, {ShotWindow::Tool::Arrow, "arrow"}, {ShotWindow::Tool::Magnifier, "magnifier"}, {ShotWindow::Tool::Laser, "laser"}, {ShotWindow::Tool::Ruler, "ruler"}, {ShotWindow::Tool::Text, "text"}, {ShotWindow::Tool::Mosaic, "redact"}, {ShotWindow::Tool::Number, "number"}};
    QJsonArray result;
    for (bool enabled : {false, true}) for (bool editing : {false, true}) for (auto tool : tools) for (int selected : {0, 1, 2}) for (bool blocked : {false, true}) for (bool existing : {false, true}) {
        ShotWindow window;
        window.resize(640, 480); window.show(); QApplication::processEvents();
        window.m_frozenFrame = QImage(640, 480, QImage::Format_ARGB32_Premultiplied);
        window.m_imageNavigationEnabled = enabled; window.m_mode = editing ? ShotWindow::Mode::Editing : ShotWindow::Mode::Selecting;
        window.m_tool = tool.first; window.m_dragging = blocked;
        for (int i = 0; i < selected; ++i) { window.ids.append(7 + i); window.members.append({7 + i, existing ? ShotWindow::Tool::Text : tool.first, 3}); }
        if (existing) { window.m_textEditor->show(); window.m_editingTextAnnotationId = 7; }
        window.updateFrozenImageRect(); pointerAt(window, QPointF(200, 200));
        QWheelEvent event(QPointF(200, 200), window.mapToGlobal(QPoint(200, 200)), {}, QPoint(0, 120), Qt::NoButton, Qt::NoModifier, Qt::NoScrollPhase, false);
        window.wheelEvent(&event);
        result.append(QJsonObject{{"enabled", enabled}, {"editing", editing}, {"tool", tool.second}, {"selected", selected}, {"editing_existing", existing && selected > 0}, {"blocked", blocked}, {"route", window.wheelRoute}, {"available", window.imageNavigationAvailable()}, {"wheel_zooms", window.wheelZoomsImage()}, {"zoom", window.m_imageZoom}, {"history", window.historyCaptured}, {"requested", window.widthRequested}});
    }
    return result;
}

/** 【画布导航对照】【倍率采集】按生产imageNavigationWheelFactor覆盖角度、纯像素、优先级与水平轴，无参数 */
QJsonArray factorSamples() {
    QJsonArray result;
    for (int angle : {-1200, -120, -1, 0, 1, 60, 120, 1200}) for (int pixel : {-160, -8, 0, 1, 8, 80, 160}) {
        QWheelEvent event({}, {}, QPoint(13, pixel), QPoint(120, angle), Qt::NoButton, Qt::NoModifier, Qt::NoScrollPhase, false);
        result.append(QJsonObject{{"angle", angle}, {"pixel", pixel}, {"factor", imageNavigationWheelFactor(&event)}});
    }
    return result;
}

/** 【画布导航对照】【入口】argc与argv指定输出文件，成功写入全部实际结果时返回零 */
int main(int argc, char **argv) {
    QApplication app(argc, argv);
    if (argc != 2) return 2;
    QFile output(QString::fromLocal8Bit(argv[1]));
    if (!output.open(QIODevice::WriteOnly)) return 3;
    output.write(QJsonDocument(QJsonObject{{"qt", QT_VERSION_STR}, {"geometry", geometrySamples()}, {"wheel", wheelSamples()}, {"factor", factorSamples()}, {"styles", styleSamples(QFileInfo(output).absolutePath())}}).toJson());
    return 0;
}
