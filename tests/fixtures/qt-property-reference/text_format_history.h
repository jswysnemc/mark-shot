// 【文字格式对照】【实际入口】读取 QTextDocument 实际格式，样式与撤销由原版方法处理

#pragma once
#include "editing_test_access.h"
#include <QTextBlock>
#include <QTextFragment>

/** 【文字格式对照】【字体记录】按 font 返回字号、家族、字重与斜体，保持真实字段，不推算历史结果。 */
QJsonObject formatFont(const QFont &font)
{
    return {{"family",font.family()},{"points",font.pointSizeF()},
        {"weight",static_cast<int>(font.weight())},{"italic",font.italic()}};
}

/** 【文字格式对照】【窗口准备】按 source、existing 构造一次开始的真实编辑器，默认样式固定为普通二十点，返回窗口。 */
std::unique_ptr<ShotWindow> TextAnnotationEditingTest::formatWindow(const QImage &source, bool existing)
{
    auto window = create(source,false);
    window->m_textFontFamily = QStringLiteral("Inter");
    window->m_textWeight = QFont::Normal;
    window->m_textItalic = false;
    window->resize(1067,667);
    window->show();
    if (existing) {
        window->m_tool = ShotWindow::Tool::Select;
        window->beginEditingSelectedTextAnnotation();
    } else {
        window->m_annotations.clear();
        window->m_selectedAnnotationId.reset();
        window->m_selectedAnnotationIds.clear();
        window->m_tool = ShotWindow::Tool::Text;
        window->beginTextAnnotation(QPointF(400,300));
    }
    QApplication::processEvents();
    return window;
}

/** 【文字格式对照】【格式状态】读取 window 默认样式、全部字符格式与持久对象，返回实际值，不依赖界面像素推算。 */
QJsonObject TextAnnotationEditingTest::formatState(const ShotWindow &window)
{
    QJsonObject result = editingState(window);
    result["defaults"] = QJsonObject{{"family",window.m_textFontFamily},{"points",19 + window.m_textSize},
        {"weight",static_cast<int>(window.m_textWeight)},{"italic",window.m_textItalic},
        {"color",window.m_currentColor.name(QColor::HexArgb)},{"background",window.m_textBackgroundColor.name(QColor::HexArgb)}};
    result["widget_font"] = formatFont(window.m_textEditor->font());
    result["document_font"] = formatFont(window.m_textEditor->document()->defaultFont());
    result["current_font"] = formatFont(window.m_textEditor->currentCharFormat().font());
    result["metadata"] = QJsonObject{{"family",window.m_textEditor->property("markShotFontFamily").toString()},
        {"weight",window.m_textEditor->property("markShotFontWeight").toInt()},
        {"italic",window.m_textEditor->property("markShotFontItalic").toBool()}};
    result["selection"] = QJsonArray{window.m_textEditor->textCursor().selectionStart(),window.m_textEditor->textCursor().selectionEnd()};
    QJsonArray spans;
    for (QTextBlock block = window.m_textEditor->document()->begin(); block.isValid(); block = block.next()) {
        for (auto iterator = block.begin(); !iterator.atEnd(); ++iterator) {
            const auto fragment = iterator.fragment();
            if (!fragment.isValid()) continue;
            auto entry = formatFont(fragment.charFormat().font());
            entry["text"] = fragment.text();
            entry["position"] = fragment.position();
            spans.append(entry);
        }
    }
    result["spans"] = spans;
    QJsonArray annotations;
    for (const auto &annotation : window.m_annotations) {
        annotations.append(QJsonObject{{"id",annotation.id},{"text",annotation.text},{"points",19 + annotation.width},
            {"family",annotation.fontFamily},{"weight",static_cast<int>(annotation.fontWeight)},{"italic",annotation.textItalic}});
    }
    result["annotations"] = annotations;
    return result;
}

/** 【文字格式对照】【完整样本】按 source 对新建和已有文字执行样式、局部撤销与提交，返回实际状态序列。 */
QJsonArray TextAnnotationEditingTest::formats(const QImage &source)
{
    QJsonArray result;
    for (bool existing : {false,true}) {
        for (const QString &action : {QStringLiteral("bold"),QStringLiteral("italic"),QStringLiteral("family"),
                QStringLiteral("size"),QStringLiteral("color"),QStringLiteral("background"),QStringLiteral("opacity"),QStringLiteral("same-size")}) {
            auto window = formatWindow(source,existing);
            if (!existing) editorKey(*window,Qt::Key_unknown,Qt::NoModifier,QStringLiteral("alpha beta"));
            QJsonArray states{formatState(*window)};
            editorKey(*window,Qt::Key_A,Qt::ControlModifier);
            states.append(formatState(*window));
            if (action == "bold") window->setSelectedTextBold(true);
            else if (action == "italic") window->setSelectedTextItalic(true);
            else if (action == "family") window->setSelectedTextFontFamily(QStringLiteral("Liberation Sans"));
            else if (action == "size") window->setSelectedTextFontSize(30);
            else if (action == "same-size") window->setSelectedTextFontSize(20);
            else if (action == "opacity") window->setSelectedAnnotationOpacity(90);
            else {
                window->m_controls.m_propertyColorEditingTextBackground = action == "background";
                window->applyPropertyColor(QColor(255,77,77,128));
            }
            states.append(formatState(*window));
            editorKey(*window,Qt::Key_Z,Qt::ControlModifier);
            states.append(formatState(*window));
            editorKey(*window,Qt::Key_Z,Qt::ControlModifier | Qt::ShiftModifier);
            states.append(formatState(*window));
            editorKey(*window,Qt::Key_Z,Qt::ControlModifier);
            editorKey(*window,Qt::Key_unknown,Qt::NoModifier,QStringLiteral("x"));
            states.append(formatState(*window));
            editorKey(*window,Qt::Key_Z,Qt::ControlModifier);
            states.append(formatState(*window));
            window->commitTextEditor();
            states.append(formatState(*window));
            window->undoAnnotationEdit();
            states.append(formatState(*window));
            result.append(QJsonObject{{"name",QString(existing ? "existing-" : "new-")+action},{"states",states}});
        }
    }
    {
        auto window = formatWindow(source,false);
        editorKey(*window,Qt::Key_unknown,Qt::NoModifier,QStringLiteral("alpha"));
        QJsonArray states{formatState(*window)};
        window->setSelectedTextBold(true);
        states.append(formatState(*window));
        window->setSelectedTextItalic(true);
        states.append(formatState(*window));
        for (int index=0;index<4;++index) {
            editorKey(*window,Qt::Key_Z,Qt::ControlModifier);
            states.append(formatState(*window));
        }
        for (int index=0;index<4;++index) {
            editorKey(*window,Qt::Key_Z,Qt::ControlModifier | Qt::ShiftModifier);
            states.append(formatState(*window));
        }
        result.append(QJsonObject{{"name","sequential"},{"states",states}});
    }
    for (const QString &action : {QStringLiteral("empty"),QStringLiteral("resume-empty"),QStringLiteral("confirm-again"),QStringLiteral("middle")}) {
        auto window = formatWindow(source,false);
        if (action != "empty") editorKey(*window,Qt::Key_unknown,Qt::NoModifier,QStringLiteral("alpha"));
        QJsonArray states{formatState(*window)};
        window->setSelectedTextFontSize(30);
        states.append(formatState(*window));
        editorKey(*window,Qt::Key_Z,Qt::ControlModifier);
        states.append(formatState(*window));
        if (action == "resume-empty") {
            editorKey(*window,Qt::Key_Z,Qt::ControlModifier);
        } else if (action == "confirm-again") {
            window->setSelectedTextFontSize(30);
        } else if (action == "middle") {
            editorKey(*window,Qt::Key_Home,Qt::NoModifier);
            editorKey(*window,Qt::Key_Right,Qt::NoModifier);
        }
        states.append(formatState(*window));
        editorKey(*window,Qt::Key_unknown,Qt::NoModifier,QStringLiteral("x"));
        states.append(formatState(*window));
        editorKey(*window,Qt::Key_Z,Qt::ControlModifier);
        states.append(formatState(*window));
        editorKey(*window,Qt::Key_Z,Qt::ControlModifier | Qt::ShiftModifier);
        states.append(formatState(*window));
        result.append(QJsonObject{{"name",action},{"states",states}});
    }
    for (bool existing : {false,true}) {
        auto window = formatWindow(source,existing);
        QJsonArray states{formatState(*window)};
        for (int index=0;index<3;++index) {
            editorKey(*window,Qt::Key_Z,Qt::ControlModifier);
            states.append(formatState(*window));
        }
        result.append(QJsonObject{{"name",existing ? "initial-existing" : "initial-new"},{"states",states}});
    }
    for (bool submit : {false,true}) {
        auto window = formatWindow(source,false);
        editorKey(*window,Qt::Key_unknown,Qt::NoModifier,QStringLiteral("alpha"));
        QJsonArray states{formatState(*window)};
        window->initializePropertyFontPanel();
        window->toggleSelectedTextFontPanel();
        QApplication::processEvents();
        window->m_controls.m_propertyFontSizeSpin->setFocus();
        window->m_controls.m_propertyFontSizeSpin->setValue(30);
        QKeyEvent navigate(QEvent::KeyPress,submit ? Qt::Key_Return : Qt::Key_Left,Qt::NoModifier);
        QApplication::sendEvent(window->m_controls.m_propertyFontSizeSpin->findChild<QLineEdit *>(),&navigate);
        states.append(formatState(*window));
        window->m_controls.m_propertyFontPanel->hide();
        window->m_textEditor->setFocus();
        if (submit) window->undoAnnotationEdit();
        else editorKey(*window,Qt::Key_Z,Qt::ControlModifier);
        states.append(formatState(*window));
        if (submit) window->redoAnnotation();
        else editorKey(*window,Qt::Key_Z,Qt::ControlModifier | Qt::ShiftModifier);
        states.append(formatState(*window));
        result.append(QJsonObject{{"name",submit ? "field-submit" : "field-navigation"},{"states",states}});
    }
    return result;
}
