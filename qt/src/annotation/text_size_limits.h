#pragma once

namespace markshot::shot::text_size {

inline constexpr int kMinTextPointSize = 8;
inline constexpr int kMaxTextPointSize = 300;
// 【文字标注】【字号】标注宽度保存相对于基准字号的偏移，面板与样式更新共用边界
inline constexpr int kTextPointSizeOffset = 19;

}  // namespace markshot::shot::text_size
