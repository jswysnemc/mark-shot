import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Shell from 'gi://Shell';
import St from 'gi://St';

import * as Main from 'resource:///org/gnome/shell/ui/main.js';

const IFACE = 'org.gnome.Shell.Extensions.MarkShotScrollHelper';
const OBJECT_PATH = '/org/gnome/Shell/Extensions/MarkShotScrollHelper';
const VERSION = '2';

const PANEL_WIDTH = 340;
const PANEL_GAP = 14;
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
    </method>
    <method name="HideScrollPreview">
      <arg type="s" name="session_id" direction="in"/>
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
        console.log('[MarkShotScrollHelper] Extension disabled.');
    }

    _handleMethod(methodName, parameters, invocation) {
        try {
            if (methodName === 'Version') {
                invocation.return_value(new GLib.Variant('(s)', [VERSION]));
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

    _panelRectForCapture(rect) {
        const monitor = this._findMonitor(rect);
        const minHeight = CONTROL_BAR_HEIGHT + STATUS_HEIGHT + PANEL_PADDING * 3 + 120;
        let panelHeight = Math.max(minHeight, rect.height);
        panelHeight = Math.min(panelHeight, Math.max(minHeight, monitor.height - 8));

        let top = clamp(rect.y, monitor.y + 4, monitor.y + monitor.height - panelHeight - 4);
        let left = rect.x + rect.width + PANEL_GAP;
        if (left + PANEL_WIDTH > monitor.x + monitor.width - 4) {
            const leftAlt = rect.x - PANEL_GAP - PANEL_WIDTH;
            if (leftAlt >= monitor.x + 4) {
                left = leftAlt;
            } else {
                left = monitor.x + monitor.width - PANEL_WIDTH - 4;
            }
        }
        left = clamp(left, monitor.x + 4, monitor.x + monitor.width - PANEL_WIDTH - 4);
        top = clamp(top, monitor.y + 4, monitor.y + monitor.height - panelHeight - 4);

        return { x: left, y: top, width: PANEL_WIDTH, height: panelHeight };
    }

    _showScrollPreview(state) {
        this._ensurePreview();
        this._previewSessionId = state.sessionId;

        const panel = this._panelRectForCapture(state.rect);
        this._previewRoot.set_position(panel.x, panel.y);
        this._previewRoot.set_size(panel.width, panel.height);

        const imageWidth = panel.width - PANEL_PADDING * 2;
        const imageHeight = Math.max(
            96,
            panel.height - STATUS_HEIGHT - CONTROL_BAR_HEIGHT - PANEL_PADDING * 4
        );
        this._previewImageBin.set_size(imageWidth, imageHeight);

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

    _setButtonIcon(action, iconName, label) {
        const button = this._previewButtons.get(action);
        if (!button) {
            return;
        }
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
        if (!this._previewSessionId) {
            return;
        }
        Gio.DBus.session.emit_signal(
            null,
            OBJECT_PATH,
            IFACE,
            'PreviewAction',
            new GLib.Variant('(ss)', [this._previewSessionId, action])
        );
    }

    _hideScrollPreview(sessionId) {
        if (sessionId && this._previewSessionId && sessionId !== this._previewSessionId) {
            return;
        }
        if (this._previewRoot) {
            this._previewRoot.hide();
            this._disconnectPendingPreviewLoad();
            ++this._previewImageSerial;
            this._previewImageBin.set_child(null);
        }
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
        this._disconnectPendingPreviewLoad();
        ++this._previewImageSerial;
        this._previewSessionId = '';
    }
}
