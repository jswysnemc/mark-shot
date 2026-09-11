#pragma once

#include "capture_history/history_store.h"

#include <QWidget>

class QCheckBox;
class QLabel;
class QListWidget;
class QPushButton;

namespace markshot::history {

/// @brief 浏览已完成的截图并提供常用图像操作
class HistoryWindow final : public QWidget {
public:
    /// @brief 创建截图历史窗口
    explicit HistoryWindow();

    /// @brief 重新读取历史，并尽量保留当前选择
    /// @return 无返回值
    void refresh();

protected:
    /// @brief 重新激活窗口时同步其他截图会话新增或删除的历史
    /// @param event 窗口状态变更事件
    /// @return 无返回值
    void changeEvent(QEvent *event) override;

    /// @brief 窗口调整大小后重新适配预览
    /// @param event 尺寸变更事件
    /// @return 无返回值
    void resizeEvent(QResizeEvent *event) override;

private:
    /// @brief 获取当前选择的历史标识
    /// @return 文件标识，没有选择时为空
    QString selectedFileName() const;
    /// @brief 读取当前历史图片并更新操作按钮
    /// @return 无返回值
    void updateSelection();
    /// @brief 按可用空间绘制当前图片预览
    /// @return 无返回值
    void updatePreview();
    /// @brief 显示操作结果
    /// @param text 提示文字
    /// @return 无返回值
    void showStatus(const QString &text);
    /// @brief 复制当前图片且不重复记录历史
    /// @return 无返回值
    void copySelected();
    /// @brief 打开当前图片进行标注
    /// @return 无返回值
    void editSelected();
    /// @brief 把当前图片钉在桌面上
    /// @return 无返回值
    void pinSelected();
    /// @brief 将当前图片另存为 PNG 文件
    /// @return 无返回值
    void saveSelected();
    /// @brief 删除当前历史项
    /// @return 无返回值
    void removeSelected();
    /// @brief 使用统一样式的对话框确认清空历史
    /// @return 无返回值
    void confirmClear();
    /// @brief 保存自动记录开关
    /// @param enabled 是否自动记录完成的截图
    /// @return 无返回值
    void setRecordingEnabled(bool enabled);

    HistoryStore m_store;
    QImage m_currentImage;
    QListWidget *m_list = nullptr;
    QLabel *m_preview = nullptr;
    QLabel *m_summary = nullptr;
    QLabel *m_status = nullptr;
    QCheckBox *m_enabled = nullptr;
    QPushButton *m_clear = nullptr;
    QVector<QPushButton *> m_imageActions;
};

/// @brief 显示或激活当前进程的截图历史窗口
/// @return 无返回值
void showHistoryWindow();

}
