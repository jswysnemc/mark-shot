#include "shot_window_module.h"

#include "text_annotation_layout.h"
#include "text_editor_box_chrome.h"
#include "text_editor_box_layout.h"
#include "ui/context_menu.h"

using namespace markshot::shot;

namespace {

/**
 * 去掉文字末尾的空白，保留开头的缩进和空行。
 * @param text 编辑器中的原始文字。
 * @return 去掉末尾空白后的文字，全为空白时返回空字符串。
 */
QString trimTrailingWhitespace(const QString &text)
{
    qsizetype end = text.size();
    while (end > 0 && text.at(end - 1).isSpace()) {
        --end;
    }
    return text.left(end);
}

/**
 * 判断两次提交的文字标注内容与排版是否完全一致。
 * @param left 提交前的标注。
 * @param right 提交后的标注。
 * @return 一致时返回 true。
 */
bool sameTextSubmission(const types::Annotation &left, const types::Annotation &right)
{
    return left.text == right.text && left.fontFamily == right.fontFamily
        && left.fontWeight == right.fontWeight && left.textItalic == right.textItalic
        && left.rect == right.rect && left.points == right.points;
}

}  // namespace

void ShotWindow::initializeTextEditor()
{
    m_textEditor = new QTextEdit(this);
    m_textEditor->setObjectName(QStringLiteral("textEditor"));
    m_textEditor->setPlaceholderText(MS_TR("Type text"));
    m_textEditor->setStyleSheet(markshot::theme::textEditorStyleSheet(QColor(94, 234, 212), QColor(0, 0, 0, 0), 24));
    m_textEditor->setAcceptRichText(false);
    m_textEditor->setTabChangesFocus(false);
    m_textEditor->setFrameShape(QFrame::NoFrame);
    m_textEditor->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    m_textEditor->setVerticalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    m_textEditor->setAttribute(Qt::WA_InputMethodEnabled, true);
    m_textEditor->viewport()->setAutoFillBackground(false);
    m_textEditor->setToolTip(MS_TR("Enter inserts newline, Ctrl+Enter or Esc commits, click outside commits"));
    connectSnowTextEditorGeometryIfEnabled();
    m_textEditor->hide();
    m_textEditor->installEventFilter(this);
    m_textEditor->viewport()->installEventFilter(this);
    m_textEditor->setContextMenuPolicy(Qt::NoContextMenu);
}

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
    applySnowTextEditorChromeIfEnabled();
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
    applySnowTextEditorChromeIfEnabled();
    if (m_controls.m_annotationPropertyPanel) {
        m_controls.m_annotationPropertyPanel->hide();
    }
    if (m_controls.m_propertyColorDialogPanel) {
        m_controls.m_propertyColorDialogPanel->hide();
    }
    if (m_controls.m_propertyFontPanel) {
        m_controls.m_propertyFontPanel->hide();
    }
    m_textEditor->show();
    m_textEditor->raise();
    const QRectF widgetRect = textContentRect(*annotation, true);
    m_textEditor->setGeometry(widgetRect.toAlignedRect().adjusted(0, 0, 1, 1));
    applySnowTextEditorGeometryIfEnabled();
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
    // 1. 隐藏前记下编辑器实际换行宽度，提交结果与编辑时看到的换行一致
    const QString text = trimTrailingWhitespace(m_textEditor->toPlainText());
    const QRect editorGeometry = m_textEditor->geometry();
    const std::optional<qreal> wrappedBoxWidth =
        wrappedEditorBoxWidth(m_textEditor->document(), annotationSizeScale(true));
    m_textEditor->hide();
    m_textEditor->clear();
    setFocus(Qt::OtherFocusReason);
    updateLayerShellForIme();

    // 2. 编辑现有标注时保留标注身份，清空文字即删除，内容未变不写历史
    if (m_editingTextAnnotationId.has_value()) {
        const int annotationId = *m_editingTextAnnotationId;
        m_editingTextAnnotationId.reset();
        bool removed = false;
        if (const Annotation *original = annotationById(annotationId)) {
            Annotation edited = *original;
            edited.text = text;
            edited.fontFamily = m_textEditor->font().family();
            edited.fontWeight = m_textEditor->font().weight();
            edited.textItalic = m_textEditor->font().italic();
            copyEditorFontToAnnotationIfEnabled(edited);
            fitTextAnnotationRect(edited, wrappedBoxWidth);
            if (!edited.points.isEmpty()) {
                edited.points[0] = edited.rect.topLeft();
            }
            if (text.isEmpty()) {
                pushHistorySnapshot();
                m_annotations.removeIf([annotationId](const Annotation &item) { return item.id == annotationId; });
                removed = true;
            } else if (!sameTextSubmission(*original, edited)) {
                pushHistorySnapshot();
                if (Annotation *target = annotationById(annotationId)) {
                    *target = edited;
                }
            }
        }
        // 3. 文字工具下提交后回到新建状态，选择工具保留选中以便继续调整
        if (removed || m_tool != Tool::Select) {
            setSelectedAnnotations({});
        }
        m_committingText = false;
        updateAnnotationPropertyPanel();
        updateCursor();
        update();
        return;
    }

    // 4. 新内容非空时创建文字标注并同步默认样式
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
        copyEditorFontToAnnotationIfEnabled(annotation);
        // 5. 编辑器未换行时按单行自然宽度收框，换行时沿用编辑器的换行宽度
        fitTextAnnotationRect(annotation, wrappedBoxWidth);
        m_textFontFamily = annotation.fontFamily;
        m_textWeight = annotation.fontWeight;
        m_textItalic = annotation.textItalic;
        m_annotations.append(annotation);
    }

    m_committingText = false;
    updateCursor();
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

    // 截图浮层固定为暗色玻璃风格，菜单与之保持一致
    markshot::ui::ContextMenu menu(this);
    menu.setColorScheme(markshot::ui::ContextMenu::ColorScheme::Dark);
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

namespace {

/// 关闭时保留原来的虚线输入框，以及 Esc 取消编辑
constexpr bool FEATURE_TEXT_EDITOR_SNOW_INPUT_ENABLED = true;

/**
 * 生成与 snow 画布输入一致的编辑器样式：编辑器透明，背景与边框由窗口绘制，字号字重交给 QFont。
 * @param color 文字颜色。
 * @param font 编辑器字体。
 * @param insets 输入框内边距，只把左上两边写入样式，右下两边留在视口内放光标。
 * @return 样式表。
 */
QString snowTextEditorStyleSheet(const QColor &color, const QFont &font, const TextEditorBoxInsets &insets)
{
    const QString foreground = QStringLiteral("rgba(%1, %2, %3, %4)")
                                   .arg(color.red())
                                   .arg(color.green())
                                   .arg(color.blue())
                                   .arg(color.alpha());
    QString family = font.family();
    family.replace(QLatin1Char('\\'), QStringLiteral("\\\\"));
    family.replace(QLatin1Char('\''), QStringLiteral("\\'"));
    return QStringLiteral(
               "QTextEdit#textEditor {"
               " color: %1;"
               " background: transparent;"
               " border: 0;"
               " padding: %3px 0px 0px %4px;"
               " font-family: '%2';"
               " font-size: %5pt;"
               " font-weight: %6;"
               " font-style: %7;"
               " selection-background-color: rgba(51, 153, 255, 120);"
               " selection-color: #FFFFFF;"
               "}"
               "QTextEdit#textEditor QAbstractScrollArea { background: transparent; border: 0; }"
               "QTextEdit#textEditor QWidget { background: transparent; font-family: '%2'; }")
        .arg(foreground, family)
        .arg(insets.top)
        .arg(insets.left)
        .arg(std::max(1, font.pointSize()))
        .arg(static_cast<int>(font.weight()))
        .arg(font.italic() ? QStringLiteral("italic") : QStringLiteral("normal"));
}

}  // namespace

/**
 * 按 snow 的文字输入规则处理编辑器事件：内边距上的鼠标留在输入框内，Esc 与 Ctrl+回车提交。开关关闭时直接返回。
 * @param watched 事件来源。
 * @param event 当前事件。
 * @return 已处理时返回 true。
 */
bool ShotWindow::handleSnowTextEditorEventIfEnabled(QObject *watched, QEvent *event)
{
    if (!FEATURE_TEXT_EDITOR_SNOW_INPUT_ENABLED) {
        return false;
    }
    // 1. 输入框内边距不属于视口，按在上面时不能被当成框外点击而提交
    if (handleTextEditorBoxPaddingMouse(m_textEditor, watched, event)) {
        return true;
    }
    if (!m_textEditor || !m_textEditor->isVisible() || watched != m_textEditor
        || !event || event->type() != QEvent::KeyPress) {
        return false;
    }

    // 2. Esc 与 Ctrl+回车结束输入；单独回车仍由编辑器换行
    const auto *keyEvent = static_cast<const QKeyEvent *>(event);
    const bool commitByEscape = keyEvent->key() == Qt::Key_Escape;
    const bool commitByEnter = (keyEvent->key() == Qt::Key_Return || keyEvent->key() == Qt::Key_Enter)
        && keyEvent->modifiers().testFlag(Qt::ControlModifier);
    if (!commitByEscape && !commitByEnter) {
        return false;
    }
    commitTextEditor();
    return true;
}

/**
 * 文字变化时让编辑器跟着内容增高。开关关闭时直接返回。
 * @return 无返回值。
 */
void ShotWindow::connectSnowTextEditorGeometryIfEnabled()
{
    if (!FEATURE_TEXT_EDITOR_SNOW_INPUT_ENABLED || !m_textEditor || !m_textEditor->document()) {
        return;
    }
    connect(m_textEditor->document(), &QTextDocument::contentsChanged, this, [this] {
        applySnowTextEditorGeometryIfEnabled();
    });
}

/**
 * 去掉文字编辑器的虚线框和占位提示，改用实际字体绘制。开关关闭时直接返回。
 * @return 无返回值。
 */
void ShotWindow::applySnowTextEditorChromeIfEnabled()
{
    if (!FEATURE_TEXT_EDITOR_SNOW_INPUT_ENABLED || !m_textEditor) {
        return;
    }
    // 1. 空输入框只留光标宽度，占位提示放不下，统一不显示
    m_textEditor->setPlaceholderText(QString());
    // 2. 内边距也属于输入框，鼠标指针与文字区一致，全局光标策略按该属性处理
    m_textEditor->setProperty("textCursorInPadding", true);
    m_textEditor->setCursor(Qt::IBeamCursor);
    // 3. 字体与成图保持一致
    applyEditorDisplayFontIfEnabled();
}

/**
 * 按文字内容摆放输入框：框即提交后的背景框，逐字加宽，右侧留出行尾光标。开关关闭时直接返回。
 * @return 已按 snow 规则摆放时返回 true，开关关闭或编辑器未显示时返回 false。
 */
bool ShotWindow::applySnowTextEditorGeometryIfEnabled()
{
    if (!FEATURE_TEXT_EDITOR_SNOW_INPUT_ENABLED || !m_textEditor || !m_textEditor->isVisible()) {
        return false;
    }
    if (m_updatingSnowTextEditorGeometry) {
        return true;
    }
    m_updatingSnowTextEditorGeometry = true;

    // 1. 内边距与提交后的背景一致，右侧内边距同时放得下行尾光标
    QTextDocument *document = m_textEditor->document();
    const qreal scale = annotationSizeScale(true);
    const TextEditorBoxInsets insets = textEditorBoxInsets(scale, m_textEditor->cursorWidth());
    const int insetWidth = insets.left + insets.right;
    const int insetHeight = insets.top + insets.bottom;
    const int fontHeight = m_textEditor->fontMetrics().height();
    const int minTextWidth = std::max(1, fontHeight / 2);
    const QRectF selection = imageRectToWidget(normalizedSelection());

    // 2. 已有文字从原背景框左上角开始，新文字从点击处开始，放不下最小输入框时向选区内回移
    QPoint boxTopLeft;
    const Annotation *editingAnnotation =
        m_editingTextAnnotationId.has_value() ? annotationById(*m_editingTextAnnotationId) : nullptr;
    if (editingAnnotation) {
        boxTopLeft = textContentRect(*editingAnnotation, true).topLeft().toPoint();
    } else {
        const QSize minimumBox(fontHeight * 2 + insetWidth, fontHeight + insetHeight);
        boxTopLeft = newTextEditorBoxTopLeft(imageToWidget(m_textEditorImagePoint).toPoint(),
                                             selection.toRect(), minimumBox);
    }

    // 3. 提交后背景框要留在选区内，导出时才不会被裁掉；多减 1px 吸收换算取整
    const int boxRightLimit = qFloor(std::min<qreal>(width() - 8, selection.right()));
    int maxTextWidth = boxRightLimit - boxTopLeft.x() - insetWidth - 1;
    // 4. 重新编辑已换行的文字时沿用原换行宽度，进入编辑不改变排版
    if (editingAnnotation && textAnnotationWraps(*editingAnnotation)) {
        const qreal wrapWidth = editingAnnotation->rect.normalized().width() - kTextBackgroundPaddingX * 2.0;
        maxTextWidth = std::min(maxTextWidth, qCeil(wrapWidth * scale));
    }
    maxTextWidth = std::max({minTextWidth, fontHeight, maxTextWidth});
    const int maxTextHeight = std::max(fontHeight, height() - boxTopLeft.y() - 8 - insetHeight);

    // 5. 按单行自然宽度逐字加宽，含输入法预编辑文字；超出上限后换行并增高
    // 换行宽度用固定像素值，编辑器改尺寸时 QTextEdit 不会再把它改回视口宽度
    m_textEditor->setLineWrapMode(QTextEdit::FixedPixelWidth);
    document->setTextWidth(-1);
    const int textWidth = std::clamp(qCeil(document->idealWidth()), minTextWidth, maxTextWidth);
    m_textEditor->setLineWrapColumnOrWidth(textWidth);
    const int textHeight = std::clamp(qCeil(document->size().height()), fontHeight, maxTextHeight);

    // 6. 编辑器就是整个输入框，保持左上角锚点，只把超出窗口的部分移回可见区域
    QRect boxRect(boxTopLeft, QSize(textWidth + insetWidth, textHeight + insetHeight));
    const int maxLeft = std::max(8, width() - boxRect.width() - 8);
    const int maxTop = std::max(8, height() - boxRect.height() - 8);
    boxRect.moveLeft(std::clamp(boxRect.left(), 8, maxLeft));
    boxRect.moveTop(std::clamp(boxRect.top(), 8, maxTop));
    if (m_textEditor->geometry() != boxRect) {
        m_textEditor->setGeometry(boxRect);
    }
    m_updatingSnowTextEditorGeometry = false;
    return true;
}

/**
 * 把编辑器字体改成和成图相同的字体。开关关闭时直接返回。
 * @return 无返回值。
 */
void ShotWindow::applyEditorDisplayFontIfEnabled()
{
    if (!FEATURE_TEXT_EDITOR_SNOW_INPUT_ENABLED || !m_textEditor) {
        return;
    }

    // 1. 字号、字重、家族与 drawAnnotation 使用同一套计算
    QString family = m_textFontFamily;
    QFont::Weight weight = m_textWeight;
    bool italic = m_textItalic;
    qreal width = m_textSize;
    QColor color = m_currentColor;
    QColor background = m_textBackgroundColor;
    if (m_editingTextAnnotationId.has_value()) {
        if (const Annotation *annotation = annotationById(*m_editingTextAnnotationId)) {
            if (!annotation->fontFamily.isEmpty()) {
                family = annotation->fontFamily;
            }
            weight = annotation->fontWeight;
            italic = annotation->textItalic;
            width = annotation->width;
            color = annotation->color;
            background = annotation->backgroundColor;
        }
    }
    const int pointSize = std::max(1, qRound((19.0 + width) * annotationSizeScale(true)));
    QFont font = markshot::theme::textFont(pointSize, weight, family);
    font.setItalic(italic);

    // 2. 样式表锁住家族和字号，避免编辑器回落到界面字体
    m_textEditor->setFont(font);
    if (QTextDocument *document = m_textEditor->document()) {
        document->setDocumentMargin(0);
        document->setDefaultFont(font);
    }
    const int position = m_textEditor->textCursor().position();
    QTextCursor cursor = m_textEditor->textCursor();
    cursor.select(QTextCursor::Document);
    QTextCharFormat format;
    format.setFont(font);
    cursor.setCharFormat(format);
    cursor.clearSelection();
    cursor.setPosition(std::clamp(position, 0, std::max(0, m_textEditor->document()->characterCount() - 1)));
    m_textEditor->setTextCursor(cursor);
    m_textEditor->mergeCurrentCharFormat(format);
    m_textEditor->setProperty("markShotFontFamily", family.isEmpty() ? font.family() : family);
    m_textEditor->setProperty("markShotFontWeight", static_cast<int>(weight));
    m_textEditor->setProperty("markShotFontItalic", italic);
    // 3. 编辑器只负责文字，背景与内边距按提交后的样子由窗口在下方绘制
    setTextEditorBoxBackground(m_textEditor, background);
    m_textEditor->setStyleSheet(snowTextEditorStyleSheet(
        color, font, textEditorBoxInsets(annotationSizeScale(true), m_textEditor->cursorWidth())));
}

/**
 * 提交时用编辑器记录的字体覆盖解析后的字体名。开关关闭时直接返回。
 * @param annotation 正在提交的文字标注。
 * @return 无返回值。
 */
void ShotWindow::copyEditorFontToAnnotationIfEnabled(Annotation &annotation)
{
    if (!FEATURE_TEXT_EDITOR_SNOW_INPUT_ENABLED || !m_textEditor) {
        return;
    }
    const QString family = m_textEditor->property("markShotFontFamily").toString();
    if (!family.isEmpty()) {
        annotation.fontFamily = family;
    }
    const QVariant weight = m_textEditor->property("markShotFontWeight");
    if (weight.isValid()) {
        annotation.fontWeight = static_cast<QFont::Weight>(weight.toInt());
    }
    const QVariant italic = m_textEditor->property("markShotFontItalic");
    if (italic.isValid()) {
        annotation.textItalic = italic.toBool();
    }
}
