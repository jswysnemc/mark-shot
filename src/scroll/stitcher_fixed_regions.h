#pragma once

#include <QImage>

namespace markshot::scroll {

// 滚动截屏中的固定区域检测器。页面整体滚动时,sticky 头部/底部(以及横向
// 模式转置后的左右固定栏)在相邻两帧的同一行位置保持不变;检测器据此把帧
// 划分为固定带与滚动内容区,供匹配裁剪与拼接去重使用。
class FixedRegionDetector {
public:
    // 一次观察得出的固定带候选。header/footer 是从帧顶/帧底起连续相同的
    // 行数;valid 为 false 表示本次滚动量不足或帧不可比,候选不可用。
    struct Observation {
        int header = 0;
        int footer = 0;
        bool valid = false;
    };

    /// @brief 清空全部检测状态,帧尺寸或滚动轴变化时调用。
    void reset();

    /// @brief 用一对发生了可信滚动的相邻帧更新固定带估计。
    /// @param prevFrame 上一帧,要求 ARGB32_Premultiplied 格式。
    /// @param curFrame 当前帧,要求与上一帧同尺寸。
    /// @param offset 匹配得出的当前帧相对上一帧的位移。
    /// @return 本次观察的固定带候选;位移过小或帧不可比时 valid 为 false。
    Observation observe(const QImage &prevFrame, const QImage &curFrame, int offset);

    /// @brief 参与匹配裁剪的顶部固定带高度(经多帧平滑后的稳定值)。
    /// @return 顶部固定带行数。
    int matchTopIgnore() const;

    /// @brief 参与匹配裁剪的底部固定带高度(经多帧平滑后的稳定值)。
    /// @return 底部固定带行数。
    int matchBottomIgnore() const;

private:
    // 单侧固定带的平滑状态:candidate 立即下调,上调需连续多次观察确认,
    // 防止页面内容偶然相同导致固定带被高估。
    struct BandState {
        int committed = 0;
        int pending = -1;
        int pendingCount = 0;
        bool hasSample = false;

        /// @brief 用一次观察候选更新平滑值。
        /// @param candidate 本次观察得出的固定带行数。
        void update(int candidate);
    };

    BandState m_header;
    BandState m_footer;
};

}  // namespace markshot::scroll
