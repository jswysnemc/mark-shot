#include "shot_window_module.h"

namespace cfg = markshot::config;
namespace shortcuts = markshot::shortcut;
using namespace markshot::shot;

void ShotWindow::hideAnnotationPropertyPanels()
{
    if (m_annotationPropertyPanel) {
        m_annotationPropertyPanel->hide();
    }
    if (m_propertyColorDialogPanel) {
        m_propertyColorDialogPanel->hide();
    }
    if (m_propertyFontPanel) {
        m_propertyFontPanel->hide();
    }
}

void ShotWindow::hideTransientPanels()
{
    if (m_openWithPanel) {
        m_openWithPanel->hide();
    }
    if (m_extensionPanel) {
        m_extensionPanel->hide();
    }
    if (m_colorPalette) {
        m_colorPalette->hide();
    }
    hideAnnotationPropertyPanels();
}

void ShotWindow::enterFullscreenAnnotation(bool resetAnnotations)
{
    commitTextEditor();
    emit selectionActivated(this);
    if (m_colorPalette) {
        m_colorPalette->hide();
    }

    if (!m_fullscreenAnnotation && hasUsableSelection()) {
        m_selectionBeforeFullscreenAnnotation = normalizedSelection();
    }
    m_mode = Mode::Editing;
    m_dragging = false;
    m_fullscreenAnnotation = true;
    applyToolbarLayout();
    updateMinimumImageWindowSize();
    m_toolbarDragging = false;
    m_toolbarUserPlaced = false;
    m_selection = QRectF(QPointF(0, 0), QSizeF(m_frozenFrame.size()));
    if (resetAnnotations) {
        m_annotations.clear();
        m_undoStack.clear();
        m_redoStack.clear();
        m_laserStrokes.clear();
        m_laserDraft.reset();
    }
    m_draft.reset();
    setSelectedAnnotations({});
    if (resetAnnotations) {
        m_nextNumber = 1;
        m_nextAnnotationId = 1;
    }
    if (m_openWithPanel) {
        m_openWithPanel->hide();
    }
    if (m_extensionPanel) {
        m_extensionPanel->hide();
    }
    if (m_annotationPropertyPanel) {
        m_annotationPropertyPanel->hide();
    }
    if (m_propertyColorDialogPanel) {
        m_propertyColorDialogPanel->hide();
    }
    if (m_propertyFontPanel) {
        m_propertyFontPanel->hide();
    }
    setTool(defaultEditingTool());
    if (m_toolbar) {
        setFullscreenActionButtonsVisible(true);
        m_toolbar->show();
    }
    updateMinimumImageWindowSize();
    if (m_actionToolbar) {
        m_actionToolbar->hide();
    }
    updateToolbarGeometry();
    updateToolbarState();
    update();
}

void ShotWindow::leaveFullscreenAnnotation()
{
    commitTextEditor();
    m_dragging = false;
    m_toolbarDragging = false;
    m_toolbarUserPlaced = false;
    m_fullscreenAnnotation = false;
    m_toolbarVerticalLayout = false;
    applyToolbarLayout();
    m_selectionDrag = SelectionDrag::None;
    m_annotationDrag = SelectionDrag::None;
    m_annotationSelectionBoxActive = false;
    m_draft.reset();
    m_laserDraft.reset();

    if (m_selectionBeforeFullscreenAnnotation.has_value()) {
        m_selection = *m_selectionBeforeFullscreenAnnotation;
    } else {
        resetImageZoom();
        m_mode = Mode::Selecting;
        m_selection = {};
        if (m_toolbar) {
            m_toolbar->hide();
        }
        if (m_actionToolbar) {
            m_actionToolbar->hide();
        }
        setFullscreenActionButtonsVisible(false);
        updateToolbarState();
        update();
        return;
    }

    m_mode = Mode::Editing;
    setFullscreenActionButtonsVisible(false);
    if (m_toolbar) {
        m_toolbar->show();
    }
    if (m_actionToolbar) {
        m_actionToolbar->show();
    }
    updateToolbarGeometry();
    updateActionToolbarGeometry();
    updateOpenWithPanelGeometry();
    updateExtensionPanelGeometry();
    updateAnnotationPropertyPanelGeometry();
    updateToolbarState();
    update();
}

void ShotWindow::toggleCaptureScope()
{
    resetImageZoom();
    if (m_fullscreenAnnotation) {
        leaveFullscreenAnnotation();
    } else {
        enterFullscreenAnnotation(false);
    }
}

void ShotWindow::toggleToolbarLayout()
{
    m_toolbarVerticalLayout = !m_toolbarVerticalLayout;
    m_toolbarUserPlaced = false;
    applyToolbarLayout();
    updateToolbarGeometry();
    updateOpenWithPanelGeometry();
    updateExtensionPanelGeometry();
    updateAnnotationPropertyPanelGeometry();
    updateToolbarState();
}

void ShotWindow::applyToolbarLayout()
{
    if (!m_toolbarLayout) {
        return;
    }

    m_toolbarLayout->setDirection(m_toolbarVerticalLayout ? QBoxLayout::TopToBottom : QBoxLayout::LeftToRight);
    m_toolbar->setFixedSize(QWIDGETSIZE_MAX, QWIDGETSIZE_MAX);
    m_toolbar->adjustSize();
}

QPushButton *ShotWindow::addToolbarButton(Action action, const QString &shortcutText, QWidget *parentToolbar)
{
    QWidget *toolbar = parentToolbar ? parentToolbar : m_toolbar;
    auto *button = new QPushButton(toolbar);
    button->setIcon(markshot::ui::makeToolIcon(action));
    const bool actionToolbarButton = parentToolbar && parentToolbar == m_actionToolbar;
    const int iconSize = actionToolbarButton
        ? m_toolbarAppearance.actionToolbarIconSize
        : m_toolbarAppearance.toolbarIconSize;
    button->setIconSize(QSize(iconSize, iconSize));
    button->setFocusPolicy(Qt::NoFocus);
    button->setToolTip(QStringLiteral("%1 (%2)").arg(markshot::i18n::translate(markshot::ui::actionName(action)), shortcutText));
    button->setProperty("action", markshot::ui::actionName(action));
    if (action == Action::ScrollCapture && isGnomeWaylandSession() && !hasGnomeScrollHelper()) {
        button->setEnabled(false);
        button->setToolTip(MS_TR("Scroll capture is not supported on GNOME Wayland."));
    }
    if (!parentToolbar && action == Action::ToolMove) {
        button->installEventFilter(this);
    }
    if (action == Action::Save) {
        button->setProperty("role", QStringLiteral("primary"));
    } else if (action == Action::Cancel) {
        button->setProperty("role", QStringLiteral("danger"));
    } else if (action == Action::OpenWith || action == Action::Extensions || action == Action::Pin || action == Action::OcrCopy || action == Action::Copy || action == Action::ScrollCapture || action == Action::Upload) {
        button->setProperty("role", QStringLiteral("secondary"));
    }

    if (action == Action::ToolMove) {
        connect(button, &QPushButton::clicked, this, [this] { setTool(Tool::Move); });
    } else if (action == Action::ToolSelect) {
        connect(button, &QPushButton::clicked, this, [this] { setTool(Tool::Select); });
    } else if (action == Action::ToolPen) {
        connect(button, &QPushButton::clicked, this, [this] { setTool(Tool::Pen); });
    } else if (action == Action::ToolLine) {
        connect(button, &QPushButton::clicked, this, [this] { setTool(Tool::Line); });
    } else if (action == Action::ToolHighlighter) {
        connect(button, &QPushButton::clicked, this, [this] { setTool(Tool::Highlighter); });
    } else if (action == Action::ToolRectangle) {
        connect(button, &QPushButton::clicked, this, [this] { setTool(Tool::Rectangle); });
    } else if (action == Action::ToolEllipse) {
        connect(button, &QPushButton::clicked, this, [this] { setTool(Tool::Ellipse); });
    } else if (action == Action::ToolArrow) {
        connect(button, &QPushButton::clicked, this, [this] { setTool(Tool::Arrow); });
    } else if (action == Action::ToolText) {
        connect(button, &QPushButton::clicked, this, [this] { setTool(Tool::Text); });
    } else if (action == Action::ToolNumber) {
        connect(button, &QPushButton::clicked, this, [this] { setTool(Tool::Number); });
    } else if (action == Action::ToolMosaic) {
        connect(button, &QPushButton::clicked, this, [this] { setTool(Tool::Mosaic); });
    } else if (action == Action::ToolMagnifier) {
        connect(button, &QPushButton::clicked, this, [this] { setTool(Tool::Magnifier); });
    } else if (action == Action::ToolLaser) {
        connect(button, &QPushButton::clicked, this, [this] { setTool(Tool::Laser); });
    } else if (action == Action::ToggleCaptureScope) {
        connect(button, &QPushButton::clicked, this, [this] { toggleCaptureScope(); });
    } else if (action == Action::ToggleToolbarLayout) {
        connect(button, &QPushButton::clicked, this, [this] { toggleToolbarLayout(); });
    } else if (action == Action::Clear) {
        connect(button, &QPushButton::clicked, this, [this] { clearAnnotations(); });
    } else if (action == Action::Undo) {
        connect(button, &QPushButton::clicked, this, [this] { undoAnnotationEdit(); });
    } else if (action == Action::Redo) {
        connect(button, &QPushButton::clicked, this, [this] { redoAnnotation(); });
    } else if (action == Action::OpenWith) {
        connect(button, &QPushButton::clicked, this, [this] { toggleOpenWithPanel(); });
    } else if (action == Action::Extensions) {
        connect(button, &QPushButton::clicked, this, [this] { toggleExtensionPanel(); });
    } else if (action == Action::Pin) {
        connect(button, &QPushButton::clicked, this, [this] { pinSelection(); });
    } else if (action == Action::ScrollCapture) {
        connect(button, &QPushButton::clicked, this, [this] { startScrollCapture(); });
    } else if (action == Action::OcrCopy) {
        connect(button, &QPushButton::clicked, this, [this] { ocrCopySelection(); });
    } else if (action == Action::Copy) {
        connect(button, &QPushButton::clicked, this, [this] { copySelection(); });
    } else if (action == Action::Save) {
        connect(button, &QPushButton::clicked, this, [this] { saveSelectionAs(); });
    } else if (action == Action::Upload) {
        connect(button, &QPushButton::clicked, this, [this] { uploadSelection(); });
    } else if (action == Action::Cancel) {
        connect(button, &QPushButton::clicked, this, [this] { close(); });
    }

    return button;
}

QVector<ShotWindow::DesktopApp> ShotWindow::imageDesktopApps() const
{
    QVector<DesktopApp> apps;
    QStringList seenPaths;

    for (const QString &appDir : desktopSearchDirs()) {
        if (!QDir(appDir).exists()) {
            continue;
        }

        QDirIterator iterator(appDir, {QStringLiteral("*.desktop")}, QDir::Files, QDirIterator::Subdirectories);
        while (iterator.hasNext()) {
            const QString desktopPath = iterator.next();
            if (seenPaths.contains(desktopPath)) {
                continue;
            }
            seenPaths.append(desktopPath);

            QFile file(desktopPath);
            if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) {
                continue;
            }

            const QStringList lines = QString::fromUtf8(file.readAll()).split(QLatin1Char('\n'));
            if (desktopEntryValue(lines, QStringLiteral("Type")) != QStringLiteral("Application")) {
                continue;
            }
            if (desktopEntryBool(lines, QStringLiteral("Hidden"))
                || desktopEntryBool(lines, QStringLiteral("NoDisplay"))
                || !desktopEntrySupportsImage(lines)) {
                continue;
            }

            const QString exec = desktopEntryValue(lines, QStringLiteral("Exec"));
            const QString name = desktopEntryValue(lines, QStringLiteral("Name"));
            const QString icon = desktopEntryValue(lines, QStringLiteral("Icon"));
            if (exec.isEmpty() || name.isEmpty()) {
                continue;
            }

            apps.append({name, desktopPath, exec, icon});
        }
    }

    std::sort(apps.begin(), apps.end(), [](const DesktopApp &left, const DesktopApp &right) {
        return QString::localeAwareCompare(left.name, right.name) < 0;
    });
    return apps;
}

QVector<ShotWindow::ExtensionCommand> ShotWindow::extensionCommands(QString *errorMessage) const
{
    if (errorMessage) {
        errorMessage->clear();
    }

    const QString configPath = extensionCommandsConfigPath();
    QFile file(configPath);
    if (!file.exists()) {
        return {};
    }
    if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        if (errorMessage) {
            *errorMessage = QStringLiteral("Cannot read %1").arg(configPath);
        }
        return {};
    }

    QJsonParseError parseError;
    const QJsonDocument document = QJsonDocument::fromJson(file.readAll(), &parseError);
    if (parseError.error != QJsonParseError::NoError) {
        if (errorMessage) {
            *errorMessage = QStringLiteral("Invalid JSON at offset %1: %2").arg(parseError.offset).arg(parseError.errorString());
        }
        return {};
    }

    QJsonArray commandArray;
    if (document.isArray()) {
        commandArray = document.array();
    } else if (document.isObject()) {
        const QJsonObject root = document.object();
        if (root.value(QStringLiteral("commands")).isArray()) {
            commandArray = root.value(QStringLiteral("commands")).toArray();
        } else if (root.value(QStringLiteral("command")).isString()) {
            commandArray.append(root);
        }
    } else {
        if (errorMessage) {
            *errorMessage = QStringLiteral("Expected a JSON array, a command object, or an object with a commands array");
        }
        return {};
    }

    if (commandArray.isEmpty()) {
        if (errorMessage) {
            *errorMessage = QStringLiteral("No extension commands found");
        }
        return {};
    }

    QVector<ExtensionCommand> commands;
    for (const QJsonValue &value : commandArray) {
        if (!value.isObject()) {
            continue;
        }

        const QJsonObject object = value.toObject();
        ExtensionCommand command;
        command.name = object.value(QStringLiteral("name")).toString().trimmed();
        command.command = object.value(QStringLiteral("command")).toString().trimmed();
        command.workingDirectory = object.value(QStringLiteral("workingDirectory"))
                                       .toString(object.value(QStringLiteral("cwd")).toString())
                                       .trimmed();
        command.description = object.value(QStringLiteral("description")).toString().trimmed();
        command.saveImage = extensionCommandUsesImagePlaceholder(command.command)
            || object.value(QStringLiteral("saveImage")).toBool(false)
            || object.value(QStringLiteral("needsImage")).toBool(false);
        if (object.value(QStringLiteral("closeOnStart")).isBool()) {
            command.closeOnStart = object.value(QStringLiteral("closeOnStart")).toBool();
        }

        if (command.name.isEmpty() || command.command.isEmpty()) {
            continue;
        }
        commands.append(command);
    }

    return commands;
}

bool ShotWindow::eventFilter(QObject *watched, QEvent *event)
{
    if (event->type() == QEvent::MouseButtonPress || event->type() == QEvent::KeyPress) {
        clearWheelPreview();
    }

    const bool isFullscreenMoveButton = m_fullscreenAnnotation
        && watched->property("action").toString() == markshot::ui::actionName(Action::ToolMove);
    if (isFullscreenMoveButton) {
        if (event->type() == QEvent::MouseButtonPress) {
            auto *mouseEvent = static_cast<QMouseEvent *>(event);
            if (mouseEvent->button() == Qt::LeftButton) {
                auto *eventWidget = qobject_cast<QWidget *>(watched);
                if (!eventWidget) {
                    return false;
                }
                m_dragging = true;
                m_toolbarDragging = true;
                m_toolbarDragStart = eventWidget->mapTo(this, mouseEvent->pos());
                m_toolbarBeforeDrag = m_toolbar->geometry();
                setCursor(Qt::SizeAllCursor);
                return true;
            }
        } else if (event->type() == QEvent::MouseMove && m_toolbarDragging) {
            auto *mouseEvent = static_cast<QMouseEvent *>(event);
            auto *eventWidget = qobject_cast<QWidget *>(watched);
            if (!eventWidget) {
                return false;
            }
            const QPoint delta = eventWidget->mapTo(this, mouseEvent->pos()) - m_toolbarDragStart;
            m_toolbarUserPlaced = true;
            m_toolbar->setGeometry(clampedToolbarGeometry(m_toolbarBeforeDrag.translated(delta)));
            updateOpenWithPanelGeometry();
            updateExtensionPanelGeometry();
            updateAnnotationPropertyPanelGeometry();
            return true;
        } else if (event->type() == QEvent::MouseButtonRelease && m_toolbarDragging) {
            auto *mouseEvent = static_cast<QMouseEvent *>(event);
            if (mouseEvent->button() == Qt::LeftButton) {
                m_dragging = false;
                m_toolbarDragging = false;
                updateCursor();
                updateOpenWithPanelGeometry();
                updateExtensionPanelGeometry();
                updateAnnotationPropertyPanelGeometry();
                return true;
            }
        }
    }

    if (watched == m_textEditor && event->type() == QEvent::KeyPress) {
        auto *keyEvent = static_cast<QKeyEvent *>(event);
        if (imageNavigationAvailable() && keyEvent->key() == Qt::Key_Control && !keyEvent->isAutoRepeat()) {
            if (m_ctrlTapTimer.isValid() && m_ctrlTapTimer.elapsed() <= kCtrlDoubleTapMs) {
                resetImageZoom();
                m_ctrlTapTimer.invalidate();
            } else {
                m_ctrlTapTimer.restart();
            }
            return true;
        }
        if (keyEvent->key() == Qt::Key_Escape) {
            m_draft.reset();
            m_editingTextAnnotationId.reset();
            m_textEditor->hide();
            m_textEditor->clear();
            setFocus(Qt::OtherFocusReason);
            update();
            return true;
        }
    }

    if (watched == m_colorPalette && event->type() == QEvent::MouseButtonPress) {
        m_colorPalette->hide();
        update();
        return true;
    }

    return QWidget::eventFilter(watched, event);
}

void ShotWindow::setFullscreenActionButtonsVisible(bool visible)
{
    for (QPushButton *button : std::as_const(m_fullscreenActionButtons)) {
        if (button) {
            button->setVisible(visible);
        }
    }
}

void ShotWindow::setStartupTool(StartupTool tool)
{
    if (m_mode != Mode::Selecting) {
        return;
    }

    if (m_startupTool == tool) {
        leaveStartupTool();
        return;
    }

    m_startupTool = tool;
    m_dragging = false;
    m_hoveredWindowRect.reset();
    hideDisplayCapturePicker();
    m_startupHoverValid = false;
    m_startupRulerDragging = false;
    m_startupRulerHasMeasure = false;
    if (m_startupColorPanel) {
        m_startupColorPanel->hide();
    }
    setCursor(tool == StartupTool::Ruler ? QCursor(Qt::SizeAllCursor) : captureCrossCursor());
    update();
}

void ShotWindow::leaveStartupTool()
{
    m_startupTool = StartupTool::None;
    hideDisplayCapturePicker();
    m_startupHoverValid = false;
    m_startupRulerDragging = false;
    m_startupRulerHasMeasure = false;
    m_dragging = false;
    if (m_startupColorPanel) {
        m_startupColorPanel->hide();
    }
    setCursor(captureCrossCursor());
    update();
}

QColor ShotWindow::sampledImageColor(QPointF imagePoint) const
{
    if (m_frozenFrame.isNull()) {
        return {};
    }

    const QPointF clamped = clampImagePoint(imagePoint);
    const int x = std::clamp(qRound(clamped.x()), 0, std::max(0, m_frozenFrame.width() - 1));
    const int y = std::clamp(qRound(clamped.y()), 0, std::max(0, m_frozenFrame.height() - 1));
    return m_frozenFrame.pixelColor(x, y);
}

void ShotWindow::showStartupColorDialog(QColor color, QPoint anchor)
{
    if (!color.isValid()) {
        return;
    }

    if (m_startupColorPanel) {
        m_startupColorPanel->deleteLater();
        m_startupColorPanel = nullptr;
    }

    m_startupColorPanel = new QWidget(this);
    m_startupColorPanel->setObjectName(QStringLiteral("startupColorInspector"));
    m_startupColorPanel->setAttribute(Qt::WA_DeleteOnClose, true);
    m_startupColorPanel->setStyleSheet(QStringLiteral(
        "QWidget#startupColorInspector {"
        " background: rgba(229, 231, 235, 238);"
        " border: 1px solid rgba(15, 23, 42, 55);"
        " border-radius: 16px;"
        "}"
        "QLabel { color: #172033; font-size: 12px; }"
        "QLabel#formatName { font-weight: 700; color: #475569; min-width: 76px; }"
        "QLabel#formatValue {"
        " font-family: %1;"
        " font-weight: 700;"
        " color: #172033;"
        "}"
        "QFrame#formatRow {"
        " background: rgba(248, 250, 252, 228);"
        " border-radius: 9px;"
        "}"
        "QPushButton {"
        " background: rgba(148, 163, 184, 70);"
        " border: 0;"
        " border-radius: 7px;"
        " padding: 5px 9px;"
        " color: #334155;"
        " font-weight: 700;"
        "}"
        "QPushButton:hover { background: rgba(45, 212, 191, 150); color: #042F2E; }")
            .arg(markshot::theme::monospaceFontFamilyCss()));

    auto *layout = new QVBoxLayout(m_startupColorPanel);
    layout->setContentsMargins(12, 12, 12, 12);
    layout->setSpacing(9);

    const QString hex = colorHexRgb(color);
    auto *preview = new QLabel(hex, m_startupColorPanel);
    preview->setAlignment(Qt::AlignCenter);
    preview->setMinimumSize(300, 88);
    preview->setStyleSheet(QStringLiteral(
        "QLabel {"
        " background: %1;"
        " border-radius: 13px;"
        " color: %2;"
        " font-size: 20px;"
        " font-weight: 800;"
        "}").arg(hex, propertyIconInkForFill(color).name()));
    layout->addWidget(preview);

    /// @brief Struct representing a row of formatted color information.
    struct FormatRow {
        /// @brief The name of the color format.
        QString name;
        /// @brief The formatted value string.
        QString value;
    };
    const QVector<FormatRow> rows = {
        {QStringLiteral("HEX"), hex},
        {QStringLiteral("hex lower"), hex.toLower()},
        {QStringLiteral("RGBA hex"), colorHexRgba(color)},
        {QStringLiteral("RGB"), QStringLiteral("rgb(%1, %2, %3)").arg(color.red()).arg(color.green()).arg(color.blue())},
        {QStringLiteral("RGBA"), QStringLiteral("rgba(%1, %2, %3, %4)").arg(color.red()).arg(color.green()).arg(color.blue()).arg(alphaText(color))},
        {QStringLiteral("HSL"), QStringLiteral("hsl(%1, %2%, %3%)")
             .arg(colorHueOrZero(color.hslHue()))
             .arg(qRound(color.hslSaturationF() * 100.0))
             .arg(qRound(color.lightnessF() * 100.0))},
        {QStringLiteral("HSV"), QStringLiteral("hsv(%1, %2%, %3%)")
             .arg(colorHueOrZero(color.hsvHue()))
             .arg(qRound(color.hsvSaturationF() * 100.0))
             .arg(qRound(color.valueF() * 100.0))},
        {QStringLiteral("Qt"), QStringLiteral("Qt.rgba(%1, %2, %3, %4)")
             .arg(normalizedColorChannel(color.red()),
                  normalizedColorChannel(color.green()),
                  normalizedColorChannel(color.blue()),
                  alphaText(color))},
    };

    for (const FormatRow &row : rows) {
        auto *frame = new QFrame(m_startupColorPanel);
        frame->setObjectName(QStringLiteral("formatRow"));
        auto *rowLayout = new QHBoxLayout(frame);
        rowLayout->setContentsMargins(10, 7, 8, 7);
        rowLayout->setSpacing(8);

        auto *nameLabel = new QLabel(row.name, frame);
        nameLabel->setObjectName(QStringLiteral("formatName"));
        rowLayout->addWidget(nameLabel);

        auto *valueLabel = new QLabel(row.value, frame);
        valueLabel->setObjectName(QStringLiteral("formatValue"));
        valueLabel->setTextInteractionFlags(Qt::TextSelectableByMouse);
        rowLayout->addWidget(valueLabel, 1);

        auto *copyButton = new QPushButton(MS_TR("Copy"), frame);
        copyButton->setFocusPolicy(Qt::NoFocus);
        connect(copyButton, &QPushButton::clicked, this, [this, value = row.value] {
            if (!markshot::copyTextToClipboard(value)) {
                return;
            }
            QTimer::singleShot(180, this, [this] {
                emit sessionCancelRequested();
                close();
            });
        });
        rowLayout->addWidget(copyButton);
        layout->addWidget(frame);
    }

    m_startupColorPanel->adjustSize();
    const QSize panelSize = m_startupColorPanel->sizeHint();
    int x = anchor.x() + 22;
    int y = anchor.y() + 22;
    if (x + panelSize.width() > width() - 12) {
        x = anchor.x() - panelSize.width() - 22;
    }
    if (y + panelSize.height() > height() - 12) {
        y = anchor.y() - panelSize.height() - 22;
    }
    x = std::clamp(x, 12, std::max(12, width() - panelSize.width() - 12));
    y = std::clamp(y, 12, std::max(12, height() - panelSize.height() - 12));
    m_startupColorPanel->setGeometry(QRect(QPoint(x, y), panelSize));
    m_startupColorPanel->show();
    m_startupColorPanel->raise();
}

void ShotWindow::drawStartupColorLoupe(QPainter &painter, QPointF imagePoint) const
{
    if (!m_startupHoverValid || m_frozenFrame.isNull()) {
        return;
    }

    const QColor color = sampledImageColor(imagePoint);
    const QPointF widgetPoint = imageToWidget(imagePoint);
    QRectF loupe(widgetPoint.x() + 22.0,
                 widgetPoint.y() + 22.0,
                 m_startupColorLoupeSize,
                 m_startupColorLoupeSize);
    if (loupe.right() > width() - 12.0) {
        loupe.moveRight(widgetPoint.x() - 22.0);
    }
    if (loupe.bottom() > height() - 12.0) {
        loupe.moveBottom(widgetPoint.y() - 22.0);
    }
    loupe.moveLeft(std::clamp(loupe.left(), 12.0, std::max(12.0, width() - loupe.width() - 12.0)));
    loupe.moveTop(std::clamp(loupe.top(), 12.0, std::max(12.0, height() - loupe.height() - 12.0)));

    const QPoint center = clampImagePoint(imagePoint).toPoint();
    const QRect sourceCandidate(center.x() - 6, center.y() - 6, 13, 13);
    const QRect sourceRect = sourceCandidate.intersected(QRect(QPoint(0, 0), m_frozenFrame.size()));

    painter.save();
    QPainterPath clip;
    clip.addEllipse(loupe);
    painter.setClipPath(clip);
    painter.setRenderHint(QPainter::SmoothPixmapTransform, false);
    painter.drawImage(loupe, m_frozenFrame.copy(sourceRect));
    painter.setClipping(false);
    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.setPen(QPen(QColor(59, 40, 46), 3.0));
    painter.setBrush(Qt::NoBrush);
    painter.drawEllipse(loupe);

    const QPointF c = loupe.center();
    painter.setPen(QPen(QColor(96, 165, 250), 2.0));
    painter.drawLine(QPointF(c.x() - 12.0, c.y()), QPointF(c.x() + 12.0, c.y()));
    painter.drawLine(QPointF(c.x(), c.y() - 12.0), QPointF(c.x(), c.y() + 12.0));

    painter.setFont(markshot::theme::uiFont(11, QFont::DemiBold));
    const QString hex = colorHexRgb(color);
    const QFontMetrics metrics(painter.font());
    const QRectF label(loupe.center().x() - (metrics.horizontalAdvance(hex) + 20.0) / 2.0,
                       loupe.bottom() - metrics.height() - 11.0,
                       metrics.horizontalAdvance(hex) + 20.0,
                       metrics.height() + 7.0);
    drawRoundedLabel(painter, label, hex, QColor(8, 13, 19, 210));
    painter.restore();
}
