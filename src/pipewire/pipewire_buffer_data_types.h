#pragma once

#include <array>
#include <cstdint>

namespace markshot::pipewire {

/**
 * 【录制】【PipeWire协商】构造视频缓冲区可接受的数据类型掩码。
 * @param hasModifier 视频格式是否携带 DRM modifier。
 * @return SPA 数据类型位掩码。
 */
std::uint32_t bufferDataTypeMask(bool hasModifier);

/**
 * 【录制】【PipeWire协商】返回格式 modifier 变体的声明顺序。
 * @param rawStreamMode 是否为录制使用的 raw 帧流。
 * @return 两个变体的 withModifier 参数，按协商优先级排列。
 */
std::array<bool, 2> modifierPreference(bool rawStreamMode);

}  // namespace markshot::pipewire
