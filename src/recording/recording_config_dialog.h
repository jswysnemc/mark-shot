#pragma once

#include "recording/recording_options.h"

#include <QDialog>
#include <QVector>

class QCheckBox;
class QComboBox;
class QLabel;
class QLineEdit;

namespace markshot::recording {

class RecordingConfigDialog final : public QDialog {
public:
    /**
     * 创建录制配置对话框。
     * @param mode 录制模式。
     * @param parent 父窗口。
     */
    explicit RecordingConfigDialog(RecordingMode mode, QWidget *parent = nullptr);

    /**
     * 读取用户确认后的录制配置。
     * @return 录制配置。
     */
    RecordingOptions options() const;

private:
    /**
     * 打开输出文件选择对话框。
     * @return 无返回值。
     */
    void browseOutputPath();

    /**
     * 打开 FFmpeg 可执行文件选择对话框。
     * @return 无返回值。
     */
    void browseFfmpegPath();

    /**
     * 按录制模式更新音频控件可用状态。
     * @return 无返回值。
     */
    void updateAudioControls();

    RecordingMode m_mode = RecordingMode::Gif;
    QVector<DisplaySource> m_sources;
    QLabel *m_title = nullptr;
    QComboBox *m_modeSelector = nullptr;
    QComboBox *m_fps = nullptr;
    QCheckBox *m_audio = nullptr;
    QComboBox *m_display = nullptr;
    QComboBox *m_scope = nullptr;
    QLineEdit *m_outputPath = nullptr;
    QLineEdit *m_ffmpegPath = nullptr;
    bool m_outputPathTouched = false;
};

}  // namespace markshot::recording
