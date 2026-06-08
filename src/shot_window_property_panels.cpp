#include "shot_window_module.h"

namespace cfg = markshot::config;
namespace shortcuts = markshot::shortcut;
using namespace markshot::shot;

void ShotWindow::setSelectedAnnotationOpacity(int opacity)
{
    opacity = std::clamp(opacity, 0, 100);
    const int alpha = qRound(opacity * 255.0 / 100.0);
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (!selectedIds.isEmpty()) {
        bool changed = false;
        for (int id : selectedIds) {
            const Annotation *annotation = annotationById(id);
            if (annotation && annotation->color.alpha() != alpha) {
                changed = true;
                break;
            }
        }
        if (!changed) {
            return;
        }
        pushHistorySnapshot();
        for (int id : selectedIds) {
            if (Annotation *annotation = annotationById(id)) {
                annotation->color.setAlpha(alpha);
            }
        }
    } else {
        if (m_currentColor.alpha() == alpha) {
            return;
        }
        m_currentColor.setAlpha(alpha);
    }

    if (m_draft.has_value()) {
        m_draft->color.setAlpha(alpha);
    }
    if (m_laserDraft.has_value()) {
        m_laserDraft->color.setAlpha(alpha);
    }
    if (m_propertyColorPicker && m_propertyColorDialogPanel && m_propertyColorDialogPanel->isVisible()) {
        const QSignalBlocker blocker(m_propertyColorPicker);
        m_propertyColorPicker->setColor(selectedIds.isEmpty() ? m_currentColor : annotationById(selectedIds.first())->color);
    }
    updateColorPalettePreview();
    updateAnnotationPropertyPanel();
    update();
}

void ShotWindow::setSelectedAnnotationFilled(bool filled)
{
    if (m_selectedAnnotationId.has_value()) {
        Annotation *annotation = annotationById(*m_selectedAnnotationId);
        if (!annotation || annotation->filled == filled) {
            return;
        }
        if (annotation->tool != Tool::Rectangle && annotation->tool != Tool::Ellipse) {
            return;
        }
        pushHistorySnapshot();
        annotation->filled = filled;
    } else {
        if (m_tool != Tool::Rectangle && m_tool != Tool::Ellipse) {
            return;
        }
        m_shapeFilled = filled;
    }
    updateAnnotationPropertyPanel();
    update();
}

void ShotWindow::setSelectedAnnotationCornerRadius(int radius)
{
    if (m_selectedAnnotationId.has_value()) {
        Annotation *annotation = annotationById(*m_selectedAnnotationId);
        if (!annotation || annotation->tool != Tool::Rectangle || qRound(annotation->cornerRadius) == radius) {
            return;
        }
        pushHistorySnapshot();
        annotation->cornerRadius = radius;
    } else {
        if (m_tool != Tool::Rectangle || qRound(m_rectangleCornerRadius) == radius) {
            return;
        }
        m_rectangleCornerRadius = radius;
    }
    updateAnnotationPropertyPanel();
    update();
}

void ShotWindow::setSelectedAnnotationArrowStyle(ArrowStyle style)
{
    if (m_selectedAnnotationId.has_value()) {
        Annotation *annotation = annotationById(*m_selectedAnnotationId);
        if (!annotation || annotation->tool != Tool::Arrow || annotation->arrowStyle == style) {
            return;
        }
        pushHistorySnapshot();
        annotation->arrowStyle = style;
    } else {
        if (m_tool != Tool::Arrow || m_arrowStyle == style) {
            return;
        }
        m_arrowStyle = style;
    }
    updateAnnotationPropertyPanel();
    update();
}

void ShotWindow::setSelectedHighlighterStyle(HighlighterStyle style)
{
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (!selectedIds.isEmpty()) {
        bool changed = false;
        for (int id : selectedIds) {
            const Annotation *annotation = annotationById(id);
            if (annotation && annotation->tool == Tool::Highlighter
                && annotation->highlighterStyle != style) {
                changed = true;
                break;
            }
        }
        if (!changed) {
            return;
        }
        pushHistorySnapshot();
        for (int id : selectedIds) {
            if (Annotation *annotation = annotationById(id);
                annotation && annotation->tool == Tool::Highlighter) {
                annotation->highlighterStyle = style;
            }
        }
    } else {
        if (m_tool != Tool::Highlighter || m_highlighterStyle == style) {
            return;
        }
        m_highlighterStyle = style;
    }

    if (m_draft.has_value() && m_draft->tool == Tool::Highlighter) {
        m_draft->highlighterStyle = style;
    }
    updateAnnotationPropertyPanel();
    update();
}

void ShotWindow::setSelectedNumberStyle(NumberStyle style)
{
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (!selectedIds.isEmpty()) {
        bool changed = false;
        for (int id : selectedIds) {
            const Annotation *annotation = annotationById(id);
            if (annotation && annotation->tool == Tool::Number
                && annotation->numberStyle != style) {
                changed = true;
                break;
            }
        }
        if (!changed) {
            return;
        }
        pushHistorySnapshot();
        for (int id : selectedIds) {
            if (Annotation *annotation = annotationById(id);
                annotation && annotation->tool == Tool::Number) {
                annotation->numberStyle = style;
            }
        }
    } else {
        if (m_tool != Tool::Number || m_numberStyle == style) {
            return;
        }
        m_numberStyle = style;
    }

    if (m_draft.has_value() && m_draft->tool == Tool::Number) {
        m_draft->numberStyle = style;
    }
    updateAnnotationPropertyPanel();
    update();
}

void ShotWindow::resetNumberSequence()
{
    if (m_nextNumber == 1) {
        return;
    }

    pushHistorySnapshot();
    m_nextNumber = 1;
    if (m_draft.has_value() && m_draft->tool == Tool::Number) {
        m_draft->number = m_nextNumber;
    }
    updateAnnotationPropertyPanel();
    update();
}

void ShotWindow::setSelectedMagnifierScale(int scaleValue)
{
    const qreal scale = magnifierScaleFromSliderValue(scaleValue);
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (!selectedIds.isEmpty()) {
        bool changed = false;
        for (int id : selectedIds) {
            const Annotation *annotation = annotationById(id);
            if (annotation && annotation->tool == Tool::Magnifier
                && !qFuzzyCompare(clampedMagnifierScale(annotation->magnifierScale), scale)) {
                changed = true;
                break;
            }
        }
        if (!changed) {
            return;
        }
        pushHistorySnapshot();
        for (int id : selectedIds) {
            if (Annotation *annotation = annotationById(id);
                annotation && annotation->tool == Tool::Magnifier) {
                annotation->magnifierScale = scale;
            }
        }
    } else {
        if (m_tool != Tool::Magnifier || qFuzzyCompare(m_magnifierScale, scale)) {
            return;
        }
        m_magnifierScale = scale;
    }

    if (m_draft.has_value() && m_draft->tool == Tool::Magnifier) {
        m_draft->magnifierScale = scale;
    }
    updateAnnotationPropertyPanel();
    update();
}

void ShotWindow::deleteSelectedAnnotation()
{
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (selectedIds.isEmpty()) {
        return;
    }
    pushHistorySnapshot();
    for (int i = m_annotations.size() - 1; i >= 0; --i) {
        if (selectedIds.contains(m_annotations.at(i).id)) {
            m_annotations.removeAt(i);
        }
    }
    setSelectedAnnotations({});
    m_annotationDrag = SelectionDrag::None;
    updateAnnotationPropertyPanel();
    updateCursor();
    update();
}

void ShotWindow::openSelectedAnnotationColorPalette()
{
    if (!m_propertyColorDialogPanel || !m_propertyColorPicker || !m_annotationPropertyPanel) {
        return;
    }
    const bool wasEditingTextBackground = m_propertyColorEditingTextBackground;
    m_propertyColorEditingTextBackground = false;

    if (m_propertyColorDialogPanel->isVisible() && !wasEditingTextBackground) {
        m_propertyColorDialogPanel->hide();
        return;
    }

    if (m_colorPalette) {
        m_colorPalette->hide();
    }
    QColor color = m_currentColor;
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (!selectedIds.isEmpty()) {
        if (const Annotation *annotation = annotationById(selectedIds.first())) {
            color = annotation->color;
        }
    }
    m_propertyColorEditHistoryCaptured = false;
    {
        const QSignalBlocker blocker(m_propertyColorPicker);
        m_propertyColorPicker->setColor(color);
    }
    updateAnnotationPropertyPanel();
    if (m_annotationPropertyPanel) {
        m_annotationPropertyPanel->show();
        m_annotationPropertyPanel->raise();
        if (QLayout *panelLayout = m_annotationPropertyPanel->layout()) {
            panelLayout->activate();
        }
        updateAnnotationPropertyPanelGeometry();
    }
    if (QLayout *colorLayout = m_propertyColorDialogPanel->layout()) {
        colorLayout->activate();
    }
    updatePropertyColorDialogGeometry();
    m_propertyColorDialogPanel->show();
    updatePropertyColorDialogGeometry();
    m_propertyColorDialogPanel->raise();
    QTimer::singleShot(0, this, [this] {
        if (m_propertyColorDialogPanel && m_propertyColorDialogPanel->isVisible()) {
            updatePropertyColorDialogGeometry();
            m_propertyColorDialogPanel->raise();
        }
    });
}

void ShotWindow::openSelectedTextBackgroundColorPalette()
{
    if (!m_propertyColorDialogPanel || !m_propertyColorPicker || !m_annotationPropertyPanel) {
        return;
    }
    const bool wasEditingTextBackground = m_propertyColorEditingTextBackground;
    m_propertyColorEditingTextBackground = true;

    if (m_propertyColorDialogPanel->isVisible() && wasEditingTextBackground) {
        m_propertyColorDialogPanel->hide();
        return;
    }

    if (m_colorPalette) {
        m_colorPalette->hide();
    }
    QColor color = m_textBackgroundColor;
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (selectedIds.size() == 1) {
        if (const Annotation *annotation = annotationById(selectedIds.first());
            annotation && annotation->tool == Tool::Text) {
            color = annotation->backgroundColor;
        }
    }
    m_propertyColorEditHistoryCaptured = false;
    {
        const QSignalBlocker blocker(m_propertyColorPicker);
        m_propertyColorPicker->setColor(color);
    }
    updateAnnotationPropertyPanel();
    if (m_annotationPropertyPanel) {
        m_annotationPropertyPanel->show();
        m_annotationPropertyPanel->raise();
        if (QLayout *panelLayout = m_annotationPropertyPanel->layout()) {
            panelLayout->activate();
        }
        updateAnnotationPropertyPanelGeometry();
    }
    if (QLayout *colorLayout = m_propertyColorDialogPanel->layout()) {
        colorLayout->activate();
    }
    updatePropertyColorDialogGeometry();
    m_propertyColorDialogPanel->show();
    updatePropertyColorDialogGeometry();
    m_propertyColorDialogPanel->raise();
    QTimer::singleShot(0, this, [this] {
        if (m_propertyColorDialogPanel && m_propertyColorDialogPanel->isVisible()) {
            updatePropertyColorDialogGeometry();
            m_propertyColorDialogPanel->raise();
        }
    });
}

void ShotWindow::toggleSelectedTextFontPanel()
{
    if (!m_propertyFontPanel || !m_propertyFontList || !m_propertyFontButton) {
        return;
    }

    if (m_propertyFontPanel->isVisible()) {
        m_propertyFontPanel->hide();
        return;
    }

    if (m_propertyColorDialogPanel) {
        m_propertyColorDialogPanel->hide();
    }
    updateAnnotationPropertyPanel();
    if (QLayout *fontLayout = m_propertyFontPanel->layout()) {
        fontLayout->activate();
    }
    updatePropertyFontPanelGeometry();
    m_propertyFontPanel->show();
    updatePropertyFontPanelGeometry();
    m_propertyFontPanel->raise();
}

void ShotWindow::applyPropertyColor(QColor color)
{
    if (!color.isValid()) {
        return;
    }
    const QVector<int> selectedIds = selectedAnnotationIds();
    if (m_propertyColorEditingTextBackground) {
        if (!selectedIds.isEmpty()) {
            if (!m_propertyColorEditHistoryCaptured) {
                pushHistorySnapshot();
                m_propertyColorEditHistoryCaptured = true;
            }
            for (int id : selectedIds) {
                if (Annotation *annotation = annotationById(id);
                    annotation && annotation->tool == Tool::Text) {
                    annotation->backgroundColor = color;
                }
            }
        } else if (m_tool == Tool::Text) {
            m_textBackgroundColor = color;
        }
    } else if (!selectedIds.isEmpty()) {
        if (!m_propertyColorEditHistoryCaptured) {
            pushHistorySnapshot();
            m_propertyColorEditHistoryCaptured = true;
        }
        for (int id : selectedIds) {
            if (Annotation *annotation = annotationById(id)) {
                annotation->color = color;
            }
        }
    } else {
        m_currentColor = color;
    }
    if (m_draft.has_value()) {
        m_draft->color = color;
    }
    if (m_textEditor && m_textEditor->isVisible()) {
        QColor editorColor = m_currentColor;
        QColor editorBackgroundColor = m_textBackgroundColor;
        qreal editorWidth = m_shapeWidth;
        if (m_editingTextAnnotationId.has_value()) {
            if (const Annotation *annotation = annotationById(*m_editingTextAnnotationId)) {
                editorColor = annotation->color;
                editorBackgroundColor = annotation->backgroundColor;
                editorWidth = annotation->width;
            }
        }
        m_textEditor->setStyleSheet(markshot::theme::textEditorStyleSheet(editorColor, editorBackgroundColor, qRound(20.0 + editorWidth)));
    }
    updateColorPalettePreview();
    updateAnnotationPropertyPanel();
    update();
}

void ShotWindow::clearAnnotations()
{
    commitTextEditor();
    if (m_annotations.isEmpty() && !m_draft.has_value() && m_laserStrokes.isEmpty() && !m_laserDraft.has_value()) {
        return;
    }

    pushHistorySnapshot();
    m_annotations.clear();
    m_draft.reset();
    m_laserStrokes.clear();
    m_laserDraft.reset();
    setSelectedAnnotations({});
    m_annotationDrag = SelectionDrag::None;
    m_annotationSelectionBoxActive = false;
    m_annotationHistoryCaptured = false;
    m_nextNumber = 1;
    m_nextAnnotationId = 1;
    if (m_propertyColorDialogPanel) {
        m_propertyColorDialogPanel->hide();
    }
    if (m_propertyFontPanel) {
        m_propertyFontPanel->hide();
    }
    updateAnnotationPropertyPanel();
    updateCursor();
    update();
}

void ShotWindow::setSelectedTextFontFamily(const QString &fontFamily)
{
    if (fontFamily.isEmpty()) {
        return;
    }

    if (m_selectedAnnotationId.has_value()) {
        Annotation *annotation = annotationById(*m_selectedAnnotationId);
        if (!annotation || annotation->tool != Tool::Text || annotation->fontFamily == fontFamily) {
            return;
        }
        pushHistorySnapshot();
        annotation->fontFamily = fontFamily;
    } else {
        if (m_tool != Tool::Text || m_textFontFamily == fontFamily) {
            return;
        }
        m_textFontFamily = fontFamily;
        if (m_textEditor && m_textEditor->isVisible() && !m_editingTextAnnotationId.has_value()) {
            m_textEditor->setFont(markshot::theme::textFont(qRound(20.0 + m_shapeWidth),
                                                            QFont::DemiBold,
                                                            m_textFontFamily));
        }
    }
    updateAnnotationPropertyPanel();
    update();
}

void ShotWindow::toggleOpenWithPanel()
{
    commitTextEditor();
    if (!m_openWithPanel || !hasUsableSelection()) {
        return;
    }
    if (m_colorPalette) {
        m_colorPalette->hide();
    }
    if (m_extensionPanel) {
        m_extensionPanel->hide();
    }

    if (m_openWithPanel->isVisible()) {
        m_openWithPanel->hide();
        return;
    }

    updateOpenWithPanel();
    updateOpenWithPanelGeometry();
    m_openWithPanel->show();
    m_openWithPanel->raise();
}

void ShotWindow::updateOpenWithPanel()
{
    if (!m_openWithPanel) {
        return;
    }

    QLayout *layout = m_openWithPanel->layout();
    while (QLayoutItem *item = layout->takeAt(0)) {
        if (QWidget *widget = item->widget()) {
            delete widget;
        }
        delete item;
    }

    auto *title = new QLabel(MS_TR("Open with"), m_openWithPanel);
    layout->addWidget(title);

    const QVector<DesktopApp> apps = imageDesktopApps();
    if (apps.isEmpty()) {
        auto *empty = new QLabel(MS_TR("No image desktop entries found"), m_openWithPanel);
        empty->setWordWrap(true);
        layout->addWidget(empty);
        m_openWithPanel->adjustSize();
        return;
    }

    auto *list = new QListWidget(m_openWithPanel);
    list->setFocusPolicy(Qt::NoFocus);
    list->setVerticalScrollMode(QAbstractItemView::ScrollPerPixel);
    list->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    list->setIconSize(QSize(22, 22));
    for (const DesktopApp &app : apps) {
        auto *item = new QListWidgetItem(app.name, list);
        item->setToolTip(app.desktopPath);
        item->setData(Qt::UserRole, app.desktopPath);
        item->setData(Qt::UserRole + 1, app.exec);
        item->setData(Qt::UserRole + 2, app.icon);
        QIcon icon;
        if (!app.icon.isEmpty()) {
            if (app.icon.startsWith(QLatin1Char('/')) && QFile::exists(app.icon)) {
                icon = QIcon(app.icon);
            } else {
                icon = QIcon::fromTheme(app.icon);
            }
        }
        if (!icon.isNull()) {
            item->setIcon(icon);
        }
    }
    list->setFixedHeight(std::min(360, std::max(48, static_cast<int>(apps.size()) * 36)));
    connect(list, &QListWidget::itemClicked, this, [this](QListWidgetItem *item) {
        if (!item) {
            return;
        }
        DesktopApp app;
        app.name = item->text();
        app.desktopPath = item->data(Qt::UserRole).toString();
        app.exec = item->data(Qt::UserRole + 1).toString();
        app.icon = item->data(Qt::UserRole + 2).toString();
        openSelectionWithDesktop(app);
    });
    layout->addWidget(list);

    m_openWithPanel->adjustSize();
}

void ShotWindow::updateOpenWithPanelGeometry()
{
    if (!m_openWithPanel) {
        return;
    }

    m_openWithPanel->adjustSize();
    const QSize panelSize(std::min(340, std::max(280, m_openWithPanel->sizeHint().width())),
                          std::min(540, std::max(80, m_openWithPanel->sizeHint().height())));
    const QRect toolbarRect = m_fullscreenAnnotation && m_toolbar
        ? m_toolbar->geometry()
        : (m_actionToolbar ? m_actionToolbar->geometry() : QRect(width() - 64, height() / 2 - 80, 56, 160));
    int x = toolbarRect.left() - panelSize.width() - kToolbarMargin;
    int y = toolbarRect.top();
    if (x < 8) {
        x = toolbarRect.right() + kToolbarMargin;
    }
    x = std::clamp(x, 8, std::max(8, width() - panelSize.width() - 8));
    y = std::clamp(y, 8, std::max(8, height() - panelSize.height() - 8));
    m_openWithPanel->setGeometry(x, y, panelSize.width(), panelSize.height());
}

void ShotWindow::toggleExtensionPanel()
{
    commitTextEditor();
    if (!m_extensionPanel || !hasUsableSelection()) {
        return;
    }
    if (m_colorPalette) {
        m_colorPalette->hide();
    }
    if (m_openWithPanel) {
        m_openWithPanel->hide();
    }

    if (m_extensionPanel->isVisible()) {
        m_extensionPanel->hide();
        return;
    }

    updateExtensionPanel();
    updateExtensionPanelGeometry();
    m_extensionPanel->show();
    m_extensionPanel->raise();
}

void ShotWindow::updateExtensionPanel()
{
    if (!m_extensionPanel) {
        return;
    }

    QLayout *layout = m_extensionPanel->layout();
    while (QLayoutItem *item = layout->takeAt(0)) {
        if (QWidget *widget = item->widget()) {
            delete widget;
        }
        delete item;
    }

    auto *title = new QLabel(MS_TR("Extensions"), m_extensionPanel);
    layout->addWidget(title);

    QString errorMessage;
    const QVector<ExtensionCommand> commands = extensionCommands(&errorMessage);
    if (!errorMessage.isEmpty()) {
        auto *error = new QLabel(errorMessage, m_extensionPanel);
        error->setWordWrap(true);
        layout->addWidget(error);
        m_extensionPanel->adjustSize();
        return;
    }

    if (commands.isEmpty()) {
        auto *empty = new QLabel(MS_TR("No extension commands configured.\nCreate %1").arg(extensionCommandsConfigPath()),
                                 m_extensionPanel);
        empty->setWordWrap(true);
        layout->addWidget(empty);
        m_extensionPanel->adjustSize();
        return;
    }

    auto *list = new QListWidget(m_extensionPanel);
    list->setFocusPolicy(Qt::NoFocus);
    list->setVerticalScrollMode(QAbstractItemView::ScrollPerPixel);
    list->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    for (const ExtensionCommand &command : commands) {
        auto *item = new QListWidgetItem(command.name, list);
        const QString tooltip = command.description.isEmpty()
            ? command.command
            : QStringLiteral("%1\n%2").arg(command.description, command.command);
        item->setToolTip(tooltip);
        item->setData(Qt::UserRole, command.command);
        item->setData(Qt::UserRole + 1, command.workingDirectory);
        item->setData(Qt::UserRole + 2, command.description);
        item->setData(Qt::UserRole + 3, command.saveImage);
        item->setData(Qt::UserRole + 4, command.closeOnStart);
    }
    list->setFixedHeight(std::min(360, std::max(48, static_cast<int>(commands.size()) * 36)));
    connect(list, &QListWidget::itemClicked, this, [this](QListWidgetItem *item) {
        if (!item) {
            return;
        }

        ExtensionCommand command;
        command.name = item->text();
        command.command = item->data(Qt::UserRole).toString();
        command.workingDirectory = item->data(Qt::UserRole + 1).toString();
        command.description = item->data(Qt::UserRole + 2).toString();
        command.saveImage = item->data(Qt::UserRole + 3).toBool();
        command.closeOnStart = item->data(Qt::UserRole + 4).toBool();
        runExtensionCommand(command);
    });
    layout->addWidget(list);

    m_extensionPanel->adjustSize();
}

void ShotWindow::updateExtensionPanelGeometry()
{
    if (!m_extensionPanel) {
        return;
    }

    m_extensionPanel->adjustSize();
    const QSize panelSize(std::min(380, std::max(300, m_extensionPanel->sizeHint().width())),
                          std::min(540, std::max(80, m_extensionPanel->sizeHint().height())));
    const QRect toolbarRect = m_fullscreenAnnotation && m_toolbar
        ? m_toolbar->geometry()
        : (m_actionToolbar ? m_actionToolbar->geometry() : QRect(width() - 64, height() / 2 - 80, 56, 160));
    int x = toolbarRect.left() - panelSize.width() - kToolbarMargin;
    int y = toolbarRect.top();
    if (x < 8) {
        x = toolbarRect.right() + kToolbarMargin;
    }
    x = std::clamp(x, 8, std::max(8, width() - panelSize.width() - 8));
    y = std::clamp(y, 8, std::max(8, height() - panelSize.height() - 8));
    m_extensionPanel->setGeometry(x, y, panelSize.width(), panelSize.height());
}

void ShotWindow::toggleColorPalette(QPoint position)
{
    commitTextEditor();
    if (m_openWithPanel) {
        m_openWithPanel->hide();
    }
    if (m_extensionPanel) {
        m_extensionPanel->hide();
    }
    if (!m_colorPalette) {
        return;
    }

    m_colorPaletteAnchor = position;
    if (m_colorPalette->isVisible()) {
        m_colorPalette->hide();
    } else {
        updateColorPaletteGeometry(position);
        m_colorPalette->show();
        m_colorPalette->raise();
    }
    update();
}

void ShotWindow::updateColorPaletteGeometry(QPoint anchor)
{
    if (!m_colorPalette) {
        return;
    }

    const QSize paletteSize(178, 178);
    int x = anchor.x() - paletteSize.width() / 2;
    int y = anchor.y() - paletteSize.height() / 2;
    x = std::clamp(x, 8, std::max(8, width() - paletteSize.width() - 8));
    y = std::clamp(y, 8, std::max(8, height() - paletteSize.height() - 8));
    m_colorPalette->setGeometry(x, y, paletteSize.width(), paletteSize.height());

    const QPoint center(paletteSize.width() / 2, paletteSize.height() / 2);
    const qreal radius = 68.0;
    const auto buttons = m_colorPalette->findChildren<QPushButton *>(QString(), Qt::FindDirectChildrenOnly);
    for (int i = 0; i < buttons.size(); ++i) {
        const qreal angle = -M_PI / 2.0 + (2.0 * M_PI * i / std::max<qsizetype>(1, buttons.size()));
        const QPoint pos(qRound(center.x() + std::cos(angle) * radius - 15.0),
                         qRound(center.y() + std::sin(angle) * radius - 15.0));
        buttons.at(i)->setGeometry(QRect(pos, QSize(30, 30)));
    }
    updateColorPalettePreview();
}

void ShotWindow::updateColorPalettePreview()
{
    if (!m_colorPalettePreview) {
        return;
    }

    const int size = std::clamp(qRound(currentToolPreviewSize()), 8, 34);
    const QPoint center(89, 89);
    m_colorPalettePreview->setGeometry(center.x() - size / 2, center.y() - size / 2, size, size);
    m_colorPalettePreview->setStyleSheet(QStringLiteral(
        "QWidget#colorPalettePreview {"
        " background: %1;"
        " border: 0;"
        " border-radius: 3px;"
        "}").arg(m_currentColor.name()));
}

void ShotWindow::updateTextEditorGeometry()
{
    if (!m_textEditor || !m_textEditor->isVisible()) {
        return;
    }
    auto keepEditorInsideWindow = [this](QRect editorRect) {
        const int minLeft = 8;
        const int minTop = 8;
        const int maxLeft = std::max(minLeft, width() - editorRect.width() - 8);
        const int maxTop = std::max(minTop, height() - editorRect.height() - 8);
        editorRect.moveLeft(std::clamp(editorRect.left(), minLeft, maxLeft));
        editorRect.moveTop(std::clamp(editorRect.top(), minTop, maxTop));
        return editorRect;
    };

    if (m_editingTextAnnotationId.has_value()) {
        if (const Annotation *annotation = annotationById(*m_editingTextAnnotationId)) {
            QRect editorRect = textContentRect(*annotation, true).toAlignedRect().adjusted(0, 0, 1, 1);
            m_textEditor->setGeometry(keepEditorInsideWindow(editorRect));
        }
        return;
    }

    const QPointF topLeft = imageToWidget(m_textEditorImagePoint);
    const QRectF selection = imageRectToWidget(normalizedSelection());
    constexpr int kMinTextEditorWidth = 96;
    constexpr int kMinTextEditorHeight = 38;
    const int spaceRight = qRound(selection.right() - topLeft.x() - 12);
    const int spaceLeft = qRound(topLeft.x() - selection.left() - 12);
    const int spaceBottom = qRound(selection.bottom() - topLeft.y() - 12);
    const int spaceTop = qRound(topLeft.y() - selection.top() - 12);
    const int availableRight = std::max(kMinTextEditorWidth, std::max(spaceRight, spaceLeft));
    const int availableBottom = std::max(kMinTextEditorHeight, std::max(spaceBottom, spaceTop));
    const int editorWidth = std::clamp(220, kMinTextEditorWidth, availableRight);
    const int editorHeight = std::clamp(m_textEditor->fontMetrics().height() + 18, kMinTextEditorHeight, availableBottom);
    QPoint editorTopLeft = topLeft.toPoint();
    if (spaceRight < kMinTextEditorWidth && spaceLeft > spaceRight) {
        editorTopLeft.setX(qRound(topLeft.x()) - editorWidth);
    }
    if (spaceBottom < kMinTextEditorHeight && spaceTop > spaceBottom) {
        editorTopLeft.setY(qRound(topLeft.y()) - editorHeight);
    }
    m_textEditor->setGeometry(keepEditorInsideWindow(QRect(editorTopLeft, QSize(editorWidth, editorHeight))));
    m_textEditor->ensureCursorVisible();
}

void ShotWindow::redoAnnotation()
{
    if (m_redoStack.isEmpty()) {
        return;
    }

    const HistorySnapshot current = currentHistorySnapshot();
    const HistorySnapshot next = m_redoStack.takeLast();
    m_undoStack.append(current);
    restoreHistorySnapshot(next);
}

void ShotWindow::updateToolbarState()
{
    if (!m_toolbar) {
        return;
    }

    const QString active = currentToolName();
    const QString scopeAction = markshot::ui::actionName(Action::ToggleCaptureScope);
    const QString layoutAction = markshot::ui::actionName(Action::ToggleToolbarLayout);
    const auto buttons = m_toolbar->findChildren<QPushButton *>();
    for (QPushButton *button : buttons) {
        const QString action = button->property("action").toString();
        const bool isActiveTool = action == active
            || (action == scopeAction && m_fullscreenAnnotation)
            || (action == layoutAction && m_toolbarVerticalLayout);
        button->setProperty("active", isActiveTool);
        button->style()->unpolish(button);
        button->style()->polish(button);
        button->update();
    }
}
