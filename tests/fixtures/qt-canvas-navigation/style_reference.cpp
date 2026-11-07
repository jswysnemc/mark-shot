// 【画布导航对照】【原生样式】编译完整生产样式，记录真实滚动条控制范围、状态和小型像素图
#include "ui/canvas_scrollbar_style.h"
#include <QApplication>
#include <QCryptographicHash>
#include <QDir>
#include <QEnterEvent>
#include <QFile>
#include <QJsonArray>
#include <QJsonObject>
#include <QPixmap>
#include <QScrollBar>
#include <QStyle>
#include <QStyleOptionSlider>
#include <stdexcept>

class NativeBar : public QScrollBar {
public:
    using QScrollBar::QScrollBar;
    /** 【画布导航对照】【样式状态】返回真实控件内部样式参数，无参数 */
    QStyleOptionSlider options() const { QStyleOptionSlider value; initStyleOption(&value); return value; }
};

/** 【画布导航对照】【范围编码】按rect返回原生局部控制范围 */
QJsonArray styleRect(QRect rect) { return {rect.x(), rect.y(), rect.width(), rect.height()}; }

/** 【画布导航对照】【实际绘制】按root保存生产样式的原生PNG和实际几何，返回全部小型控件样本 */
QJsonArray styleSamples(const QString &root) {
    QJsonArray result;
    QDir().mkpath(root + QStringLiteral("/style-images"));
    int index = 0;
    for (auto orientation : {Qt::Horizontal, Qt::Vertical}) for (int length : {84, 300, 853}) for (int maximum : {100, 10000}) for (bool hover : {false, true}) for (bool pressed : {false, true}) for (bool dark : {false, true}) {
        QWidget parent;
        NativeBar bar(orientation, &parent);
        const QSize size = orientation == Qt::Horizontal ? QSize(length, 14) : QSize(14, length);
        parent.resize(size); bar.resize(size);
        markshot::ui::setupCanvasScrollBar(&bar, 14);
        bar.setRange(0, maximum); bar.setPageStep(640); bar.setSingleStep(53); bar.setValue(maximum / 3);
        parent.show(); bar.show(); QApplication::processEvents();
        // 1. 【画布导航对照】【悬停投递】进入事件交给完整生产事件过滤器，拖动状态由原生滑条维护
        if (hover) {
            QEnterEvent event(QPointF(7, 7), QPointF(7, 7), parent.mapToGlobal(QPoint(7, 7)));
            QCoreApplication::sendEvent(&bar, &event);
        } else {
            QEvent event(QEvent::Leave); QCoreApplication::sendEvent(&bar, &event);
        }
        bar.setSliderDown(pressed);
        const auto option = bar.options();
        const auto handle = bar.style()->subControlRect(QStyle::CC_ScrollBar, &option, QStyle::SC_ScrollBarSlider, &bar);
        const auto groove = bar.style()->subControlRect(QStyle::CC_ScrollBar, &option, QStyle::SC_ScrollBarGroove, &bar);
        // 2. 【画布导航对照】【原生像素】控件直接绘入实色背景，只记录实际绘制，不使用重写端光栅算法
        QPixmap pixels(size); pixels.fill(dark ? QColor(30, 34, 38) : QColor(238, 242, 246));
        bar.render(&pixels);
        const QString name = QStringLiteral("style-images/%1.png").arg(index++);
        if (!pixels.save(root + "/" + name, "PNG")) throw std::runtime_error("【画布导航对照】【原生像素】无法保存滚动条图像");
        QFile bytes(root + "/" + name); if (!bytes.open(QIODevice::ReadOnly)) throw std::runtime_error("【画布导航对照】【原生像素】无法读取滚动条图像");
        const auto hash = QCryptographicHash::hash(bytes.readAll(), QCryptographicHash::Sha256).toHex();
        result.append(QJsonObject{{"horizontal", orientation == Qt::Horizontal}, {"length", length}, {"maximum", maximum}, {"page_step", bar.pageStep()}, {"value", bar.value()}, {"hover", hover}, {"pressed", pressed}, {"dark", dark}, {"handle", styleRect(handle)}, {"groove", styleRect(groove)}, {"image", name}, {"sha256", QString::fromLatin1(hash)}, {"native_hover_property", bar.property("canvasHover").toBool()}, {"native_slider_down", bar.isSliderDown()}, {"native_under_mouse", bar.underMouse()}, {"native_option_state", static_cast<int>(option.state)}, {"native_active_controls", static_cast<int>(option.activeSubControls)}});
    }
    return result;
}
