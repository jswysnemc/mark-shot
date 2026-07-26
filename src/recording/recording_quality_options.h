#pragma once

#include <QString>

namespace markshot::recording {

enum class RecordingQuality {
    Efficient,
    Balanced,
    High,
};

struct RecordingQualityProfile {
    // 恒定质量取值，按编码器分别映射为 crf、cq、qp 或 global_quality
    int constantQuality = 23;
    // libx264 等软件编码器的速度预设
    QString softwarePreset;
    // NVENC 等硬件编码器的速度预设
    QString hardwarePreset;
    // 需要显式码率的编码器在估算码率上的倍率
    double bitRateFactor = 1.0;
};

/**
 * 【录制】【质量档位】读取质量档位对应的编码参数。
 * @param quality 质量档位。
 * @param fps 目标帧率，高帧率会自动放宽速度预设。
 * @return 编码参数。
 */
RecordingQualityProfile recordingQualityProfile(RecordingQuality quality, int fps);

/**
 * 【录制】【质量档位】读取质量档位的配置文本。
 * @param quality 质量档位。
 * @return 配置文本。
 */
QString recordingQualityName(RecordingQuality quality);

/**
 * 【录制】【质量档位】从配置文本解析质量档位。
 * @param text 配置文本。
 * @return 质量档位，无法识别时返回 Balanced。
 */
RecordingQuality recordingQualityFromName(QString text);

}  // namespace markshot::recording
