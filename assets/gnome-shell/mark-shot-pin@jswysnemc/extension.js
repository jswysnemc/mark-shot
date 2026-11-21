import GLib from 'gi://GLib';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

const APP_ID = 'mark-shot-pin';

export default class MarkShotPin extends Extension {
    /** 【GNOME贴图】【扩展启用】无参数，监听新窗口并检查已有窗口，无返回值 */
    enable() {
        this._watch = new Map();
        this._created = global.display.connect('window-created', (_display, window) => this._consider(window));
        // 1. 【GNOME贴图】【已有窗口】使用窗口列表接口，避免已废弃的窗口演员列表
        for (const window of global.display.list_all_windows())
            this._consider(window);
    }

    /** 【GNOME贴图】【扩展停用】无参数，移除本扩展的监听及定时器，无返回值 */
    disable() {
        if (this._created)
            global.display.disconnect(this._created);
        this._created = 0;
        for (const window of [...this._watch.keys()])
            this._unwatch(window);
        this._watch.clear();
    }

    /** 【GNOME贴图】【窗口识别】按 window 精确检查应用标识，返回是否完成置顶，已有置顶窗口无需重复操作 */
    _raise(window) {
        if (window.get_wm_class() !== APP_ID)
            return false;
        try {
            if (!window.get_compositor_private()?.mapped)
                return false;
            if (!window.is_above())
                window.make_above();
            return window.is_above();
        } catch (error) {
            console.error(`【GNOME贴图】【置顶失败】${error}`);
            return false;
        }
    }

    /** 【GNOME贴图】【延迟识别】按 window 等待应用标识和映射完成，无返回值，重试限定为六次 */
    _consider(window) {
        if (!window || this._watch.has(window) || this._raise(window))
            return;
        const entry = {signals: [], timer: 0, attempts: 0};
        this._watch.set(window, entry);
        const check = () => {
            if (this._raise(window))
                this._unwatch(window);
        };
        // 1. 【GNOME贴图】【身份延迟】创建通知到达时 app_id 可能为空，等待属性变更而非只匹配一次
        entry.signals.push(window.connect('notify::wm-class', check));
        entry.signals.push(window.connect('notify::title', check));
        entry.signals.push(window.connect('unmanaged', () => this._unwatch(window)));
        // 2. 【GNOME贴图】【映射重试】属性通知缺失时短暂重试，成功后移除监听，保留用户手动取消置顶的选择
        entry.timer = GLib.timeout_add(GLib.PRIORITY_DEFAULT, 400, () => {
            if (this._raise(window) || ++entry.attempts >= 6) {
                entry.timer = 0;
                this._unwatch(window);
                return GLib.SOURCE_REMOVE;
            }
            return GLib.SOURCE_CONTINUE;
        });
    }

    /** 【GNOME贴图】【监听回收】按 window 释放该窗口的信号与定时器，无返回值，不改变窗口置顶状态 */
    _unwatch(window) {
        const entry = this._watch.get(window);
        if (!entry)
            return;
        this._watch.delete(window);
        if (entry.timer)
            GLib.source_remove(entry.timer);
        for (const signal of entry.signals) {
            try {
                window.disconnect(signal);
            } catch (error) {
                console.error(`【GNOME贴图】【监听回收】${error}`);
            }
        }
    }
}
