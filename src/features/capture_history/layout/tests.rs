//! 【截图历史测试】【紧凑布局】窗口缩放不拉长操作栏，两侧正文和底部操作始终可达

use super::*;

/// 【截图历史测试】【控件排列】多种宽度和状态行高度下检查控件边界及分区，保持操作库存不变，无参数和返回值
#[test]
fn compact_history_keeps_controls_and_split_panes_accessible() {
    for width in [440, 620, 720, 1280] {
        for status in [0.0, 19.0, 38.0] {
            let layout = Layout::with_options((width, 480), 190.0, status);
            assert_eq!(layout.buttons.len(), 8);
            assert!(
                layout
                    .buttons
                    .iter()
                    .all(|(_, r)| r.x() >= 0.0 && r.right() <= width as f32 && r.bottom() <= 480.0)
            );
            assert!(layout.list.width() >= 184.0 && layout.preview.width() >= 180.0);
            assert_eq!(layout.list.y(), layout.preview.y());
            for control in [
                Control::Copy,
                Control::Edit,
                Control::Pin,
                Control::Save,
                Control::Delete,
            ] {
                let bounds = layout
                    .buttons
                    .iter()
                    .find(|(c, _)| *c == control)
                    .unwrap()
                    .1;
                assert_eq!(bounds.height(), 26.0);
                assert!(bounds.width() <= 82.0 && bounds.bottom() < layout.list.y());
                assert_eq!(
                    layout.control((f64::from(bounds.x() + 1.0), f64::from(bounds.y() + 1.0))),
                    Some(control)
                );
            }
            assert!(
                layout
                    .buttons
                    .iter()
                    .filter(|(c, _)| matches!(c, Control::Enabled | Control::Clear))
                    .all(|(_, r)| r.y() >= layout.list.bottom())
            );
        }
    }
}
