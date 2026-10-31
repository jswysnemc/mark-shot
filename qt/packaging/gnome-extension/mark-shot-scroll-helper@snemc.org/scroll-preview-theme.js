/**
 * 【滚动截图】【主题】返回与 Qt 工具栏一致的基本配色
 * @param {boolean} dark 是否使用深色主题
 * @returns {object} 预览和按钮所需颜色
 */
export function previewPalette(dark) {
    return dark ? {
        panel: 'rgba(15,17,23,0.95)', border: 'rgba(255,255,255,0.11)',
        text: '#E5E7EB', image: '#111827', control: 'rgba(255,255,255,0.06)',
        active: 'rgba(153,246,228,0.3)', accent: '#5EEAD4',
    } : {
        panel: 'rgba(248,250,252,0.98)', border: '#CBD5E1',
        text: '#334155', image: '#FFFFFF', control: 'rgba(15,23,42,0.04)',
        active: '#CCFBF1', accent: '#0F766E',
    };
}

/**
 * 【滚动截图】【面板主题】生成紧凑预览面板样式
 * @param {object} palette 当前配色
 * @param {number} padding 面板内边距
 * @returns {string} Shell 样式
 */
export function previewPanelStyle(palette, padding) {
    return `background-color: ${palette.panel}; border: 1px solid ${palette.border};`
        + `border-radius: 10px; padding: ${padding}px; spacing: 8px;`;
}

/**
 * 【滚动截图】【按钮主题】为操作按钮和方向握柄生成统一样式
 * @param {object} palette 当前配色
 * @param {boolean} primary 是否为保存动作
 * @returns {string} Shell 样式
 */
export function previewButtonStyle(palette, primary = false) {
    return `color: ${palette.text}; background-color: ${primary ? palette.active : palette.control};`
        + `border: 1px solid ${primary ? palette.accent : palette.border}; border-radius: 8px;`
        + 'min-width: 40px; max-width: 40px; min-height: 36px; max-height: 36px; padding: 0;';
}
