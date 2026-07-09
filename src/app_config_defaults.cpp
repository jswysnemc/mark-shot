#include "app_config_defaults.h"

#include "ui/theme.h"

#include <QColor>
#include <QJsonObject>

namespace markshot {

QJsonObject defaultAppConfigRoot(const QString &windowDetectionCommand)
{
    QJsonObject root;
    root.insert(QStringLiteral("env"), QJsonObject());

    QJsonObject debug;
    debug.insert(QStringLiteral("enabled"), false);
    debug.insert(QStringLiteral("logPath"), QString());
    root.insert(QStringLiteral("debug"), debug);

    QJsonObject annotation;
    annotation.insert(QStringLiteral("defaultTool"), QStringLiteral("pen"));
    annotation.insert(QStringLiteral("fullscreenDefaultTool"), QStringLiteral("pen"));
    annotation.insert(QStringLiteral("defaultColor"), QStringLiteral("#FF4D4D"));
    root.insert(QStringLiteral("annotation"), annotation);

    QJsonObject toolbar;
    toolbar.insert(QStringLiteral("iconSize"), QStringLiteral("middle"));
    toolbar.insert(QStringLiteral("fontSize"), QStringLiteral("middle"));
    root.insert(QStringLiteral("toolbar"), toolbar);

    QJsonObject save;
    save.insert(QStringLiteral("pathTemplate"), QStringLiteral("{pictures}/mark-shot/mark-shot-{datetime}.png"));
    root.insert(QStringLiteral("save"), save);

    QJsonObject imageFrame;
    imageFrame.insert(QStringLiteral("enabled"), false);
    imageFrame.insert(QStringLiteral("padding"), 112);
    imageFrame.insert(QStringLiteral("cornerRadius"), 18);
    imageFrame.insert(QStringLiteral("shadowRadius"), 72);
    imageFrame.insert(QStringLiteral("shadowOffsetY"), 28);
    imageFrame.insert(QStringLiteral("shadowOpacity"), 0.32);

    QJsonObject exportObject;
    exportObject.insert(QStringLiteral("imageFrame"), imageFrame);
    root.insert(QStringLiteral("export"), exportObject);

    QJsonObject kwinScreenshot;
    kwinScreenshot.insert(QStringLiteral("enabled"), true);

    QJsonObject kde;
    kde.insert(QStringLiteral("kwinScreenshot"), kwinScreenshot);

    QJsonObject wayland;
    wayland.insert(QStringLiteral("kde"), kde);

    QJsonObject capture;
    capture.insert(QStringLiteral("wayland"), wayland);
    root.insert(QStringLiteral("capture"), capture);

    QJsonObject shortcutTools;
    shortcutTools.insert(QStringLiteral("pen"), QStringLiteral("P"));
    shortcutTools.insert(QStringLiteral("rectangle"), QStringLiteral("R"));

    QJsonObject shortcutActions;
    shortcutActions.insert(QStringLiteral("copy"), QStringLiteral("Ctrl+C"));
    shortcutActions.insert(QStringLiteral("save"), QStringLiteral("Ctrl+S"));
    shortcutActions.insert(QStringLiteral("pin"), QStringLiteral("Ctrl+P"));

    QJsonObject startupShortcuts;
    startupShortcuts.insert(QStringLiteral("colorPicker"), QStringLiteral("C"));
    startupShortcuts.insert(QStringLiteral("ruler"), QStringLiteral("R"));
    startupShortcuts.insert(QStringLiteral("codeScanner"), QStringLiteral("Q"));
    startupShortcuts.insert(QStringLiteral("displayCapture"), QStringLiteral("D"));

    QJsonObject shortcuts;
    shortcuts.insert(QStringLiteral("tools"), shortcutTools);
    shortcuts.insert(QStringLiteral("actions"), shortcutActions);
    shortcuts.insert(QStringLiteral("startup"), startupShortcuts);
    root.insert(QStringLiteral("shortcuts"), shortcuts);

    QJsonObject tray;
#if defined(Q_OS_WIN)
    tray.insert(QStringLiteral("enabled"), true);
#else
    tray.insert(QStringLiteral("enabled"), false);
#endif

    QJsonObject hotkeys;
    hotkeys.insert(QStringLiteral("capture"), QStringLiteral("Ctrl+Alt+S"));

    QJsonObject windows;
    windows.insert(QStringLiteral("tray"), tray);
    windows.insert(QStringLiteral("hotkeys"), hotkeys);
    root.insert(QStringLiteral("windows"), windows);

    QJsonObject codeScan;
    codeScan.insert(QStringLiteral("command"), QString());
    codeScan.insert(QStringLiteral("timeoutMs"), 15000);
    root.insert(QStringLiteral("codeScan"), codeScan);

    QJsonObject pinnedWindow;
    pinnedWindow.insert(QStringLiteral("autoOcr"), false);
    pinnedWindow.insert(QStringLiteral("alwaysOnTop"), true);
    pinnedWindow.insert(QStringLiteral("textSelectionCopyEnabled"), true);
    pinnedWindow.insert(QStringLiteral("border"), true);
    pinnedWindow.insert(QStringLiteral("borderColor"), theme::kAccent.name(QColor::HexRgb).toUpper());
    pinnedWindow.insert(QStringLiteral("borderWidth"), 2);
    root.insert(QStringLiteral("pinnedWindow"), pinnedWindow);

    QJsonObject scrollCapture;
    scrollCapture.insert(QStringLiteral("frame"), 5);
    scrollCapture.insert(QStringLiteral("previewGap"), 5);
    scrollCapture.insert(QStringLiteral("hidePreviewDuringCapture"), false);
    root.insert(QStringLiteral("scrollCapture"), scrollCapture);

    QJsonObject clipboardImage;
    clipboardImage.insert(QStringLiteral("mode"), QStringLiteral("image/png"));
    clipboardImage.insert(QStringLiteral("thresholdM"), 4);

    QJsonObject clipboard;
    clipboard.insert(QStringLiteral("image"), clipboardImage);
    root.insert(QStringLiteral("clipboard"), clipboard);

    QJsonObject windowDetection;
    windowDetection.insert(QStringLiteral("enabled"), true);
    windowDetection.insert(QStringLiteral("command"), windowDetectionCommand);
    windowDetection.insert(QStringLiteral("env"), QJsonObject());
    windowDetection.insert(QStringLiteral("timeoutMs"), 1000);
    root.insert(QStringLiteral("windowDetection"), windowDetection);

    QJsonObject ocr;
    ocr.insert(QStringLiteral("enabled"), true);
    ocr.insert(QStringLiteral("backend"), QStringLiteral("rapidocr"));
    ocr.insert(QStringLiteral("command"), QString());
    ocr.insert(QStringLiteral("timeoutMs"), 30000);
    root.insert(QStringLiteral("ocr"), ocr);

    QJsonObject translation;
    translation.insert(QStringLiteral("autoAfterOcr"), false);
    translation.insert(QStringLiteral("targetLanguage"), QStringLiteral("Simplified Chinese"));
    translation.insert(QStringLiteral("apiBase"), QStringLiteral("https://api.openai.com/v1"));
    translation.insert(QStringLiteral("apiKeyEnv"), QStringLiteral("OPENAI_API_KEY"));
    translation.insert(QStringLiteral("apiKey"), QString());
    translation.insert(QStringLiteral("model"), QStringLiteral("gpt-4o-mini"));
    translation.insert(QStringLiteral("temperature"), 0.2);
    translation.insert(QStringLiteral("timeoutMs"), 60000);
    translation.insert(QStringLiteral("timeoutSeconds"), 60);
    translation.insert(QStringLiteral("systemPrompt"), QString());
    translation.insert(QStringLiteral("command"), QString());
    root.insert(QStringLiteral("translation"), translation);

    return root;
}

}  // namespace markshot
