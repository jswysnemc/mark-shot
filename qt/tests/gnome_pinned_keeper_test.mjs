import assert from 'node:assert/strict';
import {PinnedWindowKeeper} from '../packaging/gnome-extension/mark-shot-scroll-helper@snemc.org/pinned-window-keeper.js';
import {previewPalette, previewPanelStyle, previewButtonStyle} from '../packaging/gnome-extension/mark-shot-scroll-helper@snemc.org/scroll-preview-theme.js';

/** 【贴图测试】【信号】提供可控的 Mutter 信号替身 */
class Signals {
    handlers = new Map();
    nextId = 0;
    /** @param {string} name 信号名称 @param {function} callback 回调 @returns {number} 连接编号 */
    connect(name, callback) {
        this.handlers.set(++this.nextId, {name, callback});
        return this.nextId;
    }
    /** @param {number} id 连接编号 @returns {void} */
    disconnect(id) { this.handlers.delete(id); }
    /** @param {string} name 信号名称 @param {...object} args 信号参数 @returns {void} */
    emit(name, ...args) {
        for (const handler of [...this.handlers.values()]) {
            if (handler.name === name) handler.callback(this, ...args);
        }
    }
}

/** 【贴图测试】【窗口】复现延后映射和置顶状态变化 */
class Window extends Signals {
    above = false;
    raised = 0;
    /** @param {string} title 窗口标题 */
    constructor(title) { super(); this.title = title; }
    /** @returns {string} 当前标题 */
    get_title() { return this.title; }
    /** @returns {boolean} 当前置顶状态 */
    is_above() { return this.above; }
    /** @returns {void} */
    make_above() { this.above = true; this.emit('notify::above'); }
    /** @returns {void} */
    unmake_above() { this.above = false; this.emit('notify::above'); }
    /** @returns {void} */
    raise() { this.raised += 1; }
}

const display = new Signals();
const windows = [];
const keeper = new PinnedWindowKeeper(display, () => windows);
assert.equal(keeper.setAbove('Pinned Mark Shot [one]', true), 0);
const one = new Window('');
windows.push(one);
display.emit('window-created', one);
assert.equal(one.above, false);
one.title = 'Pinned Mark Shot [one]';
one.emit('notify::title');
assert.equal(one.above, true);
one.unmake_above();
assert.equal(one.above, true);

const two = new Window('Pinned Mark Shot [two]');
const settings = new Window('Mark Shot Settings');
windows.push(two, settings);
assert.equal(keeper.setAbove(two.title, true), 1);
assert.equal(two.raised, 1);
assert.equal(settings.above, false);
keeper.setAbove(one.title, false);
assert.equal(one.above, false);
assert.equal(two.above, true);
one.emit('unmanaged');
assert.equal(one.handlers.size, 0);
keeper.destroy();
assert.equal(two.above, false);
assert.equal(display.handlers.size, 0);
assert.equal(two.handlers.size, 0);
for (const dark of [false, true]) {
    const palette = previewPalette(dark);
    assert.ok(previewPanelStyle(palette, 12).includes(palette.panel));
    assert.ok(previewButtonStyle(palette).includes(palette.text));
    assert.ok(previewButtonStyle(palette, true).includes(palette.active));
    assert.notEqual(palette.text, palette.image);
}
console.log('GNOME pinned keeper: late mapping, independent state, recovery and cleanup passed');
