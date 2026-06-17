#include "shot_window_module.h"

using namespace markshot::shot;

/// @brief 切换"打开方式"面板的显示状态
/// @return 无返回值
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

/// @brief 刷新"打开方式"面板中的桌面应用列表
/// @return 无返回值
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

/// @brief 根据工具栏位置更新"打开方式"面板几何
/// @return 无返回值
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

/// @brief 切换扩展命令面板的显示状态
/// @return 无返回值
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

/// @brief 刷新扩展命令面板中的命令列表
/// @return 无返回值
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

/// @brief 根据工具栏位置更新扩展命令面板几何
/// @return 无返回值
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
