#include "ui/compact_toolbar.h"

#include "ui/svg_icon.h"
#include "ui/toolbar_grip.h"

#include <QBoxLayout>
#include <QEvent>
#include <QPainter>
#include <QPushButton>
#include <QStyle>

namespace markshot::ui {
namespace {

constexpr qreal kPanelRadius = 11.0;
constexpr int kSeparatorLength = 16;
// 分隔线两侧留白只依赖布局间距，避免分组后工具栏总长超过未分组版本，进而撑大全屏窗口最小宽度
constexpr int kSeparatorGap = 0;

/// @brief 将按钮角色转换为主题样式表使用的 role 属性值
/// @param role 按钮角色
/// @return role 属性值；普通按钮返回空字符串
QString roleName(ToolbarButtonRole role)
{
    switch (role) {
    case ToolbarButtonRole::Primary:
        return QStringLiteral("primary");
    case ToolbarButtonRole::Accent:
        return QStringLiteral("accent");
    case ToolbarButtonRole::Secondary:
        return QStringLiteral("secondary");
    case ToolbarButtonRole::Danger:
        return QStringLiteral("danger");
    case ToolbarButtonRole::Normal:
        break;
    }
    return {};
}

/// @brief 判断悬停时是否切换为强调色底上的前景
/// @param button 工具栏按钮
/// @return 普通工具按钮返回 true
bool usesHoverForeground(const QPushButton *button)
{
    const QString role = button->property("role").toString();
    return role.isEmpty() || role == QStringLiteral("secondary");
}

}

/// @brief 【截图工具栏】【分组分隔】绘制分组间细线，方向与工具栏排列垂直
class ToolbarSeparator final : public QWidget {
public:
    /// @brief 创建分隔线
    /// @param parent 所属工具栏
    explicit ToolbarSeparator(QWidget *parent)
        : QWidget(parent)
    {
        setAttribute(Qt::WA_TransparentForMouseEvents, true);
        setOrientation(Qt::Horizontal);
    }

    /// @brief 设置分隔线颜色
    /// @param color 分隔线颜色
    /// @return 无返回值
    void setColor(const QColor &color)
    {
        m_color = color;
        update();
    }

    /// @brief 按工具栏方向设置分隔线尺寸
    /// @param orientation 工具栏排列方向
    /// @return 无返回值
    void setOrientation(Qt::Orientation orientation)
    {
        m_orientation = orientation;
        const int thickness = 1 + kSeparatorGap * 2;
        if (orientation == Qt::Horizontal) {
            setFixedSize(thickness, kSeparatorLength);
        } else {
            setFixedSize(kSeparatorLength, thickness);
        }
    }

protected:
    /// @brief 绘制居中的单像素细线
    /// @param event 绘制事件
    /// @return 无返回值
    void paintEvent(QPaintEvent *event) override
    {
        Q_UNUSED(event)
        QPainter painter(this);
        painter.setPen(Qt::NoPen);
        painter.setBrush(m_color);
        if (m_orientation == Qt::Horizontal) {
            painter.drawRect(QRect(width() / 2, 0, 1, height()));
        } else {
            painter.drawRect(QRect(0, height() / 2, width(), 1));
        }
    }

private:
    Qt::Orientation m_orientation = Qt::Horizontal;
    QColor m_color = ToolbarPalette{}.separator;
};

CompactToolbar::CompactToolbar(const QString &objectName,
                               const QString &gripObjectName,
                               Qt::Orientation orientation,
                               Metrics metrics,
                               QWidget *parent)
    : QWidget(parent)
    , m_orientation(orientation)
    , m_metrics(metrics)
{
    setObjectName(objectName);
    setCursor(Qt::ArrowCursor);

    // 1. 创建单一方向盒布局，横纵切换只改变方向，不重建子控件
    m_layout = new QBoxLayout(orientation == Qt::Horizontal ? QBoxLayout::LeftToRight : QBoxLayout::TopToBottom, this);
    m_layout->setContentsMargins(metrics.margin, metrics.margin, metrics.margin, metrics.margin);
    m_layout->setSpacing(metrics.spacing);

    // 2. 首位放置拖拽握柄，拖拽与吸附逻辑由所属窗口通过事件过滤器处理
    m_grip = new ToolbarGrip(this);
    m_grip->setObjectName(gripObjectName);
    m_grip->setOrientation(orientation);
    m_layout->addWidget(m_grip);
}

QWidget *CompactToolbar::grip() const
{
    return m_grip;
}

Qt::Orientation CompactToolbar::orientation() const
{
    return m_orientation;
}

void CompactToolbar::setOrientation(Qt::Orientation orientation)
{
    if (m_orientation == orientation) {
        return;
    }
    m_orientation = orientation;
    // 1. 切换布局方向并同步握柄、分隔线形态
    m_layout->setDirection(orientation == Qt::Horizontal ? QBoxLayout::LeftToRight : QBoxLayout::TopToBottom);
    m_grip->setOrientation(orientation);
    for (const Group &group : std::as_const(m_groups)) {
        if (group.separator) {
            group.separator->setOrientation(orientation);
        }
    }
    // 2. 让父窗口按新尺寸提示重新计算几何
    m_layout->invalidate();
    updateGeometry();
    adjustSize();
}

int CompactToolbar::addGroup()
{
    Group group;
    // 1. 非首个分组前插入分隔线，可见性在按钮显隐后统一计算
    if (!m_groups.isEmpty()) {
        group.separator = new ToolbarSeparator(this);
        group.separator->setOrientation(m_orientation);
        group.separator->setColor(m_toolbarPalette.separator);
        m_layout->addWidget(group.separator, 0, Qt::AlignCenter);
    }
    m_groups.append(group);
    refreshSeparators();
    return m_groups.size() - 1;
}

QPushButton *CompactToolbar::addButton(const ToolbarButtonSpec &spec)
{
    if (m_groups.isEmpty()) {
        addGroup();
    }

    // 1. 创建按钮并写入主题样式依赖的 action、role 属性
    auto *button = new QPushButton(this);
    button->setIcon(spec.icon);
    button->setIconSize(QSize(m_metrics.iconSize, m_metrics.iconSize));
    button->setFocusPolicy(Qt::NoFocus);
    button->setToolTip(spec.tooltip);
    button->setAccessibleName(spec.tooltip);
    button->setProperty("action", spec.actionKey);
    button->setProperty("active", false);
    const QString role = roleName(spec.role);
    if (!role.isEmpty()) {
        button->setProperty("role", role);
    }
    button->setEnabled(spec.enabled);
    if (!spec.iconName.isEmpty()) {
        button->setProperty("iconName", spec.iconName);
    }
    if (spec.iconInk.isValid()) {
        button->setProperty("iconInk", spec.iconInk);
    }

    // 2. 加入当前分组并监听显隐，用于维护分隔线
    button->installEventFilter(this);
    m_layout->addWidget(button);
    m_groups.last().buttons.append(button);
    refreshSeparators();
    return button;
}

void CompactToolbar::setGroupVisible(int group, bool visible)
{
    if (group < 0 || group >= m_groups.size()) {
        return;
    }
    // 1. 批量切换按钮，结束后统一刷新分隔线，避免逐个按钮重复计算
    m_refreshingSeparators = true;
    for (QPushButton *groupButton : std::as_const(m_groups[group].buttons)) {
        groupButton->setVisible(visible);
    }
    m_refreshingSeparators = false;
    refreshSeparators();
    updateGeometry();
}

bool CompactToolbar::isGroupVisible(int group) const
{
    if (group < 0 || group >= m_groups.size()) {
        return false;
    }
    for (const QPushButton *groupButton : m_groups[group].buttons) {
        if (!groupButton->isHidden()) {
            return true;
        }
    }
    return false;
}

int CompactToolbar::groupCount() const
{
    return m_groups.size();
}

QPushButton *CompactToolbar::button(const QString &actionKey) const
{
    for (const Group &group : m_groups) {
        for (QPushButton *groupButton : group.buttons) {
            if (groupButton->property("action").toString() == actionKey) {
                return groupButton;
            }
        }
    }
    return nullptr;
}

QVector<QPushButton *> CompactToolbar::buttons() const
{
    QVector<QPushButton *> result;
    for (const Group &group : m_groups) {
        result += group.buttons;
    }
    return result;
}

void CompactToolbar::setActiveActions(const QSet<QString> &activeKeys)
{
    for (const Group &group : std::as_const(m_groups)) {
        for (QPushButton *groupButton : group.buttons) {
            const bool active = activeKeys.contains(groupButton->property("action").toString());
            if (groupButton->property("active").toBool() == active) {
                continue;
            }
            // 1. 仅对状态变化的按钮重新应用动态属性样式
            groupButton->setProperty("active", active);
            groupButton->style()->unpolish(groupButton);
            groupButton->style()->polish(groupButton);
            syncSelectedHoverIcon(groupButton, groupButton->underMouse());
            groupButton->update();
        }
    }
}

void CompactToolbar::paintEvent(QPaintEvent *event)
{
    Q_UNUSED(event)
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing, true);
    painter.setPen(QPen(m_toolbarPalette.panelBorder, 1.0));
    painter.setBrush(m_toolbarPalette.panelFill);
    painter.drawRoundedRect(QRectF(rect()).adjusted(0.5, 0.5, -0.5, -0.5), kPanelRadius, kPanelRadius);
}

bool CompactToolbar::eventFilter(QObject *watched, QEvent *event)
{
    // 1. 按钮单独显隐时刷新分隔线；批量切换期间由调用方统一刷新
    if (!m_refreshingSeparators
        && (event->type() == QEvent::ShowToParent || event->type() == QEvent::HideToParent)
        && qobject_cast<QPushButton *>(watched)) {
        refreshSeparators();
    }
    if (auto *button = qobject_cast<QPushButton *>(watched)) {
        if (event->type() == QEvent::Enter || event->type() == QEvent::HoverEnter) {
            syncSelectedHoverIcon(button, true);
        } else if (event->type() == QEvent::Leave || event->type() == QEvent::HoverLeave) {
            syncSelectedHoverIcon(button, false);
        }
    }
    return QWidget::eventFilter(watched, event);
}

void CompactToolbar::syncSelectedHoverIcon(QPushButton *button, bool hovered)
{
    if (!button) {
        return;
    }
    const QString iconName = button->property("iconName").toString();
    const QColor normalInk = button->property("iconInk").value<QColor>();
    if (iconName.isEmpty() || !normalInk.isValid()) {
        return;
    }
    // 1. 悬停使用强调色底上的配色前景，离开后恢复常态前景
    const bool hoverInk = button->isEnabled() && hovered && usesHoverForeground(button);
    const QColor ink = hoverInk ? m_toolbarPalette.hoverIcon : normalInk;
    if (paintDynamicIconIfEnabled(button, ink)) {
        return;
    }
    button->setIcon(makeSvgIcon(iconName, ink));
}

void CompactToolbar::setDynamicIconCallback(DynamicIconCallback callback)
{
    // 1. 【工具栏】【动态图标】记录外部注入的图标绘制委托
    m_dynamicIconCallback = std::move(callback);
}

void CompactToolbar::setToolbarPalette(const ToolbarPalette &palette)
{
    // 1. 记录配色并同步握柄与已有分隔线，底板在下次绘制时生效
    m_toolbarPalette = palette;
    m_grip->setToolbarPalette(palette);
    for (const Group &group : std::as_const(m_groups)) {
        if (group.separator) {
            group.separator->setColor(palette.separator);
        }
    }
    update();
}

void CompactToolbar::refreshSeparators()
{
    if (m_refreshingSeparators) {
        return;
    }
    m_refreshingSeparators = true;
    // 1. 分隔线仅出现在前方已有可见分组、且本分组也可见的位置
    bool hasVisibleBefore = false;
    for (int index = 0; index < m_groups.size(); ++index) {
        const bool groupVisible = isGroupVisible(index);
        if (ToolbarSeparator *separator = m_groups[index].separator) {
            separator->setVisible(hasVisibleBefore && groupVisible);
        }
        hasVisibleBefore = hasVisibleBefore || groupVisible;
    }
    m_refreshingSeparators = false;
}

namespace {

constexpr bool FEATURE_DYNAMIC_TOOLBAR_ICON_ENABLED = true;

}  // namespace

bool CompactToolbar::paintDynamicIconIfEnabled(QPushButton *button, const QColor &ink)
{
    if (!FEATURE_DYNAMIC_TOOLBAR_ICON_ENABLED || !button || !m_dynamicIconCallback) {
        return false;
    }
    // 1. 【工具栏】【动态图标】委托外部按当前状态或悬停色生成图标
    const QIcon dynamicIcon = m_dynamicIconCallback(button, ink);
    if (dynamicIcon.isNull()) {
        return false;
    }
    button->setIcon(dynamicIcon);
    return true;
}

}
