// 【字号对照】【实际入口】驱动原版字号、滑条与滚轮，不代写修改或撤销逻辑

#pragma once
#include "shot_window.h"
#include "text_annotation_layout.h"
#include <QApplication>
#include <QJsonArray>
#include <QJsonObject>
#include <QSlider>
#include <QSpinBox>
#include <QTimer>
#include <QEventLoop>
#include <QWheelEvent>
#include <QListWidget>
#include <QScrollBar>

#include "editing_test_access.h"

/** 【字号对照】【已有文字】向 window 加入选中文字，无返回值，不调用私有桌面捕获。 */
void TextAnnotationEditingTest::select(ShotWindow &window)
{
        ShotWindow::Annotation annotation;
        annotation.id = 1;
        annotation.tool = ShotWindow::Tool::Text;
        annotation.text = QStringLiteral("Text properties");
        annotation.rect = QRectF(400, 300, 200, 40);
        annotation.width = 1;
        window.m_annotations.append(annotation);
        window.m_selectedAnnotationId = 1;
        window.m_selectedAnnotationIds = {1};
        window.m_tool = ShotWindow::Tool::Select;
        window.updateAnnotationPropertyPanel();
    }

/** 【字号对照】【完整样本】按 source 创建真实窗口，返回各输入入口和撤销后的实际字号与框宽。 */
QJsonArray TextAnnotationEditingTest::sizes(const QImage &source)
{
        QJsonArray result;
        for (bool wrapped : {false, true}) {
            for (const QString &entry : {QStringLiteral("slider"), QStringLiteral("spin")}) {
                auto window = create(source, wrapped);
                QJsonArray states{state(*window)};
                for (int points : {30, 40}) {
                    if (entry == QStringLiteral("spin")) {
                        window->initializePropertyFontPanel();
                        window->m_controls.m_propertyFontSizeSpin->setValue(points);
                    } else {
                        window->m_controls.m_propertyWidthSlider->setValue(points - 19);
                    }
                    states.append(state(*window));
                }
                result.append(QJsonObject{{"name", entry + (wrapped ? "-wrapped" : "-unwrapped")},
                    {"states", states}, {"undos", undo(*window, 3)}});
            }
        }
        for (const auto &item : std::initializer_list<std::pair<QString, QList<int>>>{
                {"wheel-whole", {120,120,-120,-120}}, {"wheel-fractional", {30,30,30,30,30,30,30,30}},
                {"wheel-pixels", {20,20,20,20,-40,-40}}, {"wheel-limit", {120,-120,-120}}}) {
            auto window = create(source, false);
            if (item.first == QStringLiteral("wheel-limit")) {
                window->m_annotations[0].width = 281;
                window->m_textSize = 281;
                markshot::shot::fitTextAnnotationRect(window->m_annotations[0]);
            }
            QJsonArray states{state(*window)};
            for (int delta : item.second) {
                wheel(*window, delta, item.first == QStringLiteral("wheel-pixels"));
                states.append(state(*window));
            }
            result.append(QJsonObject{{"name",item.first},{"states",states},{"undos",undo(*window,2)}});
        }
        {
            auto window = create(source, false);
            QJsonArray states{state(*window)};
            for (int index = 0; index < 2; ++index) {
                wheel(*window,120,false);
                QEventLoop wait;
                QTimer::singleShot(400,&wait,&QEventLoop::quit);
                wait.exec();
                states.append(state(*window));
            }
            result.append(QJsonObject{{"name","wheel-separated"},{"states",states},{"undos",undo(*window,3)}});
        }
        {
            auto window = create(source, false);
            window->m_textSize = 31;
            const auto before = state(*window);
            window->setSelectedTextFontSize(20);
            result.append(QJsonObject{{"name","same-value"},{"states",QJsonArray{before,state(*window)}},
                {"undos",undo(*window,1)}});
        }
        {
            auto window = create(source, true);
            auto second = window->m_annotations[0];
            second.id = 2;
            second.width = 21;
            window->m_annotations.append(second);
            wheel(*window,60,false);
            window->m_selectedAnnotationId = 2;
            window->m_selectedAnnotationIds = {2};
            window->updateAnnotationPropertyPanel();
            const auto before = state(*window);
            wheel(*window,60,false);
            result.append(QJsonObject{{"name","wheel-context"},{"states",QJsonArray{before,state(*window)}}});
        }
        controls(source, result);
        return result;
    }

/** 【字号对照】【控件滚轮】向 source 对应的真实控件发送高分辨率和像素滚轮，将状态追加到 result，无返回值。 */
void TextAnnotationEditingTest::controls(const QImage &source, QJsonArray &result)
{
        for (const QString &entry : {QStringLiteral("slider"), QStringLiteral("spin"), QStringLiteral("list"), QStringLiteral("bar")}) {
            for (const auto &item : std::initializer_list<std::pair<QString, QList<int>>>{
                    {"whole", {120,120,-120,-120}}, {"fractional", {30,30,-30,30,30,30,30}},
                    {"pixels", {20,20,20,20,-40,-40}}, {"ctrl", {120,-120}},
                    {"ctrl-fractional", {30,30,-30,30,30,30,30}}, {"limit", {30,30,-30,-30,-30,-30}}}) {
                auto window = create(source, false);
                window->initializePropertyFontPanel();
                window->updateAnnotationPropertyPanel();
                if (item.first == "limit") window->setSelectedTextFontSize(300);
                window->resize(1067, 667);
                window->show();
                window->toggleSelectedTextFontPanel();
                auto *panel = window->findChild<QWidget *>("propertyFontPanel");
                QApplication::processEvents();
                auto *list = panel->findChild<QListWidget *>();
                auto *bar = list->verticalScrollBar();
                bar->setValue(100);
                QWidget *target = entry == "slider" ? static_cast<QWidget *>(window->m_controls.m_propertyWidthSlider)
                    : entry == "spin" ? static_cast<QWidget *>(window->m_controls.m_propertyFontSizeSpin)
                    : entry == "bar" ? static_cast<QWidget *>(bar) : list->viewport();
                target->setFocus();
                QJsonArray states;
                auto record = [&]() {
                    auto value = state(*window);
                    value["list_value"] = bar->value();
                    value["list_single_step"] = bar->singleStep();
                    value["list_mode"] = list->verticalScrollMode();
                    value["spin_value"] = window->m_controls.m_propertyFontSizeSpin->value();
                    value["list_count"] = list->count();
                    value["list_max"] = bar->maximum();
                    value["list_page_step"] = bar->pageStep();
                    value["viewport_height"] = list->viewport()->height();
                    states.append(value);
                };
                record();
                for (int delta : item.second) {
                    const bool pixels = item.first == "pixels";
                    QWheelEvent event(QPointF(10,10), target->mapToGlobal(QPoint(10,10)), pixels ? QPoint(0,delta) : QPoint{},
                        pixels ? QPoint{} : QPoint(0,delta), Qt::NoButton,
                        item.first.startsWith("ctrl") ? Qt::ControlModifier : Qt::NoModifier, Qt::ScrollUpdate, false);
                    QApplication::sendEvent(target, &event);
                    QApplication::processEvents();
                    record();
                }
                result.append(QJsonObject{{"name",entry + "-wheel-" + item.first},{"states",states},{"undos",undo(*window,3)}});
            }
        }
    }

/** 【字号对照】【窗口构造】按 source 和 wrapped 返回原版编辑窗口，使用固定文字和私有配置。 */
std::unique_ptr<ShotWindow> TextAnnotationEditingTest::create(const QImage &source, bool wrapped)
{
        auto window = std::make_unique<ShotWindow>(source,"reference",QRect(0,0,1067,667),QVector<markshot::WindowInfo>{},false);
        window->setDefaultTools(ShotWindow::Tool::Text,ShotWindow::Tool::Text);
        window->startFullscreenAnnotation();
        select(*window);
        auto &annotation = window->m_annotations[0];
        annotation.fontFamily = QStringLiteral("Inter");
        annotation.fontWeight = QFont::Normal;
        annotation.text = wrapped ? QStringLiteral("The quick brown fox jumps over the lazy dog") : QStringLiteral("Mark Shot");
        annotation.rect.setWidth(wrapped ? 90.0 : 1000.0);
        if (!wrapped) markshot::shot::fitTextAnnotationRect(annotation);
        window->m_textSize = 1;
        window->updateAnnotationPropertyPanel();
        return window;
    }

/** 【字号对照】【状态记录】读取 window 选中对象、默认字号和余量；返回真实数值，不计算期望结果。 */
QJsonObject TextAnnotationEditingTest::state(const ShotWindow &window)
{
        const auto *annotation = window.annotationById(window.m_selectedAnnotationId.value_or(1));
        return QJsonObject{{"points",annotation ? 19.0 + annotation->width : -1.0},
            {"default_points",19.0 + window.m_textSize},{"width",annotation ? annotation->rect.width() : 0.0},
            {"wrapped",annotation && markshot::shot::textAnnotationWraps(*annotation)},
            {"remainder",window.m_annotationWidthWheelRemainder},
            {"history_timer_ms",window.m_annotationWidthWheelHistoryTimer->interval()}};
    }

/** 【字号对照】【滚轮发送】把 delta 作为角度或 pixels 像素事件送入 window，无返回值。 */
void TextAnnotationEditingTest::wheel(ShotWindow &window, int delta, bool pixels)
{
        QWheelEvent event(QPointF(700,500),QPointF(700,500),pixels ? QPoint(0,delta) : QPoint{},
            pixels ? QPoint{} : QPoint(0,delta),Qt::NoButton,Qt::NoModifier,Qt::ScrollUpdate,false);
        QApplication::sendEvent(&window,&event);
    }

/** 【字号对照】【撤销记录】对 window 连续撤销 count 次，返回每次实际状态，默认值不手动还原。 */
QJsonArray TextAnnotationEditingTest::undo(ShotWindow &window, int count)
{
        QJsonArray result;
        for (int index = 0; index < count; ++index) {
            window.undoAnnotationEdit();
            result.append(state(window));
        }
        return result;
    }
