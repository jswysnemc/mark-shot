#pragma once

#include "ui/toolbar_palette.h"

#include <QColor>
#include <QIcon>
#include <QSet>
#include <QString>
#include <QVector>
#include <QWidget>

#include <functional>

class QBoxLayout;
class QPushButton;

namespace markshot::ui {

class ToolbarGrip;
class ToolbarSeparator;

/// @brief 【截图工具栏】【按钮角色】映射到主题样式表中的 role 属性
enum class ToolbarButtonRole {
    Normal,
    Primary,
    // 次级强调：强于普通次要动作、弱于主动作，用于复制等高频输出
    Accent,
    Secondary,
    Danger,
};

/// @brief 【截图工具栏】【按钮描述】由业务窗口提供，组件据此创建按钮
struct ToolbarButtonSpec {
    QString actionKey;
    QIcon icon;
    QString tooltip;
    ToolbarButtonRole role = ToolbarButtonRole::Normal;
    bool enabled = true;
    /// @brief 图标资源名，用于选中悬停时按主题色重绘前景
    QString iconName;
    /// @brief 常态前景色，无效时不随悬停切换；悬停前景取自工具栏配色
    QColor iconInk;
};

/// @brief 【截图工具栏】【紧凑面板】负责按钮创建、自绘背景、分组排版、横纵切换和统一选中态
///
/// 组件不处理业务动作与拖拽定位；所属窗口通过按钮信号和握柄事件完成映射
class CompactToolbar final : public QWidget {
    Q_OBJECT

public:
    /// @brief 工具栏尺寸参数
    struct Metrics {
        int iconSize = 22;
        int margin = 6;
        int spacing = 3;
    };

    /// @brief 创建紧凑工具栏，并在首位放置拖拽握柄
    /// @param objectName 工具栏对象名，匹配主题样式表选择器
    /// @param gripObjectName 握柄对象名，供光标策略与测试定位
    /// @param orientation 初始排列方向
    /// @param metrics 图标尺寸、外边距与间距
    /// @param parent 父控件
    CompactToolbar(const QString &objectName,
                   const QString &gripObjectName,
                   Qt::Orientation orientation,
                   Metrics metrics,
                   QWidget *parent = nullptr);

    /// @brief 获取拖拽握柄
    /// @return 握柄控件，生命周期归工具栏
    QWidget *grip() const;
    /// @brief 获取当前排列方向
    /// @return 横向或纵向
    Qt::Orientation orientation() const;
    /// @brief 切换排列方向，同步握柄、分隔线与尺寸
    /// @param orientation 新排列方向
    /// @return 无返回值
    void setOrientation(Qt::Orientation orientation);

    /// @brief 开启新的视觉分组；非首个分组前自动插入分隔线
    /// @return 分组索引
    int addGroup();
    /// @brief 在当前分组末尾创建按钮；尚无分组时自动创建
    /// @param spec 按钮描述
    /// @return 新按钮，生命周期归工具栏
    QPushButton *addButton(const ToolbarButtonSpec &spec);
    /// @brief 显示或隐藏整个分组
    /// @param group 分组索引
    /// @param visible 是否显示
    /// @return 无返回值
    void setGroupVisible(int group, bool visible);
    /// @brief 判断分组是否处于显式显示状态
    /// @param group 分组索引
    /// @return 分组内存在未隐藏按钮时返回 true
    bool isGroupVisible(int group) const;
    /// @brief 获取分组数量
    /// @return 分组数量
    int groupCount() const;

    /// @brief 按动作键查找按钮
    /// @param actionKey 按钮 action 属性
    /// @return 找到的按钮；不存在时返回 nullptr
    QPushButton *button(const QString &actionKey) const;
    /// @brief 获取全部按钮
    /// @return 按创建顺序排列的按钮
    QVector<QPushButton *> buttons() const;
    /// @brief 统一刷新按钮 active 属性，仅对状态变化的按钮重新应用样式
    /// @param activeKeys 处于选中态的动作键集合
    /// @return 无返回值
    void setActiveActions(const QSet<QString> &activeKeys);

    using DynamicIconCallback = std::function<QIcon(QPushButton *button, const QColor &ink)>;

    /// @brief 设置动态图标回调函数，用于特定按钮按当前状态或悬停色动态重绘图标
    /// @param callback 动态图标回调函数
    /// @return 无返回值
    void setDynamicIconCallback(DynamicIconCallback callback);

    /// @brief 设置面板底板、边框、分隔线、握柄与悬停前景使用的配色
    /// @param palette 工具栏配色令牌
    /// @return 无返回值
    void setToolbarPalette(const ToolbarPalette &palette);

protected:
    /// @brief 绘制圆角半透明背景与细边框
    /// @param event 绘制事件
    /// @return 无返回值
    void paintEvent(QPaintEvent *event) override;
    /// @brief 监听按钮显示状态变化，刷新分隔线
    /// @param watched 被监听对象
    /// @param event 对象事件
    /// @return 始终返回 false，不拦截按钮事件
    bool eventFilter(QObject *watched, QEvent *event) override;

private:
    /// @brief 视觉分组
    struct Group {
        ToolbarSeparator *separator = nullptr;
        QVector<QPushButton *> buttons;
    };

    /// @brief 根据相邻分组的可见按钮决定分隔线显示
    /// @return 无返回值
    void refreshSeparators();
    /// @brief 悬停时把普通工具前景换成配色中的悬停前景
    /// @param button 工具栏按钮
    /// @param hovered 指针是否在按钮上
    /// @return 无返回值
    void syncSelectedHoverIcon(QPushButton *button, bool hovered);
    /// @brief 尝试使用动态图标回调更新按钮图标
    /// @param button 工具栏按钮
    /// @param ink 本次应使用的前景色
    /// @return 动态图标已绘制时返回 true；未处理时返回 false
    bool paintDynamicIconIfEnabled(QPushButton *button, const QColor &ink);

    QBoxLayout *m_layout = nullptr;
    ToolbarGrip *m_grip = nullptr;
    Qt::Orientation m_orientation = Qt::Horizontal;
    Metrics m_metrics;
    QVector<Group> m_groups;
    bool m_refreshingSeparators = false;
    DynamicIconCallback m_dynamicIconCallback;
    ToolbarPalette m_toolbarPalette;
};

}
