#include "shot_window_module.h"

/**
 * 在指定图像坐标开始创建文字标注。
 * @param imagePoint 文字编辑器在图像中的起始坐标。
 * @return 无返回值。
 */
void ShotWindow::beginTextAnnotation(QPointF imagePoint)
{
    m_editingTextAnnotationId.reset();
    m_textEditorImagePoint = imagePoint;
    m_draft.reset();
    m_textEditor->clear();
    const int pointSize = qRound(19.0 + m_textSize);
    m_textEditor->setStyleSheet(
        markshot::theme::textEditorStyleSheet(m_currentColor, m_textBackgroundColor, pointSize));
    QFont editorFont = markshot::theme::textFont(pointSize, m_textWeight, m_textFontFamily);
    editorFont.setItalic(m_textItalic);
    m_textEditor->setFont(editorFont);
    m_textEditor->show();
    m_textEditor->raise();
    updateTextEditorGeometry();
    m_textEditor->setFocus(Qt::MouseFocusReason);
    updateLayerShellForIme();
    update();
}

/**
 * 打开当前选中文字标注的内联编辑器。
 * @return 无返回值。
 */
void ShotWindow::beginEditingSelectedTextAnnotation()
{
    if (!m_selectedAnnotationId.has_value()) {
        return;
    }
    Annotation *annotation = annotationById(*m_selectedAnnotationId);
    if (!annotation || annotation->tool != Tool::Text) {
        return;
    }

    m_editingTextAnnotationId = annotation->id;
    m_textEditorImagePoint = annotation->rect.normalized().topLeft();
    m_draft.reset();
    m_textEditor->setPlainText(annotation->text);
    const int pointSize = qRound(19.0 + annotation->width);
    m_textEditor->setStyleSheet(
        markshot::theme::textEditorStyleSheet(annotation->color, annotation->backgroundColor, pointSize));
    QFont editorFont =
        markshot::theme::textFont(pointSize, annotation->fontWeight, annotation->fontFamily);
    editorFont.setItalic(annotation->textItalic);
    m_textEditor->setFont(editorFont);
    if (m_annotationPropertyPanel) {
        m_annotationPropertyPanel->hide();
    }
    if (m_propertyColorDialogPanel) {
        m_propertyColorDialogPanel->hide();
    }
    if (m_propertyFontPanel) {
        m_propertyFontPanel->hide();
    }
    m_textEditor->show();
    m_textEditor->raise();
    const QRectF widgetRect = textContentRect(*annotation, true);
    m_textEditor->setGeometry(widgetRect.toAlignedRect().adjusted(0, 0, 1, 1));
    m_textEditor->setFocus(Qt::MouseFocusReason);
    updateLayerShellForIme();
    update();
}

/**
 * 提交内联编辑器内容，并创建或更新文字标注。
 * @return 无返回值。
 */
void ShotWindow::commitTextEditor()
{
    if (m_committingText || !m_textEditor || !m_textEditor->isVisible()) {
        return;
    }

    m_committingText = true;
    const QString text = m_textEditor->toPlainText().trimmed();
    const QRect editorGeometry = m_textEditor->geometry();
    m_textEditor->hide();
    m_textEditor->clear();
    setFocus(Qt::OtherFocusReason);
    updateLayerShellForIme();

    // 1. 编辑现有标注时保留标注身份并更新文字样式
    if (m_editingTextAnnotationId.has_value()) {
        if (Annotation *annotation = annotationById(*m_editingTextAnnotationId)) {
            pushHistorySnapshot();
            annotation->text = text;
            annotation->fontFamily = m_textEditor->font().family();
            annotation->fontWeight = m_textEditor->font().weight();
            annotation->textItalic = m_textEditor->font().italic();
            annotation->rect = textContentRect(*annotation, false);
            if (!annotation->points.isEmpty()) {
                annotation->points[0] = annotation->rect.topLeft();
            }
        }
        m_editingTextAnnotationId.reset();
        m_committingText = false;
        updateAnnotationPropertyPanel();
        update();
        return;
    }

    // 2. 新内容非空时创建文字标注并同步默认样式
    if (!text.isEmpty()) {
        pushHistorySnapshot();
        Annotation annotation;
        annotation.id = m_nextAnnotationId++;
        annotation.tool = Tool::Text;
        annotation.points.append(m_textEditorImagePoint);
        annotation.rect = QRectF(widgetToImage(editorGeometry.topLeft()),
                                 widgetToImage(editorGeometry.bottomRight())).normalized();
        annotation.text = text;
        annotation.color = m_currentColor;
        annotation.backgroundColor = m_textBackgroundColor;
        annotation.width = m_textSize;
        annotation.fontFamily = m_textEditor->font().family();
        annotation.fontWeight = m_textEditor->font().weight();
        annotation.textItalic = m_textEditor->font().italic();
        annotation.rect = textContentRect(annotation, false);
        m_textFontFamily = annotation.fontFamily;
        m_textWeight = annotation.fontWeight;
        m_textItalic = annotation.textItalic;
        m_annotations.append(annotation);
    }

    m_committingText = false;
    update();
    persistAnnotationState();
}

/**
 * 显示内联文字编辑器的统一上下文菜单。
 * @param globalPosition 菜单在屏幕坐标中的显示位置。
 * @return 无返回值。
 */
void ShotWindow::showTextEditorContextMenu(const QPoint &globalPosition)
{
    if (!m_textEditor) {
        return;
    }

    QMenu menu(this);
    menu.setStyleSheet(markshot::theme::menuStyleSheet());
    const QTextCursor cursor = m_textEditor->textCursor();
    const bool hasSelection = cursor.hasSelection();
    const bool hasDocumentText = !m_textEditor->document()->isEmpty();
    const bool hasClipboardText = !QApplication::clipboard()->text().isEmpty();

    auto addAction = [&menu](const QString &label,
                             const QKeySequence &shortcut,
                             bool enabled,
                             auto callback) {
        QAction *action = menu.addAction(label);
        action->setShortcut(shortcut);
        action->setShortcutVisibleInContextMenu(true);
        action->setEnabled(enabled);
        QObject::connect(action, &QAction::triggered, menu.parent(), callback);
    };

    addAction(MS_TR("Undo"), QKeySequence::Undo,
              m_textEditor->document()->isUndoAvailable(),
              [this] { m_textEditor->undo(); });
    addAction(MS_TR("Redo"), QKeySequence::Redo,
              m_textEditor->document()->isRedoAvailable(),
              [this] { m_textEditor->redo(); });
    menu.addSeparator();
    addAction(MS_TR("Cut"), QKeySequence::Cut, hasSelection,
              [this] { m_textEditor->cut(); });
    addAction(MS_TR("Copy"), QKeySequence::Copy, hasSelection,
              [this] { m_textEditor->copy(); });
    addAction(MS_TR("Paste"), QKeySequence::Paste, hasClipboardText,
              [this] { m_textEditor->paste(); });
    addAction(MS_TR("Delete"), QKeySequence(Qt::Key_Delete), hasSelection,
              [this] {
                  QTextCursor selection = m_textEditor->textCursor();
                  selection.removeSelectedText();
                  m_textEditor->setTextCursor(selection);
              });
    menu.addSeparator();
    addAction(MS_TR("Select All"), QKeySequence::SelectAll, hasDocumentText,
              [this] { m_textEditor->selectAll(); });

    menu.exec(globalPosition);
}
