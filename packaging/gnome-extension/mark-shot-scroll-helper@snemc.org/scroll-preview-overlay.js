import St from 'gi://St';

import * as Main from 'resource:///org/gnome/shell/ui/main.js';

const FRAME_WIDTH = 3;
const HANDLE_WIDTH = 40;
const HANDLE_HEIGHT = 36;
const HANDLE_GAP = 6;
const PANEL_MARGIN = 4;
const ICON_SIZE = 18;

const ACTION_ICONS = {
    axisVertical: 'go-down-symbolic',
    axisHorizontal: 'go-next-symbolic',
};

function clamp(value, min, max) {
    return Math.max(min, Math.min(max, value));
}

function normalizedRect(rect) {
    return {
        x: Math.round(rect.x),
        y: Math.round(rect.y),
        width: Math.max(0, Math.round(rect.width)),
        height: Math.max(0, Math.round(rect.height)),
    };
}

function fallbackMonitor() {
    return Main.layoutManager.primaryMonitor
        ?? Main.layoutManager.monitors[0]
        ?? {x: 0, y: 0, width: global.stage.width, height: global.stage.height};
}

export class ScrollPreviewOverlay {
    constructor(emitAction) {
        this._emitAction = emitAction;
        this._frameParts = [];
        this._handle = null;
        this._handleAxis = '';
    }

    _ensureActors() {
        if (this._frameParts.length > 0 && this._handle) {
            return;
        }

        while (this._frameParts.length < 4) {
            const part = new St.Widget({
                reactive: false,
                style: 'background-color: rgba(45, 212, 191, 230);',
            });
            Main.layoutManager.uiGroup.add_child(part);
            part.hide();
            this._frameParts.push(part);
        }

        if (!this._handle) {
            this._handle = new St.Button({
                reactive: true,
                can_focus: true,
                accessible_name: 'Direction handle',
                style: [
                    'background-color: rgba(15, 17, 23, 242);',
                    'border: 1px solid rgba(94, 234, 212, 150);',
                    'border-radius: 10px;',
                    'padding: 0;',
                ].join(' '),
            });
            this._handle.connect('clicked', () => {
                this._emitAction?.('axis');
            });
            Main.layoutManager.uiGroup.add_child(this._handle);
            this._handle.hide();
        }
    }

    showFrame(rect) {
        const frame = normalizedRect(rect);
        if (frame.width <= 0 || frame.height <= 0) {
            this.hideFrame();
            return;
        }

        this._ensureActors();
        const parts = [
            {x: frame.x, y: frame.y, width: frame.width, height: FRAME_WIDTH},
            {x: frame.x, y: frame.y + frame.height - FRAME_WIDTH, width: frame.width, height: FRAME_WIDTH},
            {x: frame.x, y: frame.y, width: FRAME_WIDTH, height: frame.height},
            {x: frame.x + frame.width - FRAME_WIDTH, y: frame.y, width: FRAME_WIDTH, height: frame.height},
        ];
        for (let i = 0; i < this._frameParts.length; i += 1) {
            const part = this._frameParts[i];
            const partRect = parts[i];
            part.set_position(partRect.x, partRect.y);
            part.set_size(partRect.width, partRect.height);
            part.show();
        }
    }

    hideFrame() {
        for (const part of this._frameParts) {
            part.hide();
        }
    }

    _handleRectForCapture(rect, monitor) {
        const minLeft = monitor.x + PANEL_MARGIN;
        const maxLeft = Math.max(minLeft, monitor.x + monitor.width - HANDLE_WIDTH - PANEL_MARGIN);
        const minTop = monitor.y + PANEL_MARGIN;
        const maxTop = Math.max(minTop, monitor.y + monitor.height - HANDLE_HEIGHT - PANEL_MARGIN);
        const left = clamp(rect.x, minLeft, maxLeft);
        const bottomTop = rect.y + rect.height + HANDLE_GAP;
        const topTop = rect.y - HANDLE_GAP - HANDLE_HEIGHT;
        if (bottomTop <= maxTop) {
            return {x: left, y: bottomTop, width: HANDLE_WIDTH, height: HANDLE_HEIGHT};
        }
        if (topTop >= minTop) {
            return {x: left, y: topTop, width: HANDLE_WIDTH, height: HANDLE_HEIGHT};
        }
        return {x: left, y: clamp(rect.y, minTop, maxTop), width: HANDLE_WIDTH, height: HANDLE_HEIGHT};
    }

    showHandle(rect, monitor, axis) {
        this._ensureActors();
        const capture = normalizedRect(rect);
        const bounds = monitor ?? fallbackMonitor();
        const handle = this._handleRectForCapture(capture, bounds);
        this._handle.set_position(handle.x, handle.y);
        this._handle.set_size(handle.width, handle.height);

        if (axis !== this._handleAxis) {
            this._handleAxis = axis;
            this._handle.set_child(new St.Icon({
                icon_name: axis === 'horizontal' ? ACTION_ICONS.axisHorizontal : ACTION_ICONS.axisVertical,
                icon_size: ICON_SIZE,
                style: 'color: rgb(229, 231, 235);',
            }));
        }
        this._handle.show();
    }

    hideHandle() {
        this._handle?.hide();
    }

    destroy() {
        for (const part of this._frameParts) {
            part.destroy();
        }
        this._frameParts = [];
        this._handle?.destroy();
        this._handle = null;
        this._handleAxis = '';
    }
}
