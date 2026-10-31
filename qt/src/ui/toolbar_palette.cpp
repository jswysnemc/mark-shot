#include "ui/toolbar_palette.h"

namespace markshot::ui {
namespace {

/// @brief 生成亮色工具栏配色：白色玻璃底板、石板灰前景与深青色强调
/// @return 亮色配色令牌
ToolbarPalette lightToolbarPalette()
{
    ToolbarPalette palette;
    palette.dark = false;

    // 1. 底板接近不透明，压在任意截图内容上都保持可读
    palette.panelFill = QColor(248, 250, 252, 242);
    palette.panelBorder = QColor(15, 23, 42, 30);
    palette.popupFill = QColor(255, 255, 255, 248);
    palette.popupBorder = QColor(15, 23, 42, 26);
    palette.separator = QColor(100, 116, 139, 70);
    palette.gripDot = QColor(100, 116, 139, 150);
    palette.gripDotHover = QColor(51, 65, 85, 220);
    palette.gripDotDisabled = QColor(100, 116, 139, 60);
    palette.gripHoverFill = QColor(100, 116, 139, 36);

    // 2. 前景使用石板灰层级，强调色悬停时前景切换为白色
    palette.text = QColor(51, 65, 85);
    palette.strongText = QColor(15, 23, 42);
    palette.mutedText = QColor(100, 116, 139);
    palette.hoverFill = QColor(13, 148, 136);
    palette.hoverBorder = QColor(20, 184, 166);
    palette.hoverText = QColor(255, 255, 255);
    palette.hoverIcon = QColor(255, 255, 255);
    palette.pressedFill = QColor(15, 118, 110);
    palette.pressedBorder = QColor(13, 148, 136);
    palette.activeFill = QColor(13, 148, 136, 36);
    palette.activeBorder = QColor(13, 148, 136, 110);

    // 3. 次级强调与危险动作降低饱和度，避免在白底上过于刺眼
    palette.accentFill = QColor(13, 148, 136, 26);
    palette.accentBorder = QColor(13, 148, 136, 90);
    palette.accentHoverFill = QColor(13, 148, 136, 52);
    palette.accentHoverBorder = QColor(13, 148, 136, 150);
    palette.dangerHoverText = QColor(185, 28, 28);
    palette.dangerHoverFill = QColor(239, 68, 68, 40);
    palette.dangerHoverBorder = QColor(220, 38, 38, 110);

    // 4. 输入控件、列表与滑块改用深色半透明描边
    palette.controlFill = QColor(15, 23, 42, 10);
    palette.controlHoverFill = QColor(15, 23, 42, 18);
    palette.controlBorder = QColor(15, 23, 42, 30);
    palette.focusBorder = QColor(13, 148, 136, 170);
    palette.listFill = QColor(255, 255, 255);
    palette.listBorder = QColor(15, 23, 42, 26);
    palette.selectionFill = QColor(13, 148, 136, 40);
    palette.softHoverFill = QColor(13, 148, 136, 22);
    palette.softHoverBorder = QColor(13, 148, 136, 80);
    palette.sliderGroove = QColor(15, 23, 42, 30);
    palette.sliderEnd = QColor(20, 184, 166);
    palette.sliderHandleBorder = QColor(15, 23, 42, 90);
    palette.sliderHandleHoverBorder = QColor(13, 148, 136);

    // 5. 形状网格按钮使用浅灰底，选中态沿用青色渐变
    palette.tileFill = QColor(241, 245, 249);
    palette.tileBorder = QColor(148, 163, 184, 90);
    palette.tileHoverFill = QColor(13, 148, 136, 30);
    palette.tileHoverBorder = QColor(13, 148, 136, 100);
    palette.tileCheckedBorder = QColor(13, 148, 136);
    return palette;
}

}  // namespace

ToolbarPalette toolbarPalette(UiThemeMode mode)
{
    return mode == UiThemeMode::Light ? lightToolbarPalette() : ToolbarPalette{};
}

QString cssColor(const QColor &color)
{
    if (color.alpha() == 255) {
        return color.name(QColor::HexRgb).toUpper();
    }
    return QStringLiteral("rgba(%1, %2, %3, %4)")
        .arg(color.red())
        .arg(color.green())
        .arg(color.blue())
        .arg(color.alpha());
}

}  // namespace markshot::ui
