// 【文字光标对照】【实际绘制】采样原版视口，不替换光标计时、焦点或选择处理

#pragma once
#include "text_editing.h"
#include "ui/context_menu.h"
#include <QElapsedTimer>
#include <QEventLoop>
#include <QTimer>

/** 【文字光标对照】【事件等待】处理 duration 毫秒真实事件与计时器，无返回值。 */
void caretWait(int duration)
{
    QEventLoop loop;
    QTimer::singleShot(duration,&loop,&QEventLoop::quit);
    loop.exec();
}

/** 【文字光标对照】【样本采集】按 editor、prefix、name 采样 count 次实际光标区域，返回时间、焦点和像素。 */
QJsonArray caretSamples(QTextEdit *editor, const QString &prefix, const QString &name, int count)
{
    QJsonArray samples;
    QElapsedTimer elapsed;
    elapsed.start();
    const int interval = std::max(1,QApplication::cursorFlashTime()/8);
    for (int index=0;index<count;++index) {
        if (index) caretWait(interval);
        const QRect rect = editor->cursorRect().adjusted(-1,0,1,0);
        const QImage image = editor->viewport()->grab(rect).toImage().convertToFormat(QImage::Format_ARGB32);
        int caretPixels=0;
        for (int y=0;y<image.height();++y) for (int x=0;x<image.width();++x) {
            const QColor pixel = image.pixelColor(x,y);
            if (pixel.alpha()>0 && pixel.red()>150 && pixel.red()>pixel.green()*1.7 && pixel.red()>pixel.blue()*1.7) ++caretPixels;
        }
        image.save(prefix+"-"+name+"-"+QString::number(index)+".png");
        samples.append(QJsonObject{{"ms",elapsed.elapsed()},{"focus",editor->hasFocus()},
            {"selection",editor->textCursor().hasSelection()},{"caret_pixels",caretPixels},
            {"center_rgba",QJsonArray{image.pixelColor(image.width()/2,image.height()/2).red(),
                image.pixelColor(image.width()/2,image.height()/2).green(),
                image.pixelColor(image.width()/2,image.height()/2).blue(),
                image.pixelColor(image.width()/2,image.height()/2).alpha()}},
            {"cursor",QJsonArray{rect.x(),rect.y(),rect.width(),rect.height()}}});
    }
    return samples;
}

/** 【文字光标对照】【完整样本】按 source 创建真实编辑器，prefix 保存视口；返回系统周期和各焦点状态。 */
QJsonObject TextAnnotationEditingTest::caret(const QImage &source, const QString &prefix)
{
    auto window=editingWindow(source,false);
    window->activateWindow();
    window->m_textEditor->setFocus();
    editorKey(*window,Qt::Key_unknown,Qt::NoModifier,QStringLiteral("alpha "));
    QJsonObject result{{"flash_ms",QApplication::cursorFlashTime()}};
    result["typing"]=caretSamples(window->m_textEditor,prefix,"typing",17);
    editorKey(*window,Qt::Key_unknown,Qt::NoModifier,QStringLiteral(" "));
    result["input_reset"]=caretSamples(window->m_textEditor,prefix,"reset",5);
    editorKey(*window,Qt::Key_A,Qt::ControlModifier);
    result["selected"]=caretSamples(window->m_textEditor,prefix,"selected",9);
    editorKey(*window,Qt::Key_End,Qt::NoModifier);
    window->initializePropertyFontPanel();
    window->toggleSelectedTextFontPanel();
    caretWait(30);
    result["field_focus"]=caretSamples(window->m_textEditor,prefix,"field",9);
    window->m_controls.m_propertyFontPanel->hide();
    window->m_textEditor->setFocus();
    result["focus_return"]=caretSamples(window->m_textEditor,prefix,"return",9);
    window->setFocus();
    result["focus_out"]=caretSamples(window->m_textEditor,prefix,"out",9);
    window->m_textEditor->setFocus();
    QTimer::singleShot(0,window.get(),[&] {
        result["menu_focus"]=caretSamples(window->m_textEditor,prefix,"menu",9);
        if (auto *menu=qobject_cast<markshot::ui::ContextMenu *>(QApplication::activePopupWidget())) {
            QKeyEvent escape(QEvent::KeyPress,Qt::Key_Escape,Qt::NoModifier);
            QApplication::sendEvent(menu,&escape);
        }
    });
    window->showTextEditorContextMenu(window->mapToGlobal(QPoint(410,310)));
    result["menu_return"]=caretSamples(window->m_textEditor,prefix,"menu-return",9);
    const QPoint position=window->m_textEditor->cursorRect().center();
    QMouseEvent press(QEvent::MouseButtonPress,position,window->m_textEditor->viewport()->mapToGlobal(position),
        Qt::LeftButton,Qt::LeftButton,Qt::NoModifier);
    QApplication::sendEvent(window->m_textEditor->viewport(),&press);
    result["mouse_held"]=caretSamples(window->m_textEditor,prefix,"mouse-held",9);
    QMouseEvent release(QEvent::MouseButtonRelease,position,window->m_textEditor->viewport()->mapToGlobal(position),
        Qt::LeftButton,Qt::NoButton,Qt::NoModifier);
    QApplication::sendEvent(window->m_textEditor->viewport(),&release);
    result["mouse_return"]=caretSamples(window->m_textEditor,prefix,"mouse-return",9);
    window->applyPropertyColor(QColor(255,77,77,128));
    result["opacity"]=caretSamples(window->m_textEditor,prefix,"opacity",9);
    return result;
}
