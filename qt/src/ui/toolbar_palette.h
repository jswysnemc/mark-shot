#pragma once

#include "ui/interface_theme_config.h"

#include <QColor>
#include <QString>

namespace markshot::ui {

/// @brief 【截图工具栏】【配色令牌】主工具栏、动作工具栏、属性面板及其下拉面板共用的颜色
///
/// 默认值为暗色玻璃配色；亮色配色由 toolbarPalette 按界面主题生成
struct ToolbarPalette {
    // 面板底板、边框与分组分隔线
    QColor panelFill{15, 17, 23, 230};
    QColor panelBorder{255, 255, 255, 28};
    QColor popupFill{15, 17, 23, 240};
    QColor popupBorder{255, 255, 255, 14};
    QColor separator{148, 163, 184, 70};

    // 拖拽握柄点阵与悬停底板
    QColor gripDot{148, 163, 184, 150};
    QColor gripDotHover{203, 213, 225, 220};
    QColor gripDotDisabled{148, 163, 184, 60};
    QColor gripHoverFill{148, 163, 184, 40};

    // 文字与图标前景
    QColor text{229, 231, 235};
    QColor strongText{248, 250, 252};
    QColor mutedText{156, 163, 175};
    QColor primaryText{255, 255, 255};

    // 普通按钮悬停、按下与选中
    QColor hoverFill{45, 212, 191};
    QColor hoverBorder{94, 234, 212};
    QColor hoverText{4, 47, 46};
    QColor hoverIcon{26, 24, 20};
    QColor pressedFill{20, 184, 166};
    QColor pressedBorder{45, 212, 191};
    QColor activeFill{153, 246, 228, 78};
    QColor activeBorder{153, 246, 228, 150};

    // 主动作、次级强调与危险动作
    QColor primaryTop{16, 185, 129};
    QColor primaryBottom{5, 150, 105};
    QColor primaryBorder{52, 211, 153};
    QColor primaryHoverTop{52, 211, 153};
    QColor primaryHoverBottom{16, 185, 129};
    QColor primaryHoverBorder{110, 231, 183};
    QColor accentFill{45, 212, 191, 34};
    QColor accentBorder{45, 212, 191, 96};
    QColor accentHoverFill{45, 212, 191, 64};
    QColor accentHoverBorder{94, 234, 212, 150};
    QColor dangerHoverText{252, 165, 165};
    QColor dangerHoverFill{239, 68, 68, 70};
    QColor dangerHoverBorder{252, 165, 165, 140};

    // 输入框、下拉框与列表
    QColor controlFill{255, 255, 255, 16};
    QColor controlHoverFill{255, 255, 255, 26};
    QColor controlBorder{255, 255, 255, 24};
    QColor focusBorder{45, 212, 191, 150};
    QColor listFill{17, 24, 39};
    QColor listBorder{255, 255, 255, 18};
    QColor selectionFill{45, 212, 191, 70};
    QColor softHoverFill{45, 212, 191, 28};
    QColor softHoverBorder{45, 212, 191, 80};

    // 滑块
    QColor sliderGroove{255, 255, 255, 22};
    QColor sliderStart{13, 148, 136};
    QColor sliderEnd{45, 212, 191};
    QColor sliderHandle{255, 255, 255};
    QColor sliderHandleBorder{15, 17, 23, 100};
    QColor sliderHandleHoverBorder{94, 234, 212};

    // 形状标记弹层中的网格按钮
    QColor tileFill{30, 41, 59, 220};
    QColor tileBorder{148, 163, 184, 50};
    QColor tileHoverFill{45, 212, 191, 40};
    QColor tileHoverBorder{45, 212, 191, 90};
    QColor tileCheckedTop{94, 234, 212};
    QColor tileCheckedBottom{45, 212, 191};
    QColor tileCheckedBorder{153, 246, 228};
    QColor tileCheckedText{4, 47, 46};

    /// @brief 是否为暗色配色，供自绘菜单等组件选择明暗主题
    bool dark = true;
};

/// @brief 按实际明暗主题生成工具栏配色
/// @param mode 实际应用的主题，Light 返回亮色配色，其余返回暗色配色
/// @return 工具栏配色令牌
ToolbarPalette toolbarPalette(UiThemeMode mode);

/// @brief 将颜色转换为样式表文本，不透明颜色输出大写十六进制，半透明颜色输出 rgba
/// @param color 待转换颜色
/// @return 样式表颜色文本
QString cssColor(const QColor &color);

}  // namespace markshot::ui
