#pragma once

#include <QImage>
#include <QVector>

#include <array>

namespace markshot::scroll {

// 每行三段亮度采样,是所有重叠搜索共用的帧签名。比较成本低,
// 且对页面细节文字有足够的区分度。
using ColSamples = QVector<std::array<float, 3>>;

namespace frame_profile {

// 单个采样带内最多抽取的采样列数
inline constexpr int kColMaxBandSamples = 17;
// 像素级行比较判定"同一行内容"的最大平均色差
inline constexpr float kLineRowMaxDiff = 2.0f;

/// @brief 计算一帧图像的行亮度签名(每行三个采样带的平均亮度)。
/// @param frame 输入帧,要求为 ARGB32_Premultiplied 或 RGB32 格式,其余格式内部转换。
/// @return 长度等于帧高的行签名数组;帧无效时返回空数组。
ColSamples computeColSamples(const QImage &frame);

/// @brief 计算按比例忽略的帧顶部行数,用于规避浏览器工具条等边缘噪声。
/// @param height 帧高。
/// @return 顶部忽略的行数。
int contentTopIgnore(int height);

/// @brief 计算按比例忽略的帧底部行数。
/// @param height 帧高。
/// @return 底部忽略的行数。
int contentBottomIgnore(int height);

/// @brief 判断某行是否位于按比例忽略后的内容区内。
/// @param y 行号。
/// @param height 帧高。
/// @return 位于内容区返回 true。
bool isContentRow(int y, int height);

/// @brief 判断重叠是否足够长,允许启用按比例的边缘行裁剪。
/// @param overlapLen 重叠长度。
/// @param frameHeight 帧高。
/// @param minOverlap 配置的最小重叠。
/// @return 允许裁剪返回 true。
bool shouldCropContentRows(int overlapLen, int frameHeight, int minOverlap);

/// @brief 计算裁剪模式下有效比较行数的下限。
/// @param minOverlap 配置的最小重叠。
/// @param cropped 是否启用了边缘行裁剪。
/// @return 允许信任比较结果所需的最少行数。
int requiredComparedRows(int minOverlap, bool cropped);

/// @brief 计算两条像素行在采样列上的平均绝对色差。
/// @param a 第一条行的像素指针。
/// @param b 第二条行的像素指针。
/// @param startX 比较起始列。
/// @param width 比较宽度。
/// @return 平均绝对色差;参数无效时返回极大值。
float rowMeanAbsDiff(const QRgb *a, const QRgb *b, int startX, int width);

/// @brief 计算两帧行签名在给定偏移下重叠区的平均差,固定带行被强制剔除。
/// @param cols1 上一帧的行签名。
/// @param cols2 当前帧的行签名。
/// @param offset 当前帧相对上一帧的下移量(可为负)。
/// @param minOverlap 最小可信重叠行数。
/// @param fixedTop 帧顶部固定带高度,该区间行不参与比较。
/// @param fixedBottom 帧底部固定带高度。
/// @return 平均差;重叠不足或有效行过少时返回极大值。
float frameColDiff(const ColSamples &cols1,
                   const ColSamples &cols2,
                   int offset,
                   int minOverlap,
                   int fixedTop,
                   int fixedBottom);

/// @brief 计算帧签名放置在长图绝对位置上的重叠平均差。
/// @param fullCols 长图的行签名。
/// @param frameCols 当前帧的行签名。
/// @param framePos 帧顶在长图坐标中的位置。
/// @param minOverlap 最小可信重叠行数。
/// @param fixedTop 帧顶部固定带高度。
/// @param fixedBottom 帧底部固定带高度。
/// @param overlapLen 可选输出,几何重叠长度。
/// @param fullTopExclude 长图顶部排除的行数,即将被前插裁掉的旧固定带。
/// @param fullBottomExclude 长图底部排除的行数,即将被追加裁掉的旧固定带。
/// @return 平均差;重叠不足或有效行过少时返回极大值。
float overlapColDiff(const ColSamples &fullCols,
                     const ColSamples &frameCols,
                     int framePos,
                     int minOverlap,
                     int fixedTop,
                     int fixedBottom,
                     int *overlapLen = nullptr,
                     int fullTopExclude = 0,
                     int fullBottomExclude = 0);

}  // namespace frame_profile
}  // namespace markshot::scroll
