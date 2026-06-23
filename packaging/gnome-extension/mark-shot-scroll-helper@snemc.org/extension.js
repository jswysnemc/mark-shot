import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Meta from 'gi://Meta';
import Shell from 'gi://Shell';
import St from 'gi://St';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import { ScrollPreviewOverlay } from './scroll-preview-overlay.js';
const IFACE = 'org.gnome.Shell.Extensions.MarkShotScrollHelper';
const OBJECT_PATH = '/org/gnome/Shell/Extensions/MarkShotScrollHelper';
const API_VERSION = '5';

const PANEL_WIDTH = 340;
const DEFAULT_PREVIEW_GAP = 5;
const PANEL_PADDING = 12;
const CONTROL_BAR_HEIGHT = 54;
const STATUS_HEIGHT = 22;
const ICON_SIZE = 18;

const ACTION_ICONS = {
    axisVertical: 'go-down-symbolic',
    axisHorizontal: 'go-next-symbolic',
    pause: 'media-playback-pause-symbolic',
    resume: 'media-playback-start-symbolic',
    annotate: 'document-edit-symbolic',
    save: 'document-save-symbolic',
    copy: 'edit-copy-symbolic',
    cancel: 'window-close-symbolic',
};

const DBUS_XML = `
<node>
  <interface name="org.gnome.Shell.Extensions.MarkShotScrollHelper">
    <method name="Version">
      <arg type="s" name="version" direction="out"/>
    </method>
    <method name="ScreenshotArea">
      <arg type="i" name="x" direction="in"/>
      <arg type="i" name="y" direction="in"/>
      <arg type="i" name="width" direction="in"/>
      <arg type="i" name="height" direction="in"/>
      <arg type="s" name="filename" direction="in"/>
      <arg type="b" name="success" direction="out"/>
      <arg type="s" name="filename_used" direction="out"/>
    </method>
    <method name="ShowScrollPreview">
      <arg type="s" name="session_id" direction="in"/>
      <arg type="i" name="x" direction="in"/>
      <arg type="i" name="y" direction="in"/>
      <arg type="i" name="width" direction="in"/>
      <arg type="i" name="height" direction="in"/>
      <arg type="s" name="preview_path" direction="in"/>
      <arg type="s" name="status" direction="in"/>
      <arg type="s" name="axis" direction="in"/>
      <arg type="i" name="capture_pos" direction="in"/>
      <arg type="i" name="capture_len" direction="in"/>
      <arg type="i" name="total_len" direction="in"/>
      <arg type="b" name="paused" direction="in"/>
      <arg type="b" name="axis_locked" direction="in"/>
      <arg type="i" name="preview_gap" direction="in"/>
    </method>
    <method name="HideScrollPreview">
      <arg type="s" name="session_id" direction="in"/>
    </method>
    <method name="WindowGeometries">
      <arg type="s" name="json" direction="out"/>
    </method>
    <method name="SetWindowsAbove">
      <arg type="s" name="title" direction="in"/>
      <arg type="b" name="above" direction="in"/>
      <arg type="i" name="changed" direction="out"/>
    </method>
    <signal name="PreviewAction">
      <arg type="s" name="session_id"/>
      <arg type="s" name="action"/>
    </signal>
  </interface>
</node>
`;

function clamp(value, min, max) {
    return Math.max(min, Math.min(max, value));
}

function allowedWindowType(metaWindow) {
    if (!metaWindow?.get_window_type) {
        return true;
    }

    const type = metaWindow.get_window_type();
    return type === Meta.WindowType.NORMAL
        || type === Meta.WindowType.DIALOG
        || type === Meta.WindowType.MODAL_DIALOG
        || type === Meta.WindowType.UTILITY;
}

function windowRect(metaWindow) {
    const rect = metaWindow?.get_frame_rect?.() ?? metaWindow?.get_buffer_rect?.();
    if (!rect || rect.width <= 1 || rect.height <= 1) {
        return null;
    }
    return {
        x: Math.round(rect.x),
        y: Math.round(rect.y),
        width: Math.round(rect.width),
        height: Math.round(rect.height),
    };
}

function rectangleDifference(a, b) {
    const ix = Math.max(a.x, b.x);
    const iy = Math.max(a.y, b.y);
    const ix2 = Math.min(a.x + a.width, b.x + b.width);
    const iy2 = Math.min(a.y + a.height, b.y + b.height);

    if (ix >= ix2 || iy >= iy2) {
        return [a];
    }

    const pieces = [];
    if (ix > a.x) {
        pieces.push({x: a.x, y: a.y, width: ix - a.x, height: a.height});
    }
    if (ix2 < a.x + a.width) {
        pieces.push({x: ix2, y: a.y, width: a.x + a.width - ix2, height: a.height});
    }
    if (iy > a.y) {
        pieces.push({x: ix, y: a.y, width: ix2 - ix, height: iy - a.y});
    }
    if (iy2 < a.y + a.height) {
        pieces.push({x: ix, y: iy2, width: ix2 - ix, height: a.y + a.height - iy2});
    }
    return pieces;
}

function isFullyOccluded(rect, occluders) {
    let remaining = [rect];

    for (const occluder of occluders) {
        const next = [];
        for (const piece of remaining) {
            next.push(...rectangleDifference(piece, occluder));
        }
        remaining = next;
        if (remaining.length === 0) {
            return true;
        }
    }
    return false;
}

function windowVisibleOnCurrentWorkspace(metaWindow, actor) {
    if (!metaWindow) {
        return false;
    }
    if (metaWindow.minimized) {
        return false;
    }
    if (actor && actor.visible === false) {
        return false;
    }
    if (metaWindow.showing_on_its_workspace && !metaWindow.showing_on_its_workspace()) {
        return false;
    }
    if (metaWindow.is_hidden && metaWindow.is_hidden()) {
        return false;
    }
    return true;
}

function windowTitleMatches(metaWindow, title) {
    if (typeof title !== 'string' || title.trim() === '') {
        return false;
    }
    return metaWindow?.get_title?.() === title;
}

function textMatchesMarkShot(text) {
    if (typeof text !== 'string') {
        return false;
    }
    const normalized = text.trim().toLowerCase();
    return normalized === 'mark-shot'
        || normalized === 'mark shot'
        || normalized === 'markshot'
        || normalized.includes('mark shot')
        || normalized.includes('mark-shot');
}

function windowMatchesPinnedRequest(metaWindow, title) {
    if (windowTitleMatches(metaWindow, title)) {
        return true;
    }

    return textMatchesMarkShot(metaWindow?.get_title?.())
        || textMatchesMarkShot(metaWindow?.get_wm_class?.())
        || textMatchesMarkShot(metaWindow?.get_wm_class_instance?.());
}

function setWindowAbove(metaWindow, above) {
    if (!metaWindow) {
        return false;
    }
    if (above && typeof metaWindow.make_above === 'function') {
        metaWindow.make_above();
        return true;
    }
    if (!above && typeof metaWindow.unmake_above === 'function') {
        metaWindow.unmake_above();
        return true;
    }
    return false;
}

function finishScreenshotArea(source, result, stream, filename, invocation) {
    try {
        const finishResult = source.screenshot_area_finish(result);
        const success = Array.isArray(finishResult) ? finishResult[0] : Boolean(finishResult);
        const returnedPath = Array.isArray(finishResult) && typeof finishResult[1] === 'string'
            ? finishResult[1]
            : '';
        const realPath = stream ? filename : returnedPath || filename;
        invocation.return_value(new GLib.Variant('(bs)', [success, realPath]));
    } catch (e) {
        invocation.return_dbus_error(`${IFACE}.Error`, e.message);
    } finally {
        if (stream) {
            try {
                stream.close(null);
            } catch (e) {
                console.log(`[MarkShotScrollHelper] Failed to close screenshot stream: ${e.message}`);
            }
        }
    }
}

export default class MarkShotScrollHelper extends Extension {
    enable() {
        const interfaceInfo = Gio.DBusNodeInfo.new_for_xml(DBUS_XML).interfaces[0];
        this._screenshot = new Shell.Screenshot();
        this._previewSessionId = '';
        this._previewRoot = null;
        this._previewStatus = null;
        this._previewImageBin = null;
        this._previewButtons = new Map();
        this._pendingPreviewTexture = null;
        this._pendingPreviewSignalId = 0;
        this._previewImageSerial = 0;
        this._previewPanelKey = '';
        this._buttonIcons = new Map();
        this._scrollOverlay = new ScrollPreviewOverlay((action) => this._emitPreviewAction(action));

        this._dbusId = Gio.DBus.session.register_object(
            OBJECT_PATH,
            interfaceInfo,
            (connection, sender, objectPath, interfaceName, methodName, parameters, invocation) => {
                this._handleMethod(methodName, parameters, invocation);
            },
            null,
            null
        );

        console.log('[MarkShotScrollHelper] Extension enabled and DBus object exported.');
    }

    disable() {
        this._destroyPreview();
        if (this._dbusId) {
            Gio.DBus.session.unregister_object(this._dbusId);
            this._dbusId = 0;
        }
        this._screenshot = null;
        this._scrollOverlay = null;
        console.log('[MarkShotScrollHelper] Extension disabled.');
    }

    _handleMethod(methodName, parameters, invocation) {
        try {
            if (methodName === 'Version') {
                invocation.return_value(new GLib.Variant('(s)', [API_VERSION]));
                return;
            }

            if (methodName === 'ScreenshotArea') {
                this._handleScreenshotArea(parameters, invocation);
                return;
            }

            if (methodName === 'ShowScrollPreview') {
                this._handleShowScrollPreview(parameters);
                invocation.return_value(null);
                return;
            }

            if (methodName === 'HideScrollPreview') {
                const [sessionId] = parameters.deepUnpack();
                this._hideScrollPreview(sessionId);
                invocation.return_value(null);
                return;
            }

            if (methodName === 'WindowGeometries') {
                invocation.return_value(new GLib.Variant('(s)', [
                    JSON.stringify({
                        compositor: 'gnome',
                        windows: this._windowGeometries(),
                    }),
                ]));
                return;
            }

            if (methodName === 'SetWindowsAbove') {
                const [title, above] = parameters.deepUnpack();
                invocation.return_value(new GLib.Variant('(i)', [
                    this._setWindowsAbove(title, above),
                ]));
                return;
            }

            invocation.return_dbus_error(`${IFACE}.Error`, `Unknown method: ${methodName}`);
        } catch (e) {
            invocation.return_dbus_error(`${IFACE}.Error`, e.message);
        }
    }

    _handleScreenshotArea(parameters, invocation) {
        const screenshot = global.screenshot ?? this._screenshot;
        if (!screenshot?.screenshot_area) {
            invocation.return_dbus_error(`${IFACE}.Error`, 'GNOME Shell screenshot API is unavailable');
            return;
        }

        const [x, y, width, height, filename] = parameters.deepUnpack();

        let stream = null;
        try {
            const file = Gio.File.new_for_path(filename);
            stream = file.replace(null, false, Gio.FileCreateFlags.REPLACE_DESTINATION, null);
            screenshot.screenshot_area(x, y, width, height, stream, (source, result) => {
                finishScreenshotArea(source, result, stream, filename, invocation);
            });
        } catch (e) {
            if (stream) {
                try {
                    stream.close(null);
                } catch (closeError) {
                    console.log(`[MarkShotScrollHelper] Failed to close screenshot stream: ${closeError.message}`);
                }
            }

            try {
                screenshot.screenshot_area(x, y, width, height, filename, (source, result) => {
                    finishScreenshotArea(source, result, null, filename, invocation);
                });
            } catch (legacyError) {
                invocation.return_dbus_error(`${IFACE}.Error`, legacyError.message || e.message);
            }
        }
    }

    _windowGeometries() {
        const actors = global.get_window_actors?.() ?? [];
        const candidates = [];

        // First pass: collect all visible windows
        for (const actor of actors) {
            const metaWindow = actor?.meta_window;
            if (!windowVisibleOnCurrentWorkspace(metaWindow, actor) || !allowedWindowType(metaWindow)) {
                continue;
            }

            const rect = windowRect(metaWindow);
            if (!rect) {
                continue;
            }

            const item = {...rect};
            const title = metaWindow.get_title?.();
            const wmClass = metaWindow.get_wm_class?.();
            const wmClassInstance = metaWindow.get_wm_class_instance?.();
            const monitor = metaWindow.get_monitor?.();
            const workspace = metaWindow.get_workspace?.();
            if (typeof title === 'string' && title) {
                item.title = title;
            }
            if (typeof wmClass === 'string' && wmClass) {
                item.class = wmClass;
            }
            if (typeof wmClassInstance === 'string' && wmClassInstance) {
                item.instance = wmClassInstance;
            }
            if (Number.isInteger(monitor)) {
                item.monitor = monitor;
            }
            if (typeof workspace?.index === 'function') {
                item.workspace = workspace.index();
            }
            candidates.push({rect, item});
        }

        // Second pass: check occlusion back-to-front, keep only partially visible windows
        const windows = [];
        const seen = new Set();
        const occluders = [];

        for (let i = candidates.length - 1; i >= 0; i--) {
            const {rect, item} = candidates[i];
            const key = `${rect.x},${rect.y},${rect.width},${rect.height}`;

            if (seen.has(key)) {
                continue;
            }
            seen.add(key);

            if (!isFullyOccluded(rect, occluders)) {
                windows.push(item);
                occluders.push(rect);
            }
        }

        return windows;
    }

    _setWindowsAbove(title, above) {
        let changed = 0;
        const actors = global.get_window_actors?.() ?? [];

        for (const actor of actors) {
            const metaWindow = actor?.meta_window;
            if (!windowMatchesPinnedRequest(metaWindow, title) || !allowedWindowType(metaWindow)) {
                continue;
            }
            if (setWindowAbove(metaWindow, above)) {
                changed += 1;
            }
        }

        return changed;
    }

    _handleShowScrollPreview(parameters) {
        const [
            sessionId,
            x,
            y,
            width,
            height,
            previewPath,
            status,
            axis,
            capturePos,
            captureLen,
            totalLen,
            paused,
            axisLocked,
            previewGap,
        ] = parameters.deepUnpack();

        this._showScrollPreview({
            sessionId,
            rect: { x, y, width, height },
            previewPath,
            status,
            axis,
            capturePos,
            captureLen,
            totalLen,
            paused,
            axisLocked,
            previewGap,
        });
    }

    _ensurePreview() {
        if (this._previewRoot) {
            return;
        }

        this._previewRoot = new St.BoxLayout({
            vertical: true,
            reactive: true,
            style: [
                'background-color: rgba(15, 17, 23, 242);',
                'border: 1px solid rgba(255, 255, 255, 28);',
                'border-radius: 12px;',
                `padding: ${PANEL_PADDING}px;`,
                'spacing: 10px;',
            ].join(' '),
        });

        this._previewStatus = new St.Label({
            text: 'Scroll down to capture',
            style: [
                'color: rgb(204, 251, 241);',
                'font-weight: 700;',
                'font-size: 10pt;',
            ].join(' '),
        });
        this._previewRoot.add_child(this._previewStatus);

        this._previewImageBin = new St.Bin({
            style: 'background-color: rgba(8, 13, 19, 220);',
            x_expand: true,
            y_expand: true,
        });
        this._previewRoot.add_child(this._previewImageBin);

        const controls = new St.BoxLayout({
            vertical: false,
            style: 'spacing: 7px;',
        });
        this._previewRoot.add_child(controls);

        for (const [action, iconName, label] of [
            ['axis', ACTION_ICONS.axisVertical, 'Vertical'],
            ['pause', ACTION_ICONS.pause, 'Pause'],
            ['annotate', ACTION_ICONS.annotate, 'Annotate'],
            ['save', ACTION_ICONS.save, 'Save'],
            ['copy', ACTION_ICONS.copy, 'Copy'],
            ['cancel', ACTION_ICONS.cancel, 'Close'],
        ]) {
            const button = this._makeButton(action, iconName, label);
            controls.add_child(button);
            this._previewButtons.set(action, button);
        }

        Main.layoutManager.uiGroup.add_child(this._previewRoot);
        this._previewRoot.hide();
    }

    _makeButton(action, iconName, label) {
        const roleStyle = action === 'save'
            ? 'background-color: rgba(45, 212, 191, 92); border-color: rgba(94, 234, 212, 150);'
            : action === 'cancel'
                ? 'background-color: rgba(255, 255, 255, 16); border-color: rgba(248, 113, 113, 70);'
                : 'background-color: rgba(255, 255, 255, 16); border-color: rgba(255, 255, 255, 24);';

        const button = new St.Button({
            reactive: true,
            can_focus: true,
            accessible_name: label,
            style: [
                'color: rgb(229, 231, 235);',
                roleStyle,
                'border-width: 1px;',
                'border-style: solid;',
                'border-radius: 10px;',
                'min-width: 40px;',
                'max-width: 40px;',
                'min-height: 36px;',
                'max-height: 36px;',
                'padding: 0;',
            ].join(' '),
        });
        button.set_child(new St.Icon({
            icon_name: iconName,
            icon_size: ICON_SIZE,
            style: 'color: rgb(229, 231, 235);',
        }));
        button.connect('clicked', () => {
            this._emitPreviewAction(action);
        });
        return button;
    }

    _findMonitor(rect) {
        const centerX = rect.x + rect.width / 2;
        const centerY = rect.y + rect.height / 2;
        for (const monitor of Main.layoutManager.monitors) {
            if (centerX >= monitor.x && centerX < monitor.x + monitor.width
                && centerY >= monitor.y && centerY < monitor.y + monitor.height) {
                return monitor;
            }
        }
        return Main.layoutManager.primaryMonitor
            ?? Main.layoutManager.monitors[0]
            ?? { x: 0, y: 0, width: global.stage.width, height: global.stage.height };
    }

    _panelRectForCapture(rect, previewGap = DEFAULT_PREVIEW_GAP) {
        const monitor = this._findMonitor(rect);
        const minHeight = CONTROL_BAR_HEIGHT + STATUS_HEIGHT + PANEL_PADDING * 3 + 120;
        let panelHeight = Math.max(minHeight, rect.height);
        panelHeight = Math.min(panelHeight, Math.max(minHeight, monitor.height - 8));

        const gap = Math.max(0, previewGap);
        const margin = 4;
        const minLeft = monitor.x + margin;
        const maxLeft = Math.max(minLeft, monitor.x + monitor.width - PANEL_WIDTH - margin);
        const minTop = monitor.y + margin;
        const maxTop = Math.max(minTop, monitor.y + monitor.height - panelHeight - margin);
        const makeRect = (left, top) => ({
            x: clamp(left, minLeft, maxLeft),
            y: clamp(top, minTop, maxTop),
            width: PANEL_WIDTH,
            height: panelHeight,
        });
        const candidates = [
            makeRect(rect.x + rect.width + gap, rect.y),
            makeRect(rect.x - gap - PANEL_WIDTH, rect.y),
            makeRect(rect.x, rect.y + rect.height + gap),
            makeRect(rect.x, rect.y - gap - panelHeight),
            makeRect(rect.x + rect.width + gap, rect.y + rect.height - panelHeight),
            makeRect(rect.x - gap - PANEL_WIDTH, rect.y + rect.height - panelHeight),
            makeRect(rect.x + rect.width - PANEL_WIDTH, rect.y + rect.height + gap),
            makeRect(rect.x + rect.width - PANEL_WIDTH, rect.y - gap - panelHeight),
            makeRect(monitor.x + monitor.width - PANEL_WIDTH - margin, monitor.y + margin),
            makeRect(monitor.x + margin, monitor.y + margin),
            makeRect(monitor.x + monitor.width - PANEL_WIDTH - margin,
                monitor.y + monitor.height - panelHeight - margin),
            makeRect(monitor.x + margin, monitor.y + monitor.height - panelHeight - margin),
        ];

        const intersects = (a, b) => a.x < b.x + b.width
            && a.x + a.width > b.x
            && a.y < b.y + b.height
            && a.y + a.height > b.y;

        for (const candidate of candidates) {
            if (!intersects(candidate, rect)) {
                return candidate;
            }
        }

        const intersectionArea = (candidate) => {
            const left = Math.max(candidate.x, rect.x);
            const top = Math.max(candidate.y, rect.y);
            const right = Math.min(candidate.x + candidate.width, rect.x + rect.width);
            const bottom = Math.min(candidate.y + candidate.height, rect.y + rect.height);
            return Math.max(0, right - left) * Math.max(0, bottom - top);
        };
        return candidates.reduce((best, candidate) =>
            intersectionArea(candidate) < intersectionArea(best) ? candidate : best);
    }

    _showScrollPreview(state) {
        this._previewSessionId = state.sessionId;
        const handleMode = state.previewGap < 0;
        const previewGap = handleMode ? Math.max(0, -state.previewGap - 1) : state.previewGap;

        this._scrollOverlay?.showFrame(state.rect);
        if (handleMode) {
            this._hidePreviewPanel();
            this._scrollOverlay?.showHandle(state.rect, this._findMonitor(state.rect), state.axis);
            return;
        }

        this._scrollOverlay?.hideHandle();
        this._ensurePreview();

        const panel = this._panelRectForCapture(state.rect, previewGap);
        const imageWidth = panel.width - PANEL_PADDING * 2;
        const imageHeight = Math.max(
            96,
            panel.height - STATUS_HEIGHT - CONTROL_BAR_HEIGHT - PANEL_PADDING * 4
        );
        const panelKey = `${panel.x},${panel.y},${panel.width},${panel.height},${imageWidth},${imageHeight}`;
        if (panelKey !== this._previewPanelKey) {
            this._previewPanelKey = panelKey;
            this._previewRoot.set_position(panel.x, panel.y);
            this._previewRoot.set_size(panel.width, panel.height);
            this._previewImageBin.set_size(imageWidth, imageHeight);
        }

        this._previewStatus.set_text(state.status || 'Scroll down to capture');
        this._setButtonIcon(
            'axis',
            state.axis === 'horizontal' ? ACTION_ICONS.axisHorizontal : ACTION_ICONS.axisVertical,
            state.axis === 'horizontal' ? 'Horizontal' : 'Vertical'
        );
        this._setButtonIcon(
            'pause',
            state.paused ? ACTION_ICONS.resume : ACTION_ICONS.pause,
            state.paused ? 'Resume' : 'Pause'
        );
        if (this._previewButtons.has('axis')) {
            const button = this._previewButtons.get('axis');
            button.reactive = !state.axisLocked;
            button.opacity = state.axisLocked ? 120 : 255;
        }

        this._setPreviewImage(state.previewPath, imageWidth, imageHeight);
        this._previewRoot.show();
    }

    _hidePreviewPanel() {
        if (!this._previewRoot) {
            return;
        }
        this._previewRoot.hide();
        this._previewPanelKey = '';
        this._disconnectPendingPreviewLoad();
        ++this._previewImageSerial;
        this._previewImageBin?.set_child(null);
    }

    _setButtonIcon(action, iconName, label) {
        const button = this._previewButtons.get(action);
        if (!button) {
            return;
        }
        const key = `${iconName}:${label}`;
        if (this._buttonIcons.get(action) === key) {
            return;
        }
        this._buttonIcons.set(action, key);
        button.accessible_name = label;
        button.set_child(new St.Icon({
            icon_name: iconName,
            icon_size: ICON_SIZE,
            style: 'color: rgb(229, 231, 235);',
        }));
    }

    _disconnectPendingPreviewLoad() {
        if (this._pendingPreviewTexture && this._pendingPreviewSignalId) {
            try {
                this._pendingPreviewTexture.disconnect(this._pendingPreviewSignalId);
            } catch (e) {
                console.log(`[MarkShotScrollHelper] Failed to disconnect preview load signal: ${e.message}`);
            }
        }
        this._pendingPreviewTexture = null;
        this._pendingPreviewSignalId = 0;
    }

    _setPreviewImage(path, width, height) {
        const serial = ++this._previewImageSerial;
        if (!path) {
            this._disconnectPendingPreviewLoad();
            return;
        }

        const file = Gio.File.new_for_path(path);
        if (!file.query_exists(null)) {
            return;
        }

        const texture = St.TextureCache.get_default().load_file_async(file, width, height, 1, 1);
        texture.set_size(width, height);

        const commitTexture = () => {
            if (serial !== this._previewImageSerial || !this._previewImageBin) {
                return;
            }
            this._disconnectPendingPreviewLoad();
            this._previewImageBin.set_child(texture);
        };

        if (texture.content) {
            commitTexture();
            return;
        }

        this._disconnectPendingPreviewLoad();
        this._pendingPreviewTexture = texture;
        this._pendingPreviewSignalId = texture.connect('notify::content', () => {
            if (texture.content) {
                commitTexture();
            }
        });
    }

    _emitPreviewAction(action) {
        const sessionId = this._previewSessionId;
        if (!sessionId) {
            return;
        }
        Gio.DBus.session.emit_signal(
            null,
            OBJECT_PATH,
            IFACE,
            'PreviewAction',
            new GLib.Variant('(ss)', [sessionId, action])
        );
    }

    _hideScrollPreview(sessionId) {
        if (sessionId && this._previewSessionId && sessionId !== this._previewSessionId) {
            return;
        }
        if (this._previewRoot) {
            this._hidePreviewPanel();
        }
        this._scrollOverlay?.hideFrame();
        this._scrollOverlay?.hideHandle();
        this._previewSessionId = '';
    }

    _destroyPreview() {
        if (this._previewRoot) {
            this._previewRoot.destroy();
            this._previewRoot = null;
        }
        this._previewStatus = null;
        this._previewImageBin = null;
        this._previewButtons = new Map();
        this._buttonIcons = new Map();
        this._previewPanelKey = '';
        this._disconnectPendingPreviewLoad();
        this._scrollOverlay?.destroy();
        ++this._previewImageSerial;
        this._previewSessionId = '';
    }
}
