// 【字段输入法对照】【实际控件】驱动原版字体搜索、字号和十六进制字段，不替换验证器或业务回调

#pragma once
#include "editing_test_access.h"
#include "ui/color_picker.h"
#include <QApplication>
#include <QFocusEvent>
#include <QInputMethodEvent>
#include <QLineEdit>
#include <QSpinBox>

/** 【字段输入法对照】【状态记录】读取 edit 的实际文本、光标、验证结果及信号次数，path 保存画面，返回状态 */
QJsonObject lineInputState(QLineEdit &edit, const QString &path, int changed, int edited)
{
    QApplication::processEvents();
    QInputMethodQueryEvent query(Qt::ImSurroundingText | Qt::ImCursorPosition | Qt::ImAnchorPosition | Qt::ImCursorRectangle);
    QApplication::sendEvent(&edit, &query);
    const auto rect = query.value(Qt::ImCursorRectangle).toRectF();
    int anchor = edit.cursorPosition();
    if (edit.hasSelectedText()) {
        const int start = edit.selectionStart();
        anchor = edit.cursorPosition() == start ? start + edit.selectedText().size() : start;
    }
    edit.grab().save(path);
    return {{"text",edit.text()},{"display_text",edit.displayText()},
        {"cursor",edit.cursorPosition()},{"anchor",anchor},{"selection",edit.selectedText()},
        {"acceptable",edit.hasAcceptableInput()},{"undo",edit.isUndoAvailable()},{"redo",edit.isRedoAvailable()},
        {"changed",changed},{"edited",edited},{"maximum",edit.maxLength()},
        {"surrounding",query.value(Qt::ImSurroundingText).toString()},
        {"surrounding_cursor",query.value(Qt::ImCursorPosition).toInt()},
        {"surrounding_anchor",query.value(Qt::ImAnchorPosition).toInt()},
        {"cursor_rect",QJsonArray{rect.x(),rect.y(),rect.width(),rect.height()}}};
}

/** 【字段输入法对照】【完整样本】按 source 构造原版字段，prefix 保存每次画面；返回输入事件与实际结果，不填写预期 */
QJsonArray TextAnnotationEditingTest::lineInputMethod(const QImage &source, const QString &prefix)
{
    const auto ime = [](const QString &preedit, const QString &commit = {}, int start = 0, int length = 0, int caret = -2) {
        return QJsonObject{{"kind","ime"},{"preedit",preedit},{"commit",commit},
            {"replace_start",start},{"replace_length",length},{"caret",caret}};
    };
    const auto key = [](const QString &name, bool ctrl = false, bool shift = false) {
        return QJsonObject{{"kind","key"},{"key",name},{"ctrl",ctrl},{"shift",shift}};
    };
    const QJsonObject undo{{"kind","undo"}}, redo{{"kind","redo"}}, enter{{"kind","enter"}};
    struct Case { QString role, name, initial; int cursor, anchor; QJsonArray events; };
    const QList<Case> cases{
        {"search","preedit-cancel",{},0,0,{ime("ni"),ime("nih"),ime({}),undo}},
        {"search","selection-preedit-cancel","alpha beta",7,3,{ime("ni"),ime({}),undo,redo}},
        {"search","empty-selected-event","alpha beta",7,3,{ime({}),undo}},
        {"search","backward-selection","alpha beta",3,7,{ime("ni"),ime({},QStringLiteral("\u4f60")),undo,redo}},
        {"search","unicode-confirmations",{},0,0,{ime({},QStringLiteral("\u4f60")),ime({},QStringLiteral("\u597d")),undo,undo,redo,redo}},
        {"search","commit-and-next",{},0,0,{ime("ni"),ime("hao",QStringLiteral("\u4f60")),ime({},QStringLiteral("\u597d")),undo,redo}},
        {"search","replacement-forward","alpha beta",3,3,{ime({},"x",1,3),undo,redo}},
        {"search","replacement-selected","alpha beta",7,3,{ime({},"x",-1,2),undo,redo}},
        {"search","supplementary",{},0,0,{ime(QStringLiteral("\U00020000")),ime({},QStringLiteral("\U00020000")),undo,redo}},
        {"search","hidden-cursor",{},0,0,{ime("ni",{},0,0,-1),ime({})}},
        {"search","programmatic-reset","alpha",5,5,{ime("ni"),QJsonObject{{"kind","set"},{"text","beta"}},undo}},
        {"search","enter-preedit","alpha",5,5,{ime("ni"),enter}},
        {"search","identical-selection","alpha beta",7,3,{ime({},"ha b"),undo,redo}},
        {"search","line-control-characters",{},0,0,{ime({},"one\ntwo\tend"),undo,redo}},
        {"size","preedit-cancel","20 pt",2,2,{ime("ni"),ime({}),undo}},
        {"size","selected-preedit","20 pt",2,0,{ime("ni"),ime({}),undo,redo}},
        {"size","valid-selected","20 pt",2,0,{ime({},"40"),undo,redo}},
        {"size","invalid-selected","20 pt",2,0,{ime({},QStringLiteral("\u4f60")),undo}},
        {"size","mixed-invalid","20 pt",2,0,{ime({},QStringLiteral("40\u4f60")),undo}},
        {"size","over-limit","20 pt",2,0,{ime({},"400"),undo}},
        {"size","successive-confirmations","20 pt",2,0,{ime({},"4"),ime({},"2"),undo,redo}},
        {"size","commit-with-preedit","20 pt",2,0,{ime("ni","40"),ime({}),enter}},
        {"size","invalid-with-preedit","20 pt",2,0,{ime("ni",QStringLiteral("40\u4f60")),ime({}),undo}},
        {"size","suffix-delete","20 pt",2,2,{ime({},{},0,3),undo,redo}},
        {"size","suffix-partial-delete","20 pt",2,2,{ime({},{},1,1),undo,redo}},
        {"size","select-number","20 pt",2,2,{key("a",true),ime({},"40"),undo,redo}},
        {"size","select-all-delete","20 pt",2,2,{key("a",true),key("backspace"),undo,redo}},
        {"size","navigation","20 pt",1,1,{key("end"),key("right"),key("home"),key("left"),key("end",false,true),key("right")}},
        {"size","selection-collapse","20 pt",2,0,{key("left"),key("a",true),key("right"),key("left",false,true),key("home",false,true)}},
        {"size","boundary-delete","20 pt",2,2,{key("delete"),key("backspace"),undo,redo,enter}},
        {"size","suffix-navigation","20 pt",3,4,{key("right"),key("left"),key("end"),key("home")}},
        {"size","suffix-arrows","20 pt",2,2,{key("right"),key("left"),key("right",false,true),key("left",false,true),key("end"),key("left",false,true)}},
        {"size","suffix-internal-cursor","20 pt",3,3,{key("left"),key("right"),key("home"),key("right"),key("right"),key("right")}},
        {"size","intermediate-enter","20 pt",2,0,{ime({},"4"),enter,undo}},
        {"size","empty-enter","20 pt",2,0,{ime("ni"),ime({}),enter,undo}},
        {"hex","preedit-cancel","#112233",7,7,{ime("ni"),ime({}),undo}},
        {"hex","selected-preedit","#112233",7,1,{ime("ni"),ime({}),undo,redo}},
        {"hex","valid-confirm","#112233",7,1,{ime({},"AABBCC"),enter,undo}},
        {"hex","invalid-confirm","#112233",7,1,{ime({},"g"),undo}},
        {"hex","mixed-invalid","#112233",7,1,{ime({},QStringLiteral("FF\u4f60")),undo}},
        {"hex","maximum-truncation","#112233",7,0,{ime({},"#AABBCCDDFF"),enter}},
        {"hex","commit-and-next","#112233",7,1,{ime("ni","AABBCC"),ime({}),enter}},
        {"hex","invalid-with-preedit","#112233",7,1,{ime("ni",QStringLiteral("FF\u4f60")),ime({}),undo}},
    };
    QJsonArray result;
    // 1. 【字段输入法对照】【原版构造】搜索与字号使用真实 ShotWindow 字体面板，颜色使用原版 ColorPicker
    for (const auto &entry : cases) {
        auto window = formatWindow(source,false);
        window->toggleSelectedTextFontPanel();
        markshot::ui::ColorPicker picker;
        picker.setColor(QColor(17,34,51));
        picker.resize(280,280);
        QLineEdit *edit = entry.role == "search" ? window->m_controls.m_propertyFontSearch
            : entry.role == "size" ? window->m_controls.m_propertyFontSizeSpin->findChild<QLineEdit *>()
            : picker.findChild<QLineEdit *>();
        if (entry.role == "hex") picker.show();
        edit->setText(entry.initial);
        if (entry.cursor == entry.anchor) edit->setCursorPosition(entry.cursor);
        else edit->setSelection(entry.anchor,entry.cursor-entry.anchor);
        edit->setFocus(Qt::OtherFocusReason);
        int changed = 0, edited = 0;
        QObject::connect(edit,&QLineEdit::textChanged,edit,[&changed] { ++changed; });
        QObject::connect(edit,&QLineEdit::textEdited,edit,[&edited] { ++edited; });
        const auto state = [&](int index) {
            auto value = lineInputState(*edit,prefix+"-"+entry.role+"-"+entry.name+"-"+QString::number(index)+".png",changed,edited);
            value["points"] = window->m_controls.m_propertyFontSizeSpin->value();
            value["color"] = picker.color().name(QColor::HexArgb);
            return value;
        };
        QJsonArray states{state(0)};
        // 2. 【字段输入法对照】【事件交付】通过公开事件驱动原版验证器、信号和撤销，不直接修改控件私有正文
        int index = 0;
        for (const auto &item : entry.events) {
            const auto event = item.toObject();
            const auto kind = event["kind"].toString();
            if (kind == "ime") {
                QList<QInputMethodEvent::Attribute> attributes;
                const auto text = event["preedit"].toString();
                const int caret = event["caret"].toInt(-2);
                if (caret != -2) attributes.append({QInputMethodEvent::Cursor,caret < 0 ? 0 : caret,caret < 0 ? 0 : 1,{}});
                QInputMethodEvent input(text,attributes);
                input.setCommitString(event["commit"].toString(),event["replace_start"].toInt(),event["replace_length"].toInt());
                QApplication::sendEvent(edit,&input);
            } else if (kind == "undo") edit->undo();
            else if (kind == "redo") edit->redo();
            else if (kind == "set") edit->setText(event["text"].toString());
            else if (kind == "enter") {
                QKeyEvent input(QEvent::KeyPress,Qt::Key_Return,Qt::NoModifier);
                QApplication::sendEvent(edit,&input);
            } else if (kind == "key") {
                const QString name = event["key"].toString();
                const int code = name == "a" ? Qt::Key_A : name == "end" ? Qt::Key_End
                    : name == "home" ? Qt::Key_Home : name == "left" ? Qt::Key_Left
                    : name == "right" ? Qt::Key_Right : name == "delete" ? Qt::Key_Delete : Qt::Key_Backspace;
                Qt::KeyboardModifiers modifiers = Qt::NoModifier;
                if (event["ctrl"].toBool()) modifiers |= Qt::ControlModifier;
                if (event["shift"].toBool()) modifiers |= Qt::ShiftModifier;
                QKeyEvent input(QEvent::KeyPress,code,modifiers);
                QApplication::sendEvent(edit,&input);
            }
            states.append(state(++index));
        }
        result.append(QJsonObject{{"role",entry.role},{"name",entry.name},{"initial",entry.initial},
            {"cursor",entry.cursor},{"anchor",entry.anchor},{"events",entry.events},{"states",states}});
        QObject::disconnect(edit,nullptr,edit,nullptr);
        // 3. 【字段输入法对照】【窗口关闭】先交付失焦再销毁控件，避免基类析构时回调已经析构的颜色成员
        edit->clearFocus();
        picker.hide();
        window->hide();
        QApplication::processEvents();
    }
    return result;
}
