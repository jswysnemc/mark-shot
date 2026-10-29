use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Context {
    pub window_detection_command: String,
    pub pinned_border_color: String,
    pub windows: bool,
}

/// 根据 context 的平台能力与界面颜色返回默认配置，不读取进程环境或用户文件。
pub fn application(context: &Context) -> Value {
    json!({
        "env": {}, "debug": {"enabled": false, "logPath": ""},
        "annotation": {"defaultTool": "move", "fullscreenDefaultTool": "move", "defaultColor": "#FF4D4D"},
        "toolbar": {"iconSize": "middle", "fontSize": "middle"},
        "save": {"pathTemplate": "{pictures}/mark-shot/mark-shot-{datetime}.png"},
        "export": {"imageFrame": crate::export::FrameSettings::default()},
        "capture": {"wayland": {"kde": {"kwinScreenshot": {"enabled": true}}},
            "doubleClickAction": "copy", "toolbarPosition": "selection",
            "selectionLoupe": {"enabled": false}, "regionDetection": {"enabled": true}},
        "captureHistory": {"enabled": true, "limit": 50},
        "ocrResultWindow": {"alwaysOnTop": false},
        "shortcuts": {"tools": {"pen": "P", "rectangle": "R"},
            "actions": {"copy": "Ctrl+C", "save": "Ctrl+S", "pin": "Ctrl+P"},
            "startup": {"colorPicker": "C", "ruler": "R", "codeScanner": "Q", "displayCapture": "D"}},
        "windows": {"tray": {"enabled": context.windows}, "hotkeys": {"capture": "Ctrl+Alt+S"}},
        "codeScan": {"command": "", "timeoutMs": 15000},
        "pinnedWindow": {"autoOcr": false, "alwaysOnTop": true, "textSelectionCopyEnabled": true,
            "border": true, "borderColor": context.pinned_border_color, "borderWidth": 2},
        "scrollCapture": {"frame": 5, "previewGap": 5, "hidePreviewDuringCapture": false},
        "clipboard": {"image": {"mode": "image/png", "thresholdM": 4}},
        "windowDetection": {"enabled": true, "command": context.window_detection_command, "env": {}, "timeoutMs": 1000},
        "ocr": {"enabled": true, "backend": "rapidocr", "command": "", "timeoutMs": 30000},
        "translation": {"autoAfterOcr": false, "targetLanguage": "Simplified Chinese",
            "apiBase": "https://api.openai.com/v1", "apiKeyEnv": "OPENAI_API_KEY", "apiKey": "",
            "model": "gpt-4o-mini", "temperature": 0.2, "extraBody": {}, "timeoutMs": 60000,
            "timeoutSeconds": 60, "systemPrompt": "", "command": ""}
    })
}
