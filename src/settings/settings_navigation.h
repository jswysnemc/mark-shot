#pragma once

#include <QListWidget>
#include <QVector>

namespace markshot::settings {

/// @brief 设置界面侧栏导航组件。
///        封装标题区与分类列表，10 个分类按组排列，组间留白分隔。
///        对外暴露逻辑索引（0..9），与内容栈一一对应；分隔项不占用逻辑索引。
class SettingsNavigation final : public QWidget {
    Q_OBJECT

public:
    /// @brief 创建侧栏导航。
    /// @param parent 父控件。
    explicit SettingsNavigation(QWidget *parent = nullptr);

    /// @brief 设置当前激活的逻辑页索引。
    /// @param index 逻辑索引，范围 0..9。
    void setCurrentLogicalRow(int index);

signals:
    /// @brief 用户切换导航项时触发。
    /// @param logicalIndex 逻辑页索引，与内容栈对应。
    void navigationChanged(int logicalIndex);

private:
    /// @brief 构建标题区（应用名 + 副标题）。
    void buildHeader();

    /// @brief 构建导航列表，填充分类项与分隔项。
    void buildList();

    /// @brief 添加一个导航分类项。
    /// @param text 显示文本。
    /// @param icon 分类图标。
    void addCategory(const QString &text, const QIcon &icon);

    /// @brief 添加组间留白分隔项（不可选，不占逻辑索引）。
    void addSeparator();

    QListWidget *m_list = nullptr;
    /// 逻辑索引到实际列表行的映射
    QVector<int> m_logicalRows;
};

}  // namespace markshot::settings
