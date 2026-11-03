// 【文字菜单对照】【实际入口】读取原版菜单并发送真实菜单键盘事件，不复刻菜单或编辑行为

#pragma once
#include "text_editing.h"
#include "ui/context_menu.h"
#include <QClipboard>
#include <QTimer>

/** 【文字菜单对照】【完整样本】按 source 创建独立编辑器，prefix 保存菜单图像；返回真实动作状态与结果。 */
QJsonArray TextAnnotationEditingTest::menus(const QImage &source, const QString &prefix)
{
    QJsonArray result;
    for (const QString &name : {QStringLiteral("empty"),QStringLiteral("unselected"),
            QStringLiteral("selected"),QStringLiteral("copy"),QStringLiteral("cut"),
            QStringLiteral("delete"),QStringLiteral("paste"),QStringLiteral("undo"),QStringLiteral("redo")}) {
        auto window = editingWindow(source,false);
        QApplication::clipboard()->setText(name == "empty" ? QString{} : QStringLiteral("paste\r\nvalue"));
        if (name != "empty") editorKey(*window,Qt::Key_unknown,Qt::NoModifier,QStringLiteral("alpha beta"));
        if (name == "selected" || name == "copy" || name == "cut" || name == "delete")
            editorKey(*window,Qt::Key_A,Qt::ControlModifier);
        if (name == "redo") editorKey(*window,Qt::Key_Z,Qt::ControlModifier);
        QJsonObject record{{"name",name},{"before",editingState(*window)}};
        QTimer::singleShot(0,window.get(),[&] {
            auto *menu = qobject_cast<markshot::ui::ContextMenu *>(QApplication::activePopupWidget());
            if (!menu) { record["error"] = "menu absent"; QApplication::quit(); return; }
            QJsonArray rows;
            for (QAction *action : menu->actions()) {
                const QRect rect = menu->actionGeometry(action);
                rows.append(QJsonObject{{"label",action->text()},{"shortcut",action->shortcut().toString(QKeySequence::NativeText)},
                    {"enabled",action->isEnabled()},{"separator",action->isSeparator()},
                    {"geometry",QJsonArray{rect.x(),rect.y(),rect.width(),rect.height()}}});
            }
            record["rows"] = rows;
            record["size"] = QJsonArray{menu->width(),menu->height()};
            record["font"] = QJsonObject{{"family",menu->font().family()},{"points",menu->font().pointSizeF()},
                {"height",menu->fontMetrics().height()}};
            menu->grab().save(prefix + "-menu-" + name + ".png");
            const QHash<QString,QString> labels{{"copy","Copy"},{"cut","Cut"},{"delete","Delete"},
                {"paste","Paste"},{"undo","Undo"},{"redo","Redo"}};
            if (labels.contains(name)) {
                for (QAction *action : menu->actions()) if (action->text() == labels.value(name)) menu->setActiveAction(action);
                QKeyEvent event(QEvent::KeyPress,Qt::Key_Return,Qt::NoModifier);
                QApplication::sendEvent(menu,&event);
            } else {
                QKeyEvent event(QEvent::KeyPress,Qt::Key_Escape,Qt::NoModifier);
                QApplication::sendEvent(menu,&event);
            }
        });
        window->showTextEditorContextMenu(window->mapToGlobal(QPoint(410,310)));
        record["after"] = editingState(*window);
        record["clipboard"] = QApplication::clipboard()->text();
        if (name == "unselected") {
            editorKey(*window,Qt::Key_unknown,Qt::NoModifier,QStringLiteral("x"));
            record["post_menu_typing"] = editingState(*window);
            editorKey(*window,Qt::Key_Z,Qt::ControlModifier);
            record["post_menu_undo"] = editingState(*window);
        }
        result.append(record);
    }
    return result;
}
