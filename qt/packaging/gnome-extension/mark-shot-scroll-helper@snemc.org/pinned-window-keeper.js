/**
 * 【贴图】【GNOME置顶】跟踪精确标题对应的窗口，补偿窗口尚未映射和置顶状态丢失
 */
export class PinnedWindowKeeper {
    /**
     * @param {object} display Mutter 显示对象
     * @param {function(): object[]} windows 返回当前窗口的函数
     */
    constructor(display, windows) {
        this._display = display;
        this._windows = windows;
        this._requests = new Set();
        this._tracked = new Map();
        this._syncing = false;
        this._created = display.connect('window-created', (_display, window) => this._watch(window));
    }

    /**
     * @param {string} title 单个贴图的唯一标题
     * @param {boolean} above 是否保持置顶
     * @returns {number} 当前匹配的窗口数量
     */
    setAbove(title, above) {
        if (!title?.trim()) {
            return 0;
        }
        if (above) {
            // 1. 【贴图】【请求上限】限制尚未映射窗口的待处理请求数量
            if (!this._requests.has(title) && this._requests.size >= 128) {
                return 0;
            }
            this._requests.add(title);
        } else {
            this._requests.delete(title);
        }
        let count = 0;
        for (const window of this._windows()) {
            if (window.get_title?.() !== title) {
                continue;
            }
            this._watch(window);
            if (above) {
                // 2. 【贴图】【请求置顶】仅调整层级，不激活窗口或抢占键盘焦点
                window.raise?.();
            } else {
                window.unmake_above?.();
                this._tracked.get(window).changed = false;
            }
            count += 1;
        }
        return count;
    }

    /**
     * @param {object} window 新出现或已匹配的窗口
     * @returns {void}
     */
    _watch(window) {
        if (!window || this._tracked.has(window)) {
            this._sync(window);
            return;
        }
        const entry = {signals: [], changed: false};
        this._tracked.set(window, entry);
        entry.signals.push(window.connect('notify::title', () => this._sync(window)));
        entry.signals.push(window.connect('notify::above', () => this._sync(window)));
        entry.signals.push(window.connect('unmanaged', () => {
            this._requests.delete(window.get_title?.());
            this._disconnect(window);
        }));
        this._sync(window);
    }

    /**
     * @param {object} window 待同步的窗口
     * @returns {void}
     */
    _sync(window) {
        if (!window || this._syncing || !this._requests.has(window.get_title?.()) || window.is_above?.()) {
            return;
        }
        this._syncing = true;
        try {
            window.make_above?.();
            this._tracked.get(window).changed = true;
        } finally {
            this._syncing = false;
        }
    }

    /**
     * @param {object} window 不再需要跟踪的窗口
     * @returns {void}
     */
    _disconnect(window) {
        const entry = this._tracked.get(window);
        this._tracked.delete(window);
        for (const signal of entry?.signals ?? []) {
            window.disconnect(signal);
        }
    }

    /**
     * 【贴图】【清理】断开所有信号并撤销扩展施加的置顶状态
     * @returns {void}
     */
    destroy() {
        this._display.disconnect(this._created);
        this._requests.clear();
        for (const [window, entry] of this._tracked) {
            this._disconnect(window);
            if (entry.changed) {
                window.unmake_above?.();
            }
        }
    }
}
