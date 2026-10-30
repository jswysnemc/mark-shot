#include "shot_window_module.h"

#include "settings/settings_dialog.h"
#include "ui/compact_toolbar.h"
#include "ui/icon_manifest.h"

namespace cfg = markshot::config;
namespace shortcuts = markshot::shortcut;
using namespace markshot::shot;

void ShotWindow::hideAnnotationPropertyPanels()
{
    if (m_controls.m_annotationPropertyPanel) {
        m_controls.m_annotationPropertyPanel->hide();
    }
    if (m_controls.m_propertyColorDialogPanel) {
        m_controls.m_propertyColorDialogPanel->hide();
    }
    if (m_controls.m_propertyFontPanel) {
        m_controls.m_propertyFontPanel->hide();
    }
}

void ShotWindow::hideTransientPanels()
{
    if (m_controls.m_openWithPanel) {
        m_controls.m_openWithPanel->hide();
    }
    if (m_controls.m_extensionPanel) {
        m_controls.m_extensionPanel->hide();
    }
    if (m_controls.m_colorPalette) {
        m_controls.m_colorPalette->hide();
    }
    hideShapeMarkerPopup();
    hideAnnotationPropertyPanels();
}

void ShotWindow::enterFullscreenAnnotation(bool resetAnnotations)
{
    commitTextEditor();
    emit selectionActivated(this);
    if (m_controls.m_colorPalette) {
        m_controls.m_colorPalette->hide();
    }

    if (!m_fullscreenAnnotation && hasUsableSelection()) {
        m_controls.m_selectionBeforeFullscreenAnnotation = normalizedSelection();
    }
    m_mode = Mode::Editing;
    attachSelectionPointer();
    m_dragging = false;
    m_fullscreenAnnotation = true;
    applyToolbarLayout();
    updateMinimumImageWindowSize();
    m_toolbarDragging = false;
    m_toolbarUserPlaced = false;
    m_actionToolbarUserPlaced = false;
    m_selection = QRectF(QPointF(0, 0), QSizeF(m_frozenFrame.size()));
    if (resetAnnotations) {
        m_annotations.clear();
        m_annotationHistory.clear();
        m_laserStrokes.clear();
        m_laserDraft.reset();
    }
    m_draft.reset();
    setSelectedAnnotations({});
    if (resetAnnotations) {
        m_nextNumber = 1;
        m_nextAnnotationId = 1;
    }
    if (m_controls.m_openWithPanel) {
        m_controls.m_openWithPanel->hide();
    }
    if (m_controls.m_extensionPanel) {
        m_controls.m_extensionPanel->hide();
    }
    if (m_controls.m_annotationPropertyPanel) {
        m_controls.m_annotationPropertyPanel->hide();
    }
    if (m_controls.m_propertyColorDialogPanel) {
        m_controls.m_propertyColorDialogPanel->hide();
    }
    if (m_controls.m_propertyFontPanel) {
        m_controls.m_propertyFontPanel->hide();
    }
    setTool(defaultEditingTool());
    if (m_controls.m_toolbar) {
        setFullscreenActionButtonsVisible(true);
        m_controls.m_toolbar->show();
    }
    updateMinimumImageWindowSize();
    if (m_controls.m_actionToolbar) {
        m_controls.m_actionToolbar->hide();
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
    m_actionToolbarUserPlaced = false;
    m_fullscreenAnnotation = false;
    restoreConfiguredToolbarLayout();
    m_selectionDrag = SelectionDrag::None;
    m_annotationDrag = SelectionDrag::None;
    m_annotationSelectionBoxActive = false;
    m_draft.reset();
    m_laserDraft.reset();

    if (m_controls.m_selectionBeforeFullscreenAnnotation.has_value()) {
        m_selection = *m_controls.m_selectionBeforeFullscreenAnnotation;
    } else {
        resetImageZoom();
        m_mode = Mode::Selecting;
        // 1. 【画布】【按需识别】从全图编辑返回选区时补齐候选，不削减编辑窗口的选区能力
        initializeRegionDetection();
        m_selection = {};
        if (m_controls.m_toolbar) {
            m_controls.m_toolbar->hide();
        }
        if (m_controls.m_actionToolbar) {
            m_controls.m_actionToolbar->hide();
        }
        setFullscreenActionButtonsVisible(false);
        updateToolbarState();
        update();
        return;
    }

    m_mode = Mode::Editing;
    attachSelectionPointer();
    setFullscreenActionButtonsVisible(false);
    if (m_controls.m_toolbar) {
        m_controls.m_toolbar->show();
    }
    if (m_controls.m_actionToolbar) {
        m_controls.m_actionToolbar->show();
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

/// @brief 【截图工具栏】【动作映射】向指定工具栏组件当前分组添加按钮，并连接业务动作
/// @param action 工具栏动作
/// @param shortcutText 提示中显示的快捷键文本
/// @param parentToolbar 目标工具栏；为空或为主工具栏时添加到主工具栏
/// @return 新建按钮，生命周期归目标工具栏
QPushButton *ShotWindow::addToolbarButton(Action action, const QString &shortcutText, QWidget *parentToolbar)
{
    // 1. 选择目标组件，并组装按钮描述
    const bool mainToolbarButton = !parentToolbar || parentToolbar == m_controls.m_toolbar;
    markshot::ui::CompactToolbar *toolbar = mainToolbarButton ? m_controls.m_toolbarPanel : m_controls.m_actionToolbarPanel;
    markshot::ui::ToolbarButtonSpec spec;
    spec.actionKey = markshot::ui::actionName(action);
    spec.icon = markshot::ui::makeToolIcon(action);
    spec.iconName = markshot::ui::toolIconName(action);
    spec.iconInk = action == Action::Save ? QColor(255, 255, 255) : QColor(229, 231, 235);
    spec.tooltip = QStringLiteral("%1 (%2)").arg(markshot::i18n::translate(spec.actionKey), shortcutText);
    if (action == Action::ScrollCapture && isGnomeWaylandSession() && !hasGnomeScrollHelper()) {
        spec.enabled = false;
        spec.tooltip = MS_TR("Scroll capture is not supported on GNOME Wayland.");
    }
    if (action == Action::Save) {
        spec.role = markshot::ui::ToolbarButtonRole::Primary;
    } else if (action == Action::Cancel) {
        spec.role = markshot::ui::ToolbarButtonRole::Danger;
    } else if (action == Action::Copy) {
        // 复制为高频输出，使用次级强调，与保存主动作区分层级
        spec.role = markshot::ui::ToolbarButtonRole::Accent;
    } else if (action == Action::OpenWith || action == Action::Extensions || action == Action::Pin || action == Action::OcrCopy || action == Action::ScrollCapture || action == Action::Upload || action == Action::Settings) {
        spec.role = markshot::ui::ToolbarButtonRole::Secondary;
    }
    QPushButton *button = toolbar->addButton(spec);
    // 2. 全屏标注时主工具栏的移动按钮兼作拖拽握柄
    if (mainToolbarButton && action == Action::ToolMove) {
        button->installEventFilter(this);
    }

    // 3. 连接业务动作

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
    } else if (action == Action::ToolMarker) {
        connect(button, &QPushButton::clicked, this, [this] {
            // 1. 主按钮点击选中形状标记工具
            // 2. 再次点击已激活的按钮时展开形状面板
            if (m_tool == Tool::Marker) {
                toggleShapeMarkerPopup();
            } else {
                setTool(Tool::Marker);
                hideShapeMarkerPopup();
            }
        });
        m_controls.m_shapeMarkerToolbarButton = button;
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
    } else if (action == Action::Translate) {
        connect(button, &QPushButton::clicked, this, [this] { translateSelection(); });
    } else if (action == Action::Copy) {
        connect(button, &QPushButton::clicked, this, [this] { copySelection(); });
    } else if (action == Action::Save) {
        connect(button, &QPushButton::clicked, this, [this] { saveSelectionAs(); });
    } else if (action == Action::Upload) {
        connect(button, &QPushButton::clicked, this, [this] { uploadSelection(); });
    } else if (action == Action::Settings) {
        connect(button, &QPushButton::clicked, this, [this] { openSettingsAfterClosingCapture(); });
    } else if (action == Action::Cancel) {
        connect(button, &QPushButton::clicked, this, [this] { close(); });
    }

    return button;
}

void ShotWindow::openSettingsAfterClosingCapture()
{
    // 1. 下一轮事件循环打开设置，确保冻结窗口先完成关闭
    QTimer::singleShot(0, [] { markshot::settings::showSettingsDialog(); });
    // 2. 通知同一截图会话关闭所有冻结窗口；无会话管理器时关闭当前窗口
    if (receivers(SIGNAL(sessionCancelRequested())) > 0) {
        emit sessionCancelRequested();
    } else {
        close();
    }
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

/// @brief 分发工具栏抓取和内联编辑器输入，维护控件自身的交互状态
/// @param watched 当前接收事件的控件
/// @param event 控件事件
/// @return 当前事件已由窗口处理时返回 true
bool ShotWindow::eventFilter(QObject *watched, QEvent *event)
{
    // 【截图】【光标事件】设置光标会同步发出 CursorChange，不能再次进入握柄更新
    if (event->type() == QEvent::CursorChange) {
        return false;
    }
    if (event->type() == QEvent::MouseButtonPress || event->type() == QEvent::KeyPress) {
        clearWheelPreview();
    }

    const bool isMoveButton = watched->property("action").toString() == markshot::ui::actionName(Action::ToolMove);
    const bool isFullscreenMoveButton = m_fullscreenAnnotation && isMoveButton;
    if (isMoveButton && !m_fullscreenAnnotation && watched->property("dragHandle").toBool()) {
        watched->setProperty("dragHandle", false);
        if (auto *button = qobject_cast<QWidget *>(watched)) {
            button->setCursor(button->isEnabled() ? Qt::PointingHandCursor : Qt::ArrowCursor);
        }
    }
    // 1. 【截图工具栏】【拖拽把手】握柄由组件持有，窗口按实例识别并负责拖拽吸附
    const bool isToolbarGrip = m_controls.m_toolbarPanel && watched == m_controls.m_toolbarPanel->grip();
    const bool isActionToolbarGrip = m_controls.m_actionToolbarPanel && watched == m_controls.m_actionToolbarPanel->grip();
    if (isFullscreenMoveButton || isToolbarGrip || isActionToolbarGrip) {
        auto *handle = qobject_cast<QWidget *>(watched);
        if (handle) {
            handle->setProperty("dragHandle", true);
            const bool interrupted = event->type() == QEvent::Hide || event->type() == QEvent::UngrabMouse
                || !handle->isEnabled()
                || (event->type() == QEvent::MouseMove
                    && !(static_cast<QMouseEvent *>(event)->buttons() & Qt::LeftButton));
            if (m_toolbarDragging && interrupted) {
                m_toolbarDragging = false;
                m_dragging = false;
                updateCursor();
            }
            if (!handle->isEnabled()) {
                handle->setCursor(Qt::ArrowCursor);
                return false;
            }
            handle->setCursor(m_toolbarDragging ? Qt::ClosedHandCursor : Qt::OpenHandCursor);
        }
        QWidget *targetToolbar = isActionToolbarGrip ? m_controls.m_actionToolbar : m_controls.m_toolbar;
        if (event->type() == QEvent::MouseButtonPress) {
            auto *mouseEvent = static_cast<QMouseEvent *>(event);
            if (mouseEvent->button() == Qt::LeftButton) {
                auto *eventWidget = qobject_cast<QWidget *>(watched);
                if (!eventWidget) {
                    return false;
                }
                ++m_pointerInteractionSerial;
                m_dragging = true;
                m_toolbarDragging = true;
                m_controls.m_toolbarDragStart = eventWidget->mapTo(this, mouseEvent->pos());
                m_controls.m_toolbarBeforeDrag = targetToolbar->geometry();
                eventWidget->setCursor(Qt::ClosedHandCursor);
                updateCursor();
                return true;
            }
        } else if (event->type() == QEvent::MouseMove && m_toolbarDragging) {
            auto *mouseEvent = static_cast<QMouseEvent *>(event);
            auto *eventWidget = qobject_cast<QWidget *>(watched);
            if (!eventWidget) {
                return false;
            }
            const QPoint delta = eventWidget->mapTo(this, mouseEvent->pos()) - m_controls.m_toolbarDragStart;
            if (targetToolbar == m_controls.m_toolbar) {
                m_toolbarUserPlaced = true;
            }
            if (targetToolbar == m_controls.m_actionToolbar) {
                m_actionToolbarUserPlaced = true;
            }
            targetToolbar->setGeometry(clampedToolbarGeometry(m_controls.m_toolbarBeforeDrag.translated(delta)));
            updateOpenWithPanelGeometry();
            updateExtensionPanelGeometry();
            updateAnnotationPropertyPanelGeometry();
            return true;
        } else if (event->type() == QEvent::MouseButtonRelease && m_toolbarDragging) {
            auto *mouseEvent = static_cast<QMouseEvent *>(event);
            if (mouseEvent->button() == Qt::LeftButton) {
                m_dragging = false;
                m_toolbarDragging = false;
                if (handle) {
                    handle->setCursor(Qt::OpenHandCursor);
                }
                updateCursor();
                updateOpenWithPanelGeometry();
                updateExtensionPanelGeometry();
                updateAnnotationPropertyPanelGeometry();
                return true;
            }
        }
    }

    if (m_textEditor
        && watched == m_textEditor->viewport()
        && event->type() == QEvent::MouseButtonPress) {
        auto *mouseEvent = static_cast<QMouseEvent *>(event);
        if (mouseEvent->button() == Qt::RightButton) {
            showTextEditorContextMenu(m_textEditor->viewport()->mapToGlobal(mouseEvent->pos()));
            return true;
        }
    }

    if (m_textEditor && m_textEditor->isVisible()
        && (watched == m_textEditor || watched == m_textEditor->viewport())
        && event->type() == QEvent::ShortcutOverride
        && static_cast<QKeyEvent *>(event)->key() == Qt::Key_Escape) {
        // 1. 【标注】【文字编辑】保留取消编辑按键，避免窗口快捷键提前消费 Escape
        event->accept();
        return true;
    }

    if (handleSnowTextEditorEventIfEnabled(watched, event) || handleTextEditorActionShortcut(watched, event)) {
        return true;
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
            updateLayerShellForIme();
            updateAnnotationPropertyPanel();
            updateCursor();
            update();
            return true;
        }
    }

    if (watched == m_controls.m_colorPalette && event->type() == QEvent::MouseButtonPress) {
        m_controls.m_colorPalette->hide();
        update();
        return true;
    }

    return QWidget::eventFilter(watched, event);
}

/// @brief 【截图工具栏】【全屏分组】切换主工具栏中全屏专用分组的显示
/// @param visible 是否显示
/// @return 无返回值
void ShotWindow::setFullscreenActionButtonsVisible(bool visible)
{
    if (!m_controls.m_toolbarPanel) {
        return;
    }
    for (int group : std::as_const(m_controls.m_fullscreenActionGroups)) {
        m_controls.m_toolbarPanel->setGroupVisible(group, visible);
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

    if (recordingModeForStartupTool(m_startupTool).has_value()
        && !recordingModeForStartupTool(tool).has_value()) {
        m_pendingRecordingOptions.reset();
    }
    m_startupTool = tool;
    m_dragging = false;
    m_hoveredWindowRect.reset();
    hideDisplayCapturePicker();
    m_startupHoverValid = false;
    m_startupRulerDragging = false;
    m_startupRulers.clear();
    m_startupHintAnchor = markshot::startup_hint::PanelAnchor::BottomLeft;
    if (m_controls.m_startupColorPanel) {
        m_controls.m_startupColorPanel->hide();
    }
    updateCursor();
    update();
    routeStartupToolToCursorScreenIfEnabled();
}

void ShotWindow::leaveStartupTool()
{
    if (recordingModeForStartupTool(m_startupTool).has_value()) {
        m_pendingRecordingOptions.reset();
    }
    m_startupTool = StartupTool::None;
    hideDisplayCapturePicker();
    m_startupHoverValid = false;
    m_startupRulerDragging = false;
    m_startupRulers.clear();
    m_dragging = false;
    m_startupHintAnchor = markshot::startup_hint::PanelAnchor::BottomLeft;
    if (m_controls.m_startupColorPanel) {
        m_controls.m_startupColorPanel->hide();
    }
    updateCursor();
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
