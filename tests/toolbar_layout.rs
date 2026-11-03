//! 【工具栏测试】【原版排版】验证主副栏分组、横纵排列、窄屏适配与共享命中区域

use mark_shot::{
    tools::Tool,
    ui::{
        panel::UiPanel,
        toolbar::{Orientation, Toolbar, ToolbarButton, ToolbarItem},
    },
};

/// 【工具栏测试】【按钮命中】在各方向中验证实际绘制矩形与输入命中相同
#[test]
fn cell_centers_hit_the_same_button_in_each_orientation() {
    for secondary in [false, true] {
        for orientation in [Orientation::Horizontal, Orientation::Vertical] {
            let mut toolbar = if secondary {
                Toolbar::actions()
            } else {
                Toolbar::new()
            };
            toolbar.orientation = orientation;
            toolbar.opacity = 1.0;
            toolbar.render_pos = (20.0, 40.0);
            toolbar.arrange((320.0, 240.0));
            for (index, item) in toolbar.items.iter().enumerate() {
                let cell = toolbar.cell(index).unwrap();
                let point = (
                    f64::from(toolbar.render_pos.0 + cell.left() + cell.width() / 2.0),
                    f64::from(toolbar.render_pos.1 + cell.top() + cell.height() / 2.0),
                );
                assert_eq!(
                    toolbar.hit_test(point),
                    (
                        true,
                        matches!(item, ToolbarItem::Button(_)).then_some(index)
                    )
                );
            }
            assert!(
                toolbar.size.0 <= 320.0 && toolbar.size.1 <= 240.0,
                "{:?}",
                toolbar.size
            );
        }
    }
}

/// 【工具栏测试】【原版顺序】主栏按移动、选择、绘制、形状、标记、历史排列，OCR 只在副栏
#[test]
fn original_main_toolbar_groups_are_preserved() {
    let main = Toolbar::new();
    assert_eq!(
        main.items[0],
        ToolbarItem::Button(ToolbarButton::Tool(Tool::Selection))
    );
    assert_eq!(
        main.items[1],
        ToolbarItem::Button(ToolbarButton::Tool(Tool::Pick))
    );
    assert_eq!(
        main.items.last(),
        Some(&ToolbarItem::Button(ToolbarButton::Redo))
    );
    assert!(
        !main
            .items
            .contains(&ToolbarItem::Button(ToolbarButton::Tool(Tool::Ocr)))
    );
    assert!(
        Toolbar::actions()
            .items
            .contains(&ToolbarItem::Button(ToolbarButton::Tool(Tool::Ocr)))
    );
    assert_eq!(main.size.1, 42.0);
    assert_eq!(main.grip().width(), 10.0);
    assert_eq!(Toolbar::actions().grip().height(), 10.0);
}

/// 【工具栏测试】【隐藏命中】不可见工具栏没有可点击透明区域
#[test]
fn hidden_toolbar_does_not_hit_canvas() {
    let mut toolbar = Toolbar::new();
    toolbar.start_hidden();
    assert!(toolbar.rect().is_none());
    assert_eq!(toolbar.hit_test((12.0, 12.0)), (false, None));
}
