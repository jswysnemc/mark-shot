#include <QApplication>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QFocusEvent>
#include <QFontInfo>
#include <QIntValidator>
#include <QImage>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QKeyEvent>
#include <QLineEdit>
#include <QMouseEvent>
#include <QPainter>
#include <QRawFont>
#include <QWheelEvent>
#include <QtMath>

#define private public
#include "selection_aspect/selection_size_panel.h"
#include "selection_aspect/selection_size_field.h"
#include "selection_aspect/selection_ratio_popup.h"
#undef private
#include "selection_aspect/selection_hud_layout.h"
#include "selection_aspect/selection_hud_painter.h"
#include "ui/i18n.h"
#include "ui/theme.h"

namespace markshot::i18n {
/** 【选区尺寸对照】【固定语言】按 source 返回英文标签，生产控件与几何直接链接原版 */
QString translate(const QString &source) { return source; }
}

namespace aspect = markshot::selection_aspect;

/** 【选区尺寸对照】【矩形编码】按 rect 返回整数坐标和尺寸 */
QJsonArray box(const QRect &rect) { return {rect.x(), rect.y(), rect.width(), rect.height()}; }

/** 【选区尺寸对照】【浮点矩形】按 rect 返回实际双精度坐标和尺寸 */
QJsonArray box(const QRectF &rect) { return {rect.x(), rect.y(), rect.width(), rect.height()}; }

/** 【选区尺寸对照】【尺寸编码】按 size 返回宽高像素 */
QJsonArray size(QSize value) { return {value.width(), value.height()}; }

/** 【选区尺寸对照】【颜色编码】按 value 返回未预乘的四个实际颜色通道 */
QJsonArray color(QColor value) { return {value.red(), value.green(), value.blue(), value.alpha()}; }

/** 【选区尺寸对照】【鼠标投递】向 widget 发送 type、button 和 point 的实际控件事件，不修改私有状态 */
void mouse(QWidget &widget, QEvent::Type type, Qt::MouseButton button, QPointF point)
{
    QMouseEvent event(type, point, widget.mapToGlobal(point.toPoint()), button,
                     type == QEvent::MouseButtonPress ? button : Qt::NoButton, Qt::NoModifier);
    QApplication::sendEvent(&widget, &event);
    QApplication::processEvents();
}

/** 【选区尺寸对照】【按键投递】向 widget 发送 key、text、mods 的实际按键，等待同步布局完成 */
void key(QWidget &widget, int value, QString text = {}, Qt::KeyboardModifiers mods = Qt::NoModifier)
{
    QKeyEvent event(QEvent::KeyPress, value, mods, text);
    QApplication::sendEvent(&widget, &event);
    QApplication::processEvents();
}

/** 【选区尺寸对照】【滚轮投递】向 widget 发送 pixel、angle 和 mods，返回空值 */
void wheel(QWidget &widget, QPoint pixel, QPoint angle, Qt::KeyboardModifiers mods = Qt::NoModifier)
{
    const QPointF point(10, 8);
    QWheelEvent event(point, widget.mapToGlobal(point.toPoint()), pixel, angle,
                     Qt::NoButton, mods, Qt::NoScrollPhase, false);
    QApplication::sendEvent(&widget, &event);
    QApplication::processEvents();
}

/** 【选区尺寸对照】【字段采样】读取 field 实际显示和编辑状态，不修改输入框 */
QJsonObject fieldState(const aspect::SelectionSizeField *field)
{
    const QLineEdit *editor = field->m_editor;
    return {{"value", field->value()}, {"editing", field->isEditing()}, {"text", editor->text()},
            {"cursor", editor->cursorPosition()}, {"selected", editor->selectedText()},
            {"selection_start", editor->selectionStart()}, {"acceptable", editor->hasAcceptableInput()},
            {"undo", editor->isUndoAvailable()}, {"redo", editor->isRedoAvailable()}};
}

/** 【选区尺寸对照】【面板采样】按 name 和 panel 记录字段、尺寸、比例及弹层状态，返回 JSON */
QJsonObject panelState(const QString &name, const aspect::SelectionSizePanel &panel)
{
    return {{"name", name}, {"size", size(panel.m_size)}, {"ratio", panel.m_ratio},
            {"width", fieldState(panel.m_widthField)}, {"height", fieldState(panel.m_heightField)},
            {"popup", panel.m_ratioPopup && panel.m_ratioPopup->isVisible()}};
}

/** 【选区尺寸对照】【真实绘制】按 widget、path 和 scale 保存生产控件透明像素，返回是否成功 */
bool render(QWidget &widget, const QString &path, qreal scale)
{
    QImage image(qCeil(widget.width() * scale), qCeil(widget.height() * scale), QImage::Format_ARGB32_Premultiplied);
    image.setDevicePixelRatio(scale);
    image.fill(Qt::transparent);
    QPainter painter(&image);
    widget.render(&painter, QPoint(), QRegion(), QWidget::DrawChildren);
    painter.end();
    return image.save(path);
}

/** 【选区尺寸对照】【实际采集】按 argv 输出生产字段、布局和几何样本，编译产物仅写入当前测试目录 */
int main(int argc, char **argv)
{
    QApplication app(argc, argv);
    app.setStyle("Fusion");
    QWidget host;
    host.resize(800, 600);
    host.show();
    aspect::SelectionSizePanel panel(&host);
    panel.setToolbarPalette(markshot::ui::toolbarPalette(markshot::ui::UiThemeMode::Light));
    panel.setValues(QSize(640, 480), QSize(1200, 800), 0.0);
    panel.move(20, 80);
    panel.show();
    app.processEvents();
    if (argc > 1 && QString::fromLocal8Bit(argv[1]) == QStringLiteral("--interactive")) {
        panel.focusWidth();
        return app.exec();
    }
    const QFont font = panel.font();
    QJsonObject output{{"qt", QT_VERSION_STR}, {"font", QFontInfo(font).family()},
                       {"point_size", font.pointSizeF()}, {"pixel_size", QRawFont::fromFont(font).pixelSize()},
                       {"weight", static_cast<int>(font.weight())}, {"dpi", host.logicalDpiX()},
                       {"hud_size", size(panel.sizeHint())}, {"field_size", size(panel.m_widthField->sizeHint())}};
    output["hud_rects"] = QJsonObject{{"width", box(panel.m_widthField->geometry())},
                                    {"height", box(panel.m_heightField->geometry())}, {"times", box(panel.m_timesRect)},
                                    {"unit", box(panel.m_unitRect)}, {"separator", box(panel.m_separatorRect)},
                                    {"lock", box(panel.m_lockRect)}, {"ratio", box(panel.m_ratioRect)}};
    output["input_hints"] = static_cast<int>(panel.m_widthField->m_editor->inputMethodHints());
    QJsonArray advances;
    for (const QString &text : {QString("00000"), QString("×"), QString("px"), QString("Custom"), QString("Free"), QString("21:9")}) {
        advances.append(QJsonObject{{"text", text}, {"width", QFontMetrics(font).horizontalAdvance(text)}});
    }
    output["advances"] = advances;
    QJsonObject palettes;
    for (bool dark : {false, true}) {
        const auto colors = aspect::hud_style::hudColors(markshot::ui::toolbarPalette(
            dark ? markshot::ui::UiThemeMode::Dark : markshot::ui::UiThemeMode::Light));
        palettes[dark ? "dark" : "light"] = QJsonObject{{"text", color(colors.text)}, {"muted", color(colors.muted)},
            {"hover", color(colors.hover)}, {"fill", color(colors.panelFill)}, {"border", color(colors.panelBorder)},
            {"separator", color(colors.separator)}, {"accent", color(colors.accent)}, {"editor_text", color(colors.editorText)},
            {"editor_fill", color(colors.editorFill)}, {"editor_border", color(colors.editorBorder)},
            {"editor_selection", color(colors.editorSelection)}};
    }
    output["palettes"] = palettes;

    // 1. 【选区尺寸对照】【整数字段验证】直接调用实际 Qt 验证器，覆盖中间值、上限、符号和数字脚本
    QJsonArray validation;
    for (int maximum : {1, 99, 1200}) {
        QIntValidator validator(1, maximum);
        validator.setLocale(QLocale::c());
        for (const QString &input : {QString(""), QString("0"), QString("1"), QString("+"), QString("+1"),
                                    QString("-"), QString("-1"), QString("99"), QString("100"), QString("1201"),
                                    QString("9999"), QString("10000"), QString("00000"), QString(" 12"),
                                    QString("1,200"), QString("١٢"), QString("１２"), QString("1a"),
                                    QString("1,"), QString("1,2"), QString("1,20"), QString("12,00"),
                                    QString("01201"), QString("00001"), QString("00,001"), QString("0,1"),
                                    QString("1,000,000"), QString("+0"), QString("++1")}) {
            QString text = input;
            int cursor = text.size();
            const QValidator::State state = validator.validate(text, cursor);
            validation.append(QJsonObject{{"maximum", maximum}, {"text", input}, {"state", static_cast<int>(state)}});
        }
    }
    output["validation"] = validation;

    // 2. 【选区尺寸对照】【实际字段事件】通过控件事件修改输入，接收尺寸信号后只同步实际应用值
    QJsonArray states, applied;
    QObject::connect(&panel, &aspect::SelectionSizePanel::sizeApplied, [&](QSize next) {
        applied.append(size(next));
        panel.setValues(next, QSize(1200, 800), panel.m_ratio);
    });
    QObject::connect(&panel, &aspect::SelectionSizePanel::aspectRatioChanged, [&](qreal ratio) {
        panel.setValues(panel.m_size, QSize(1200, 800), ratio);
    });
    const QString renderDirectory = QFileInfo(QString::fromLocal8Bit(argv[1])).absolutePath() + QStringLiteral("/render");
    QDir().mkpath(renderDirectory);
    const auto record = [&](const char *name) {
        states.append(panelState(QString::fromLatin1(name), panel));
        for (qreal scale : {1.0, 1.5}) {
            const QString suffix = QStringLiteral("-%1.png").arg(scale);
            render(panel, renderDirectory + QStringLiteral("/") + QString::fromLatin1(name) + suffix, scale);
            if (panel.m_ratioPopup && panel.m_ratioPopup->isVisible()) {
                render(*panel.m_ratioPopup, renderDirectory + QStringLiteral("/") + QString::fromLatin1(name) + QStringLiteral("-popup") + suffix, scale);
            }
        }
    };
    record("initial");
    mouse(*panel.m_widthField, QEvent::MouseButtonPress, Qt::RightButton, QPointF(10, 8));
    record("right_no_edit");
    panel.focusWidth();
    record("width_selected");
    key(*panel.m_widthField->m_editor, Qt::Key_Backspace);
    key(*panel.m_widthField->m_editor, 0, "1201");
    record("over_max_intermediate");
    key(*panel.m_widthField->m_editor, Qt::Key_Return);
    record("over_max_committed");
    panel.focusWidth();
    key(*panel.m_widthField->m_editor, Qt::Key_Backspace);
    key(*panel.m_widthField->m_editor, Qt::Key_Up, {}, Qt::ShiftModifier);
    record("arrow_pending");
    key(*panel.m_widthField->m_editor, Qt::Key_Escape);
    record("escape_discarded");
    panel.focusWidth();
    key(*panel.m_widthField->m_editor, Qt::Key_Backspace);
    record("empty_pending");
    wheel(*panel.m_widthField->m_editor, QPoint(0, -1), {}, Qt::NoModifier);
    record("wheel_empty_falls_back");
    key(*panel.m_widthField->m_editor, Qt::Key_Tab);
    record("tab_height");
    key(*panel.m_heightField->m_editor, Qt::Key_Backspace);
    key(*panel.m_heightField->m_editor, 0, "300");
    key(*panel.m_heightField->m_editor, Qt::Key_Backtab, {}, Qt::ShiftModifier);
    record("backtab_width");
    key(*panel.m_widthField->m_editor, Qt::Key_Escape);
    wheel(*panel.m_heightField, {}, QPoint(-120, 0), Qt::ShiftModifier);
    record("horizontal_wheel_shift");
    mouse(panel, QEvent::MouseButtonPress, Qt::RightButton, panel.m_lockRect.center());
    record("right_lock_ignored");
    mouse(panel, QEvent::MouseButtonPress, Qt::LeftButton, panel.m_lockRect.center());
    record("locked_current");
    mouse(panel, QEvent::MouseButtonPress, Qt::LeftButton, panel.m_ratioRect.center());
    record("ratio_popup");
    output["popup_size"] = size(panel.m_ratioPopup->sizeHint());
    output["popup_rect"] = box(panel.m_ratioPopup->geometry());
    mouse(*panel.m_ratioPopup, QEvent::MouseButtonRelease, Qt::RightButton, QPointF(20, 4 + 2 * 26 + 13));
    record("right_ratio_ignored");
    key(*panel.m_ratioPopup, Qt::Key_Up);
    key(*panel.m_ratioPopup, Qt::Key_Return);
    record("ratio_chosen");
    panel.focusWidth();
    key(*panel.m_widthField->m_editor, Qt::Key_Backspace);
    key(*panel.m_widthField->m_editor, 0, "1,200");
    record("grouped_pending");
    key(*panel.m_widthField->m_editor, Qt::Key_Return);
    record("grouped_not_committed");
    panel.focusWidth();
    key(*panel.m_widthField->m_editor, Qt::Key_Backspace);
    key(*panel.m_widthField->m_editor, 0, "+88");
    QFocusEvent focusOut(QEvent::FocusOut);
    QApplication::sendEvent(panel.m_widthField->m_editor, &focusOut);
    record("focus_out_committed");
    output["states"] = states;
    output["applied"] = applied;

    // 3. 【选区尺寸对照】【生产几何】链接当前原版 Rust 实现，通过 Qt 包装层计算浮点尺寸与比例约束
    QJsonArray creations, adjustments, linked, resized, presets;
    for (const auto &preset : aspect::aspectRatioPresets()) {
        const qreal ratio = preset.ratio;
        presets.append(QJsonObject{{"label", preset.label}, {"ratio", ratio}, {"index", aspect::presetIndexForRatio(ratio)}});
        for (const QPointF &pointer : {QPointF(30, 20), QPointF(300, 260), QPointF(780, 580), QPointF(-20, 900)}) {
            const QPointF anchor(100, 80);
            const QSize maximum(800, 600);
            creations.append(QJsonObject{{"anchor", QJsonArray{anchor.x(), anchor.y()}},
                {"pointer", QJsonArray{pointer.x(), pointer.y()}}, {"ratio", ratio}, {"maximum", size(maximum)},
                {"result", box(aspect::constrainedCreationRect(anchor, pointer, ratio, maximum))}});
        }
        for (int handle = 10; handle <= 17; ++handle) {
            const QRectF before(100, 100, 200, 120), changed(80, 70, 300, 240);
            const QSize maximum(800, 600);
            adjustments.append(QJsonObject{{"before", box(before)}, {"adjusted", box(changed)}, {"handle", handle},
                {"ratio", ratio}, {"maximum", size(maximum)},
                {"result", box(aspect::constrainedAdjustedRect(before, static_cast<markshot::shot::types::SelectionDrag>(handle), changed, ratio, maximum))}});
        }
        for (const QSize &request : {QSize(1, 1), QSize(900, 300), QSize(1200, 800), QSize(2000, 3000)}) {
            const QSize maximum(1200, 800);
            for (aspect::SizeField side : {aspect::SizeField::Width, aspect::SizeField::Height}) {
                linked.append(QJsonObject{{"requested", size(request)}, {"maximum", size(maximum)}, {"ratio", ratio},
                    {"width", side == aspect::SizeField::Width}, {"result", size(aspect::linkedSize(request, side, ratio, maximum))}});
            }
        }
    }
    for (const QRectF &current : {QRectF(20, 30, 300, 200), QRectF(700, 500, 100, 100), QRectF(20, 30, -10, -20)}) {
        for (const QSize &request : {QSize(20, 30), QSize(900, 900), QSize(0, -1)}) {
            const QSize maximum(800, 600);
            resized.append(QJsonObject{{"current", box(current)}, {"requested", size(request)}, {"maximum", size(maximum)},
                                      {"result", box(aspect::resizedSelection(current, request, maximum))}});
        }
    }
    output["presets"] = presets;
    output["creations"] = creations;
    output["adjustments"] = adjustments;
    output["linked"] = linked;
    output["resized"] = resized;

    // 5. 【选区尺寸对照】【物理裁切】生产尺寸修改后调用 Qt 的实际对齐规则，分数原点保留向外扩展
    QJsonArray exports;
    for (qreal scale : {1.0, 1.5, 1280.0 / 853.0}) {
        for (const QPointF &origin : {QPointF(0, 0), QPointF(20, 30), QPointF(120, 140)}) {
            const QRectF current(origin.x() * scale, origin.y() * scale, 200 * scale, 120 * scale);
            const QSize requested(100, 80), maximum(1280, 800);
            const QRectF result = aspect::resizedSelection(current, requested, maximum);
            exports.append(QJsonObject{{"scale", scale}, {"current", box(current)}, {"requested", size(requested)},
                                      {"maximum", size(maximum)}, {"result", box(result)}, {"aligned", box(result.toAlignedRect())}});
        }
    }
    output["aligned_exports"] = exports;

    // 6. 【选区尺寸对照】【主题绘制】固定一百乘八十尺寸保存两种主题的实际像素，数值采样保持原有操作序列
    panel.m_widthField->cancelEditing();
    panel.m_heightField->cancelEditing();
    for (bool dark : {false, true}) {
        panel.setToolbarPalette(markshot::ui::toolbarPalette(dark ? markshot::ui::UiThemeMode::Dark : markshot::ui::UiThemeMode::Light));
        panel.setValues(QSize(100, 80), QSize(1200, 800), 0.0);
        for (qreal scale : {1.0, 1.5}) {
            render(panel, renderDirectory + QStringLiteral("/standalone-%1-%2.png").arg(dark ? "dark" : "light").arg(scale), scale);
        }
    }

    // 4. 【选区尺寸对照】【生产避让】覆盖外侧、内侧、底部、窄屏及硬障碍物导致隐藏
    QJsonArray placements;
    for (const QRect &selection : {QRect(100, 100, 400, 200), QRect(0, 0, 800, 600), QRect(760, 20, 30, 40), QRect(80, 570, 400, 30)}) {
        for (const QVector<QRect> &obstacles : {QVector<QRect>{}, QVector<QRect>{QRect(0, 30, 600, 80)},
                QVector<QRect>{QRect(0, 0, 800, 200), QRect(0, 500, 800, 100)}}) {
            const aspect::SelectionHudLayoutInput input{selection, panel.sizeHint(), host.rect(), obstacles, 6};
            const auto result = aspect::placeSelectionHud(input);
            QJsonArray blocks;
            for (const QRect &obstacle : obstacles) { blocks.append(box(obstacle)); }
            placements.append(QJsonObject{{"selection", box(selection)}, {"size", size(panel.sizeHint())}, {"bounds", box(host.rect())},
                {"obstacles", blocks}, {"result", result ? QJsonValue(box(*result)) : QJsonValue(QJsonValue::Null)}});
        }
    }
    output["placements"] = placements;
    QFile file(argv[1]);
    if (!file.open(QIODevice::WriteOnly)) { return 2; }
    file.write(QJsonDocument(output).toJson());
    return 0;
}
