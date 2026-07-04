#pragma once

#include "shot_window_internal.h"

#include <QByteArray>
#include <QPoint>
#include <QSize>
#include <QString>
#include <QWidget>

class QComboBox;
class QEvent;
class QLabel;
class QKeyEvent;
class QMouseEvent;
class QPushButton;
class QTextEdit;

namespace markshot::providers {
class ProviderTask;
}

namespace markshot::shot {

/// @brief 显示 OCR 结果并支持编辑、复制、翻译和置顶切换的浮窗。
class OcrResultWindow final : public QWidget {
public:
    /// @brief 创建 OCR 结果浮窗。
    /// @param text 初始 OCR 文本。
    explicit OcrResultWindow(QString text);

    /// @brief 取消仍在运行的翻译任务并释放临时文件。
    ~OcrResultWindow() override;

protected:
    /// @brief 处理标题栏拖动或普通鼠标按下。
    /// @param event 鼠标事件。
    void mousePressEvent(QMouseEvent *event) override;

    /// @brief 处理窗口拖动。
    /// @param event 鼠标事件。
    void mouseMoveEvent(QMouseEvent *event) override;

    /// @brief 结束窗口拖动。
    /// @param event 鼠标事件。
    void mouseReleaseEvent(QMouseEvent *event) override;

    /// @brief 处理 Esc 关闭浮窗。
    /// @param event 键盘事件。
    void keyPressEvent(QKeyEvent *event) override;

    /// @brief 处理标题栏控件拖动事件。
    /// @param watched 事件目标。
    /// @param event Qt 事件。
    /// @return 已处理时返回 true。
    bool eventFilter(QObject *watched, QEvent *event) override;

private:
    /// @brief 判断标题栏交互控件是否包含指定窗口坐标。
    /// @param windowPoint 窗口局部坐标。
    /// @return 命中标题栏控件时返回 true。
    bool titleControlContains(QPoint windowPoint) const;

    /// @brief 开始拖动 OCR 浮窗。
    /// @param event 鼠标事件。
    /// @return 成功开始拖动时返回 true。
    bool beginWindowDrag(QMouseEvent *event);

    /// @brief 更新 OCR 浮窗拖动位置。
    /// @param event 鼠标事件。
    /// @return 当前处于拖动状态时返回 true。
    bool updateWindowDrag(QMouseEvent *event);

    /// @brief 结束 OCR 浮窗拖动。
    /// @param event 鼠标事件。
    /// @return 成功结束拖动时返回 true。
    bool finishWindowDrag(QMouseEvent *event);

    /// @brief 返回 OCR 浮窗初始尺寸。
    /// @return 初始窗口尺寸。
    QSize initialWindowSize() const;

    /// @brief 将 OCR 浮窗移动到主屏幕中央。
    /// @return 无返回值。
    void centerOnPrimaryScreen();

    /// @brief 在 OCR 浮窗底部显示短提示。
    /// @param text 提示文本。
    /// @param durationMs 显示时长。
    void showToast(const QString &text, int durationMs = 2000);

    /// @brief 显示 OCR 文本编辑框右键菜单。
    /// @param globalPosition 菜单弹出全局位置。
    void showEditorContextMenu(const QPoint &globalPosition);

    /// @brief 初始化目标语言下拉框。
    /// @return 无返回值。
    void setupTargetLanguageCombo();

    /// @brief 设置目标语言下拉框当前值。
    /// @param targetLanguage 实际传递给翻译器的目标语言名称。
    void setTargetLanguageComboValue(const QString &targetLanguage);

    /// @brief 返回目标语言下拉框当前实际值。
    /// @return 实际传递给翻译器的目标语言名称。
    QString currentTargetLanguage() const;

    /// @brief 从下拉框读取目标语言并持久化到应用配置。
    /// @return 无返回值。
    void applyTargetLanguageFromCombo();

    /// @brief 启动翻译任务。
    /// @return 无返回值。
    void startTranslation();

    /// @brief 处理翻译任务输出。
    /// @param task 翻译任务。
    /// @param output 翻译输出 JSON。
    void finishTranslation(markshot::providers::ProviderTask *task, const QByteArray &output);

    /// @brief 取消正在运行的翻译任务并清理临时文件。
    /// @return 无返回值。
    void cancelTranslation();

    /// @brief 清理已结束的翻译任务。
    /// @param task 翻译任务。
    void finishTranslationCleanup(markshot::providers::ProviderTask *task);

    /// @brief 恢复翻译按钮与语言下拉框状态。
    /// @return 无返回值。
    void resetTranslationUi();

    /// @brief 切换并保存浮窗置顶状态。
    /// @param alwaysOnTop 是否保持窗口置顶。
    void setAlwaysOnTop(bool alwaysOnTop);

    QWidget *m_titleBar = nullptr;
    QLabel *m_titleLabel = nullptr;
    QTextEdit *m_editor = nullptr;
    QPushButton *m_translateButton = nullptr;
    QComboBox *m_targetLanguageCombo = nullptr;
    QPushButton *m_targetLanguagePopupButton = nullptr;
    QPushButton *m_pinButton = nullptr;
    markshot::providers::ProviderTask *m_translationTask = nullptr;
    QString m_translationInputPath;
    QPoint m_dragOffset;
    PinnedWindowConfig m_config;
    bool m_alwaysOnTop = true;
    bool m_dragging = false;
};

}  // namespace markshot::shot
