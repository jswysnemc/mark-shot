//! 【工具栏设置】【位置预览】用选区和工具栏示意当前停靠方式，读取设置快照即时绘制

use super::layout::rect;
use crate::{
    config::capture::ToolbarPosition as Position,
    renderer::{Painter, text::TextStyle},
    theme::{Rgba, window::Palette},
    ui::controls,
};
use tiny_skia::Rect;

pub(super) const HEIGHT: f32 = 192.0;

/// 【工具栏设置】【预览取值】按 values 返回当前工具栏位置，缺失或无效时回退跟随选区
pub(super) fn position(values: &serde_json::Value) -> Position {
    serde_json::from_value(values["capture"]["toolbar_position"].clone()).unwrap_or_default()
}

struct Geometry {
    screen: Rect,
    selection: Rect,
    main: Rect,
    side: Rect,
    properties: Rect,
}

/// 【工具栏设置】【预览几何】按 bounds 和 position 缩放示意屏幕，固定位置复用正式双栏停靠计算
fn geometry(bounds: Rect, position: Position) -> Geometry {
    let width = bounds.width().min(320.0);
    let scale = width / 1000.0;
    let screen = rect(
        bounds.left() + (bounds.width() - width) / 2.0,
        bounds.top() + 24.0,
        width,
        450.0 * scale,
    );
    let project = |point: (f32, f32), size: (f32, f32)| {
        rect(
            screen.left() + point.0 * scale,
            screen.top() + point.1 * scale,
            size.0 * scale,
            size.1 * scale,
        )
    };
    let vertical = matches!(position, Position::Left | Position::Right);
    let main_size = if vertical {
        (38.0, 360.0)
    } else {
        (560.0, 38.0)
    };
    let side_size = if vertical || position == Position::Selection {
        (36.0, 240.0)
    } else {
        (320.0, 36.0)
    };
    let (main, side) = if position == Position::Selection {
        ((230.0, 338.0), (810.0, 130.0))
    } else {
        crate::ui::toolbar::docking::positions(
            position,
            (1000.0, 450.0),
            main_size,
            Some(side_size),
        )
    };
    let properties = match position {
        Position::Selection => (365.0, 386.0),
        Position::Top => (335.0, 56.0),
        Position::Bottom => (335.0, 360.0),
        Position::Left => (side.0 + side_size.0 + 12.0, 208.0),
        Position::Right => (side.0 - 342.0, 208.0),
    };
    Geometry {
        screen,
        selection: project((330.0, 150.0), (360.0, 170.0)),
        main: project(main, main_size),
        side: project(side, side_size),
        properties: project(properties, (330.0, 34.0)),
    }
}

/// 【工具栏设置】【缩略工具栏】按 bounds、color 和 fill 绘制带按钮单元的工具栏，自动辨别横纵方向
fn bar(painter: &mut Painter<'_>, bounds: Rect, color: Rgba, fill: Rgba) {
    controls::frame(painter, bounds, fill, color, 2.0);
    let vertical = bounds.height() > bounds.width();
    let cross = if vertical {
        bounds.width()
    } else {
        bounds.height()
    };
    let length = if vertical {
        bounds.height()
    } else {
        bounds.width()
    };
    let cell = (cross - 5.0).max(2.0);
    let stride = cell + 4.0;
    let count = ((length - 6.0) / stride).floor() as usize;
    for index in 0..count {
        let offset = 4.0 + index as f32 * stride;
        let (x, y) = if vertical {
            ((cross - cell) / 2.0, offset)
        } else {
            (offset, (cross - cell) / 2.0)
        };
        painter.fill_rounded(
            rect(bounds.left() + x, bounds.top() + y, cell, cell),
            1.0,
            color,
        );
    }
}

/// 【工具栏设置】【示意绘制】按 bounds、position 和 chinese 绘制当前选区与三条栏，颜色沿用设置页主题
pub(super) fn draw(painter: &mut Painter<'_>, bounds: Rect, position: Position, chinese: bool) {
    let palette = Palette::current();
    let layout = geometry(bounds, position);
    let label = match (position, chinese) {
        (Position::Selection, true) => "跟随选区",
        (Position::Top, true) => "屏幕顶部",
        (Position::Bottom, true) => "屏幕底部",
        (Position::Left, true) => "屏幕左侧",
        (Position::Right, true) => "屏幕右侧",
        (Position::Selection, false) => "Follow selection",
        (Position::Top, false) => "Screen top",
        (Position::Bottom, false) => "Screen bottom",
        (Position::Left, false) => "Screen left",
        (Position::Right, false) => "Screen right",
    };
    painter.text(
        rect(bounds.left(), bounds.top(), bounds.width(), 20.0),
        &format!(
            "{} · {label}",
            if chinese {
                "位置预览"
            } else {
                "Position preview"
            }
        ),
        TextStyle::new(10.0, palette.muted).center(),
    );
    controls::frame(painter, layout.screen, palette.window, palette.border, 5.0);
    controls::frame(painter, layout.selection, palette.base, palette.border, 2.0);
    for x in [layout.selection.left(), layout.selection.right()] {
        for y in [layout.selection.top(), layout.selection.bottom()] {
            painter.fill_rounded(rect(x - 1.5, y - 1.5, 3.0, 3.0), 0.5, palette.accent);
        }
    }
    painter.text(
        layout.selection,
        if chinese { "选区" } else { "Selection" },
        TextStyle::new(10.0, palette.muted).center(),
    );
    bar(painter, layout.main, palette.accent, palette.pressed);
    bar(painter, layout.side, palette.more_icon, palette.base);
    bar(painter, layout.properties, palette.muted, palette.alternate);
    let labels = if chinese {
        ["主栏", "侧栏", "属性栏"]
    } else {
        ["Main", "Side", "Properties"]
    };
    let legend_width = layout.screen.width().min(250.0);
    let cell = legend_width / 3.0;
    let left = layout.screen.left() + (layout.screen.width() - legend_width) / 2.0;
    for (index, color) in [palette.accent, palette.more_icon, palette.muted]
        .into_iter()
        .enumerate()
    {
        let x = left + index as f32 * cell;
        let y = layout.screen.bottom() + 4.0;
        painter.fill_rounded(rect(x, y + 6.0, 7.0, 7.0), 2.0, color);
        painter.text(
            rect(x + 12.0, y, cell - 12.0, 18.0),
            labels[index],
            TextStyle::new(10.0, palette.muted),
        );
    }
}

#[cfg(test)]
mod tests;
