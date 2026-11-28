//! 【滚动截图】【行锚点】用相邻视口中的唯一纹理行对定位，不搜索累计长图

use crate::profile::Signature;
use std::collections::HashMap;

/// 【滚动截图】【位移投票】按前后帧签名及最小重叠返回有多处支持的位移；重复模板和空白不参与，平票不猜测
pub(super) fn offset(previous: &Signature, current: &Signature, minimum: i32) -> Option<i32> {
    let mut votes = HashMap::<i32, usize>::new();
    let stride = current.len().div_ceil(128).max(1);
    // 1. 【滚动截图】【锚点筛选】最多采样一百二十八组，只采用在旧视口中唯一且有纵向变化的行对
    for (row, pair) in current.windows(2).enumerate().step_by(stride) {
        if pair[0] == pair[1] {
            continue;
        }
        let mut matches = previous
            .windows(2)
            .enumerate()
            .filter(|(_, old)| *old == pair);
        let Some((old, _)) = matches.next() else {
            continue;
        };
        if matches.next().is_some() {
            continue;
        }
        let offset = old as i32 - row as i32;
        let overlap =
            (previous.len() as i32 - offset.max(0)).min(current.len() as i32 + offset.min(0));
        if overlap >= minimum {
            *votes.entry(offset).or_default() += 1;
        }
    }
    // 2. 【滚动截图】【歧义排除】签名只用于提议位置，相同支持数交回常规恢复路径
    let (&offset, &support) = votes.iter().max_by_key(|(_, count)| *count)?;
    (support >= 2 && votes.values().filter(|&&count| count == support).count() == 1)
        .then_some(offset)
}
