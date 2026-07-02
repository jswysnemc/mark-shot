#include "ui/i18n.h"

#include "app_config_store.h"
#include "ui/interface_language_config.h"

#include <QHash>
#include <QProcessEnvironment>

namespace markshot::i18n {

namespace {

/// @brief Global language setting for the application interface.
Language g_language = Language::English;

/// @brief Parses a language enum from a raw string value.
/// @param raw The raw language name string.
/// @return The parsed Language enum value.
Language languageFromString(const QString &raw)
{
    const QString value = raw.trimmed().toLower();
    if (value.startsWith(QStringLiteral("zh")) || value == QStringLiteral("chinese")
        || value == QStringLiteral("cn")) {
        return Language::Chinese;
    }
    return Language::English;
}

/// @brief Automatically detects the target system language using environment variables and system locale.
/// @return The detected Language enum value.
Language detectLanguage()
{
    const QProcessEnvironment env = QProcessEnvironment::systemEnvironment();
    const QString override = env.value(QStringLiteral("MARK_SHOT_LANG")).trimmed();
    if (!override.isEmpty()) {
        return languageFromString(override);
    }

    bool ok = false;
    const QJsonObject root = readAppConfigRoot(&ok);
    const markshot::ui::UiLanguageMode mode = ok
        ? markshot::ui::uiLanguageModeFromConfigRoot(root)
        : markshot::ui::UiLanguageMode::System;
    return markshot::ui::languageForUiLanguageMode(mode);
}

// English source string -> Simplified Chinese. Keys must match the source text
// passed to translate() exactly.
/// @brief Returns the lookup table for English to Simplified Chinese translations.
/// @return Reference to the QHash mapping English keys to Chinese values.
const QHash<QString, QString> &chineseTable()
{
    static const QHash<QString, QString> table = {
        // Window titles.
        {QStringLiteral("Mark Shot"), QStringLiteral("Mark Shot")},
        {QStringLiteral("Pinned Mark Shot"), QStringLiteral("钉住的截图")},

        // Pinned-window context menu.
        {QStringLiteral("Rotate Left"), QStringLiteral("向左旋转")},
        {QStringLiteral("Rotate Right"), QStringLiteral("向右旋转")},
        {QStringLiteral("Zoom In"), QStringLiteral("放大")},
        {QStringLiteral("Zoom Out"), QStringLiteral("缩小")},
        {QStringLiteral("Reset Size"), QStringLiteral("重置大小")},
        {QStringLiteral("Always on Top"), QStringLiteral("始终置顶")},
        {QStringLiteral("Copy Image Text"), QStringLiteral("复制图片文字")},
        {QStringLiteral("Copy Selected Text"), QStringLiteral("复制选中的文字")},
        {QStringLiteral("Text Selection Copy"), QStringLiteral("拖选复制文字")},
        {QStringLiteral("Translate"), QStringLiteral("翻译")},
        {QStringLiteral("Show Original Text"), QStringLiteral("显示原文")},
        {QStringLiteral("Show Translated Text"), QStringLiteral("显示译文")},
        {QStringLiteral("Save As"), QStringLiteral("另存为")},
        {QStringLiteral("Close"), QStringLiteral("关闭")},

        // Save dialogs.
        {QStringLiteral("Save Pinned Image"), QStringLiteral("保存钉住的图片")},
        {QStringLiteral("Save Screenshot"), QStringLiteral("保存截图")},
        {QStringLiteral("PNG Images (*.png)"), QStringLiteral("PNG 图片 (*.png)")},
        {QStringLiteral("Screenshot saved"), QStringLiteral("截图已保存")},
        {QStringLiteral("Saved to %1"), QStringLiteral("已保存到 %1")},
        {QStringLiteral("Save failed"), QStringLiteral("保存失败")},

        // Toolbar action names (also used as tooltips).
        {QStringLiteral("Move"), QStringLiteral("移动")},
        {QStringLiteral("Select"), QStringLiteral("选择")},
        {QStringLiteral("Pen"), QStringLiteral("画笔")},
        {QStringLiteral("Line"), QStringLiteral("直线")},
        {QStringLiteral("Highlighter"), QStringLiteral("荧光笔")},
        {QStringLiteral("Rect"), QStringLiteral("矩形")},
        {QStringLiteral("Ellipse"), QStringLiteral("椭圆")},
        {QStringLiteral("Arrow"), QStringLiteral("箭头")},
        {QStringLiteral("Text"), QStringLiteral("文字")},
        {QStringLiteral("Number"), QStringLiteral("序号")},
        {QStringLiteral("Mosaic"), QStringLiteral("马赛克")},
        {QStringLiteral("Magnifier"), QStringLiteral("放大镜")},
        {QStringLiteral("Toggle magnifier shape (circle/rectangle)"), QStringLiteral("切换放大镜形状（圆形/矩形）")},
        {QStringLiteral("Toggle magnifier shape"), QStringLiteral("切换放大镜形状")},
        {QStringLiteral("Laser"), QStringLiteral("激光笔")},
        {QStringLiteral("Scope"), QStringLiteral("范围")},
        {QStringLiteral("Layout"), QStringLiteral("布局")},
        {QStringLiteral("Clear"), QStringLiteral("清除")},
        {QStringLiteral("Undo"), QStringLiteral("撤销")},
        {QStringLiteral("Redo"), QStringLiteral("重做")},
        {QStringLiteral("Open With"), QStringLiteral("打开方式")},
        {QStringLiteral("Extensions"), QStringLiteral("扩展")},
        {QStringLiteral("Pin"), QStringLiteral("钉住")},
        {QStringLiteral("Scroll Capture"), QStringLiteral("滚动截屏")},
        {QStringLiteral("OCR Copy"), QStringLiteral("OCR 复制")},
        {QStringLiteral("Copy"), QStringLiteral("复制")},
        {QStringLiteral("Save"), QStringLiteral("保存")},
        {QStringLiteral("Upload"), QStringLiteral("上传")},
        {QStringLiteral("Cancel"), QStringLiteral("取消")},

        // Property panel.
        {QStringLiteral("Object"), QStringLiteral("对象")},
        {QStringLiteral("Group %1"), QStringLiteral("组合 %1")},
        {QStringLiteral("Width %1"), QStringLiteral("宽度 %1")},
        {QStringLiteral("Opacity %1%"), QStringLiteral("不透明度 %1%")},
        {QStringLiteral("Radius %1"), QStringLiteral("圆角 %1")},
        {QStringLiteral("Font"), QStringLiteral("字体")},
        {QStringLiteral("Bg"), QStringLiteral("背景")},
        {QStringLiteral("Edit"), QStringLiteral("编辑")},
        {QStringLiteral("Selected object width or size"), QStringLiteral("所选对象的宽度或大小")},
        {QStringLiteral("Selected object opacity"), QStringLiteral("所选对象的不透明度")},
        {QStringLiteral("Change selected object color"), QStringLiteral("更改所选对象颜色")},
        {QStringLiteral("Text background color"), QStringLiteral("文字背景颜色")},
        {QStringLiteral("Toggle shape fill"), QStringLiteral("切换形状填充")},
        {QStringLiteral("Rectangle corner radius"), QStringLiteral("矩形圆角半径")},
        {QStringLiteral("Rectangle style"), QStringLiteral("矩形样式")},
        {QStringLiteral("Stroke"), QStringLiteral("描边")},
        {QStringLiteral("Highlight"), QStringLiteral("高亮")},
        {QStringLiteral("Invert"), QStringLiteral("反色")},
        {QStringLiteral("Arrow style"), QStringLiteral("箭头样式")},
        {QStringLiteral("Fletched"), QStringLiteral("燕尾")},
        {QStringLiteral("KDE"), QStringLiteral("KDE")},
        {QStringLiteral("Text font"), QStringLiteral("文字字体")},
        {QStringLiteral("Edit selected text"), QStringLiteral("编辑所选文字")},

        // Text editor.
        {QStringLiteral("Type text"), QStringLiteral("输入文字")},
        {QStringLiteral("Enter inserts newline, click outside commits, Esc cancels"),
         QStringLiteral("回车换行，点击外部确认，Esc 取消")},
        {QStringLiteral("Cut"), QStringLiteral("剪切")},
        {QStringLiteral("Paste"), QStringLiteral("粘贴")},
        {QStringLiteral("Delete"), QStringLiteral("删除")},
        {QStringLiteral("Select All"), QStringLiteral("全选")},

        // Selection overlay hint.
        {QStringLiteral("Drag"), QStringLiteral("拖动")},
        {QStringLiteral("Select screenshot region"), QStringLiteral("选择截图区域")},
        {QStringLiteral("Select code region"), QStringLiteral("选择扫码区域")},
        {QStringLiteral("Return to selection"), QStringLiteral("返回选择")},
        {QStringLiteral("Pick color"), QStringLiteral("取色")},
        {QStringLiteral("Measure size"), QStringLiteral("测量尺寸")},
        {QStringLiteral("Scan QR or barcode"), QStringLiteral("扫描二维码或条形码")},
        {QStringLiteral("Quick display capture"), QStringLiteral("快速截取显示器")},
        {QStringLiteral("Record GIF"), QStringLiteral("录制 GIF")},
        {QStringLiteral("Record video"), QStringLiteral("录制视频")},
        {QStringLiteral("Select recording region"), QStringLiteral("选择录制区域")},
        {QStringLiteral("Middle"), QStringLiteral("中键")},
        {QStringLiteral("Toggle fullscreen annotation"), QStringLiteral("切换全屏标注")},
        {QStringLiteral("Right/Esc"), QStringLiteral("右键/Esc")},
        {QStringLiteral("Color picker: left click samples a pixel   Right/Esc returns"),
         QStringLiteral("取色器：左键采样像素   右键/Esc 返回")},
        {QStringLiteral("Ruler: hover reads coordinates, drag measures pixels   Right/Esc returns"),
         QStringLiteral("尺子：悬停读取坐标，拖动测量像素   右键/Esc 返回")},
        {QStringLiteral("All displays"), QStringLiteral("全部屏幕")},
        {QStringLiteral("Capture every display"), QStringLiteral("捕获所有显示器")},
        {QStringLiteral("Display"), QStringLiteral("显示器")},
        {QStringLiteral("Capture this display"), QStringLiteral("只截取这个显示器")},
        {QStringLiteral("No displays found"), QStringLiteral("未找到显示器")},

        // Open-with / extensions panels.
        {QStringLiteral("Open with"), QStringLiteral("打开方式")},
        {QStringLiteral("No image desktop entries found"), QStringLiteral("未找到可处理图片的应用")},
        {QStringLiteral("No extension commands configured.\nCreate %1"),
         QStringLiteral("未配置扩展命令。\n请创建 %1")},

        // Startup error dialogs.
        {QStringLiteral("Only one image file can be opened at a time."),
         QStringLiteral("一次只能打开一个图片文件。")},
        {QStringLiteral("Image file does not exist: %1"), QStringLiteral("图片文件不存在：%1")},
        {QStringLiteral("Failed to load image: %1\n%2"), QStringLiteral("加载图片失败：%1\n%2")},

        // Toasts.
        {QStringLiteral("OCR helper not found"), QStringLiteral("未找到 OCR 辅助程序")},
        {QStringLiteral("OCR backend not installed. Install rapidocr or tesseract."),
         QStringLiteral("未安装 OCR 后端，请安装 rapidocr 或 tesseract。")},
        {QStringLiteral("OCR timed out"), QStringLiteral("OCR 超时")},
        {QStringLiteral("OCR failed"), QStringLiteral("OCR 失败")},
        {QStringLiteral("No text recognized"), QStringLiteral("未识别到文字")},
        {QStringLiteral("OCR text copied"), QStringLiteral("已复制 OCR 文字")},
        {QStringLiteral("Display image copied"), QStringLiteral("已复制显示器截图")},
        {QStringLiteral("Image URL copied"), QStringLiteral("已复制图片链接")},
        {QStringLiteral("Upload helper not found"), QStringLiteral("未找到上传辅助程序")},
        {QStringLiteral("Upload failed"), QStringLiteral("上传失败")},
        {QStringLiteral("Upload timed out"), QStringLiteral("上传超时")},
        {QStringLiteral("Copy failed"), QStringLiteral("复制失败")},
        {QStringLiteral("Failed to save pinned window setting."), QStringLiteral("保存置顶图片设置失败。")},
        {QStringLiteral("Failed to save pinned text selection setting."), QStringLiteral("保存置顶图片拖选设置失败。")},
        {QStringLiteral("OCR Result"), QStringLiteral("OCR 结果")},
        {QStringLiteral("Review or edit the recognized text before copying."),
         QStringLiteral("复制前可先检查或编辑识别结果。")},
        {QStringLiteral("OCR text appears here"), QStringLiteral("OCR 文字会显示在这里")},
        {QStringLiteral("No text to translate"), QStringLiteral("没有可翻译的文字")},
        {QStringLiteral("Translation failed"), QStringLiteral("翻译失败")},
        {QStringLiteral("Translating..."), QStringLiteral("翻译中...")},
        {QStringLiteral("Target Language"), QStringLiteral("目标语言")},
        {QStringLiteral("Simplified Chinese"), QStringLiteral("简体中文")},
        {QStringLiteral("Traditional Chinese"), QStringLiteral("繁体中文")},
        {QStringLiteral("English"), QStringLiteral("英语")},
        {QStringLiteral("Japanese"), QStringLiteral("日语")},
        {QStringLiteral("Korean"), QStringLiteral("韩语")},
        {QStringLiteral("French"), QStringLiteral("法语")},
        {QStringLiteral("German"), QStringLiteral("德语")},
        {QStringLiteral("Spanish"), QStringLiteral("西班牙语")},
        {QStringLiteral("Russian"), QStringLiteral("俄语")},
        {QStringLiteral("Translating edited OCR text. Keep this panel open."),
         QStringLiteral("正在翻译编辑后的 OCR 文字。请保持此面板打开。")},

        // Scrolling capture session window.
        {QStringLiteral("Scroll capture is not supported on GNOME Wayland."),
         QStringLiteral("GNOME Wayland 暂不支持滚动截图。")},
        {QStringLiteral("Pause"), QStringLiteral("暂停")},
        {QStringLiteral("Resume"), QStringLiteral("继续")},
        {QStringLiteral("Continue Capture"), QStringLiteral("继续捕获")},
        {QStringLiteral("Annotate"), QStringLiteral("标注")},
        {QStringLiteral("Scroll down to capture"), QStringLiteral("向下滚动以捕获")},
        {QStringLiteral("Capturing"), QStringLiteral("正在捕获")},
        {QStringLiteral("Capture paused"), QStringLiteral("捕获已暂停")},
        {QStringLiteral("Waiting for scroll"), QStringLiteral("等待滚动")},
        {QStringLiteral("No overlap match"), QStringLiteral("无重叠匹配")},
        {QStringLiteral("Capture error"), QStringLiteral("捕获错误")},
        {QStringLiteral("Capture"), QStringLiteral("截图")},
        {QStringLiteral("Fullscreen Capture"), QStringLiteral("全屏截图")},
        {QStringLiteral("Failed to start capture session."), QStringLiteral("启动截图会话失败。")},
        {QStringLiteral("Settings"), QStringLiteral("设置")},
        {QStringLiteral("Settings Center"), QStringLiteral("设置中心")},
        {QStringLiteral("General"), QStringLiteral("通用")},
        {QStringLiteral("Interface Language"), QStringLiteral("界面语言")},
        {QStringLiteral("Follow System"), QStringLiteral("跟随系统")},
        {QStringLiteral("Shortcuts"), QStringLiteral("快捷键")},
        {QStringLiteral("Annotation"), QStringLiteral("标注")},
        {QStringLiteral("Integrations"), QStringLiteral("集成")},
        {QStringLiteral("Storage"), QStringLiteral("存储")},
        {QStringLiteral("Advanced"), QStringLiteral("高级")},
        {QStringLiteral("Apply"), QStringLiteral("应用")},
        {QStringLiteral("Some changes take effect after restarting Mark Shot."),
         QStringLiteral("部分设置需要重启 Mark Shot 后生效。")},
        {QStringLiteral("Settings saved. Some changes take effect after restarting Mark Shot."),
         QStringLiteral("设置已保存，部分设置需要重启 Mark Shot 后生效。")},
        {QStringLiteral("Failed to save settings: %1"), QStringLiteral("保存设置失败：%1")},
        {QStringLiteral("Cannot read application config"), QStringLiteral("无法读取应用配置。")},
        {QStringLiteral("Cannot save annotation state"), QStringLiteral("无法保存标注状态。")},
        {QStringLiteral("Configure tray startup and global shortcuts."),
         QStringLiteral("配置托盘启动和全局快捷键。")},
        {QStringLiteral("Configure interface language, tray startup, and global shortcuts."),
         QStringLiteral("配置界面语言、托盘启动和全局快捷键。")},
        {QStringLiteral("Start in Tray"), QStringLiteral("启动到托盘")},
        {QStringLiteral("Launch Mark Shot directly into the system tray."),
         QStringLiteral("启动 Mark Shot 时直接进入系统托盘。")},
        {QStringLiteral("Global Hotkeys"), QStringLiteral("全局快捷键")},
        {QStringLiteral("Register global capture shortcuts when the tray starts."),
         QStringLiteral("托盘启动时注册全局截图快捷键。")},
        {QStringLiteral("Hotkeys"), QStringLiteral("快捷键")},
        {QStringLiteral("Use the desktop portal on supported Linux desktops and RegisterHotKey on Windows."),
         QStringLiteral("支持的 Linux 桌面使用 desktop portal，Windows 使用 RegisterHotKey。")},
        {QStringLiteral("Capture Hotkey"), QStringLiteral("截图快捷键")},
        {QStringLiteral("Fullscreen Hotkey"), QStringLiteral("全屏截图快捷键")},
        {QStringLiteral("Adjust how the frozen screenshot is captured before annotation starts."),
         QStringLiteral("调整标注开始前冻结截图的捕获方式。")},
        {QStringLiteral("Include Cursor"), QStringLiteral("包含鼠标指针")},
        {QStringLiteral("Capture the mouse cursor in the frozen image when supported."),
         QStringLiteral("平台支持时在冻结图中捕获鼠标指针。")},
        {QStringLiteral("Freeze Scope"), QStringLiteral("冻结范围")},
        {QStringLiteral("All Screens"), QStringLiteral("全部屏幕")},
        {QStringLiteral("Cursor Screen"), QStringLiteral("鼠标所在屏幕")},
        {QStringLiteral("KDE KWin Screenshot"), QStringLiteral("KDE KWin 截图")},
        {QStringLiteral("Use KWin ScreenShot2 on KDE Wayland when available."),
         QStringLiteral("KDE Wayland 可用时使用 KWin ScreenShot2。")},
        {QStringLiteral("Tool Shortcuts"), QStringLiteral("工具快捷键")},
        {QStringLiteral("Configure shortcuts used while editing a selected screenshot."),
         QStringLiteral("配置选中截图后的标注工具快捷键。")},
        {QStringLiteral("Action Shortcuts"), QStringLiteral("动作快捷键")},
        {QStringLiteral("Configure screenshot action shortcuts."),
         QStringLiteral("配置截图操作快捷键。")},
        {QStringLiteral("Startup Tool Shortcuts"), QStringLiteral("启动阶段工具快捷键")},
        {QStringLiteral("Configure shortcuts available before a region is selected."),
         QStringLiteral("配置选区前可用的工具快捷键。")},
        {QStringLiteral("Color Picker"), QStringLiteral("取色器")},
        {QStringLiteral("Ruler"), QStringLiteral("尺子")},
        {QStringLiteral("Code Scanner"), QStringLiteral("扫码器")},
        {QStringLiteral("Display Capture"), QStringLiteral("显示器截图")},
        {QStringLiteral("Toggle Capture Scope"), QStringLiteral("切换截图范围")},
        {QStringLiteral("Toggle Toolbar Layout"), QStringLiteral("切换工具栏布局")},
        {QStringLiteral("Annotation Defaults"), QStringLiteral("标注默认值")},
        {QStringLiteral("Set the initial tools and color used by new capture windows."),
         QStringLiteral("设置新截图窗口使用的初始工具和颜色。")},
        {QStringLiteral("Region Tool"), QStringLiteral("区域截图工具")},
        {QStringLiteral("Fullscreen Tool"), QStringLiteral("全屏截图工具")},
        {QStringLiteral("File Tool"), QStringLiteral("文件标注工具")},
        {QStringLiteral("Default Color"), QStringLiteral("默认颜色")},
        {QStringLiteral("Toolbar Appearance"), QStringLiteral("工具栏外观")},
        {QStringLiteral("Change compact toolbar sizes used in capture windows."),
         QStringLiteral("调整截图窗口中紧凑工具栏的尺寸。")},
        {QStringLiteral("Icon Size"), QStringLiteral("图标大小")},
        {QStringLiteral("Font Size"), QStringLiteral("字体大小")},
        {QStringLiteral("Pinned Image"), QStringLiteral("置顶图片")},
        {QStringLiteral("Control pinned image window behavior."),
         QStringLiteral("控制置顶图片窗口行为。")},
        {QStringLiteral("Keep pinned images above normal windows when the platform supports it."),
         QStringLiteral("平台支持时让置顶图片保持在普通窗口上方。")},
        {QStringLiteral("Allow selecting OCR or translated text with the mouse."),
         QStringLiteral("允许用鼠标拖选 OCR 或翻译文本。")},
        {QStringLiteral("Pinned Border"), QStringLiteral("置顶图片边框")},
        {QStringLiteral("Draw a border around pinned images."),
         QStringLiteral("在置顶图片周围绘制边框。")},
        {QStringLiteral("Border Color"), QStringLiteral("边框颜色")},
        {QStringLiteral("Border Width"), QStringLiteral("边框宽度")},
        {QStringLiteral("OCR and Translation"), QStringLiteral("OCR 与翻译")},
        {QStringLiteral("Configure automatic text recognition for pinned images."),
         QStringLiteral("配置置顶图片的自动文字识别。")},
        {QStringLiteral("OCR Enabled"), QStringLiteral("启用 OCR")},
        {QStringLiteral("Enable OCR actions for pinned images."),
         QStringLiteral("启用置顶图片 OCR 操作。")},
        {QStringLiteral("Auto OCR"), QStringLiteral("自动 OCR")},
        {QStringLiteral("Recognize text automatically after pinning."),
         QStringLiteral("钉住图片后自动识别文字。")},
        {QStringLiteral("OCR Backend"), QStringLiteral("OCR 后端")},
        {QStringLiteral("OCR Command"), QStringLiteral("OCR 命令")},
        {QStringLiteral("OCR Timeout"), QStringLiteral("OCR 超时")},
        {QStringLiteral("Auto Translate"), QStringLiteral("自动翻译")},
        {QStringLiteral("Translate automatically after OCR completes."),
         QStringLiteral("OCR 完成后自动翻译。")},
        {QStringLiteral("Translation Command"), QStringLiteral("翻译命令")},
        {QStringLiteral("Translation Timeout"), QStringLiteral("翻译超时")},
        {QStringLiteral("Configure the floating frame and preview panel used for scrolling screenshots."),
         QStringLiteral("配置滚动截图使用的浮动边框和预览面板。")},
        {QStringLiteral("Capture Frame"), QStringLiteral("截图外框")},
        {QStringLiteral("Show an outer frame around the scrolling capture region."),
         QStringLiteral("在滚动截图区域外显示边框。")},
        {QStringLiteral("Frame Gap"), QStringLiteral("外框间距")},
        {QStringLiteral("Preview Gap"), QStringLiteral("预览间距")},
        {QStringLiteral("Hide Preview During Capture"), QStringLiteral("捕获时隐藏预览")},
        {QStringLiteral("Hide the preview panel while each frame is captured."),
         QStringLiteral("每次捕获画面时隐藏预览面板。")},
        {QStringLiteral("Configure the external helper used to recognize QR codes and barcodes."),
         QStringLiteral("配置用于识别二维码和条形码的外部辅助程序。")},
        {QStringLiteral("Scan Command"), QStringLiteral("扫码命令")},
        {QStringLiteral("Scan Timeout"), QStringLiteral("扫码超时")},
        {QStringLiteral("Image Upload"), QStringLiteral("图片上传")},
        {QStringLiteral("Configure the external helper used to upload screenshots."),
         QStringLiteral("配置用于上传截图的外部辅助程序。")},
        {QStringLiteral("Upload Command"), QStringLiteral("上传命令")},
        {QStringLiteral("Upload Timeout"), QStringLiteral("上传超时")},
        {QStringLiteral("Upload Environment"), QStringLiteral("上传环境变量")},
        {QStringLiteral("OCR and Translation Integration"), QStringLiteral("OCR 与翻译集成")},
        {QStringLiteral("Configure OCR result panels and API-based translation helpers."),
         QStringLiteral("配置 OCR 结果面板和基于 API 的翻译辅助程序。")},
        {QStringLiteral("OCR Result Panel"), QStringLiteral("OCR 结果面板")},
        {QStringLiteral("Show an editable OCR result panel before copying text."),
         QStringLiteral("复制文字前显示可编辑的 OCR 结果面板。")},
        {QStringLiteral("Translation API Base"), QStringLiteral("翻译 API 地址")},
        {QStringLiteral("API Key Environment"), QStringLiteral("API Key 环境变量")},
        {QStringLiteral("API Key"), QStringLiteral("API Key")},
        {QStringLiteral("Translation Model"), QStringLiteral("翻译模型")},
        {QStringLiteral("Temperature"), QStringLiteral("温度")},
        {QStringLiteral("System Prompt"), QStringLiteral("系统提示词")},
        {QStringLiteral("Optional translation system prompt."), QStringLiteral("可选的翻译系统提示词。")},
        {QStringLiteral("Debug"), QStringLiteral("调试")},
        {QStringLiteral("Configure diagnostic logging for troubleshooting."),
         QStringLiteral("配置用于排障的诊断日志。")},
        {QStringLiteral("Debug Logging"), QStringLiteral("调试日志")},
        {QStringLiteral("Enable debug log output."), QStringLiteral("启用调试日志输出。")},
        {QStringLiteral("Debug Log Path"), QStringLiteral("调试日志路径")},
        {QStringLiteral("Window Detection"), QStringLiteral("窗口检测")},
        {QStringLiteral("Configure the external helper used to detect windows under the selection."),
         QStringLiteral("配置用于检测选区下方窗口的外部辅助程序。")},
        {QStringLiteral("Window Detection Enabled"), QStringLiteral("启用窗口检测")},
        {QStringLiteral("Run the configured helper before region selection."),
         QStringLiteral("选区前运行已配置的辅助程序。")},
        {QStringLiteral("Window Detection Command"), QStringLiteral("窗口检测命令")},
        {QStringLiteral("Working Directory"), QStringLiteral("工作目录")},
        {QStringLiteral("Window Detection Timeout"), QStringLiteral("窗口检测超时")},
        {QStringLiteral("Window Detection Environment"), QStringLiteral("窗口检测环境变量")},
        {QStringLiteral("Application Environment"), QStringLiteral("应用环境变量")},
        {QStringLiteral("Environment variables applied when Mark Shot starts."),
         QStringLiteral("Mark Shot 启动时应用的环境变量。")},
        {QStringLiteral("Saving"), QStringLiteral("保存")},
        {QStringLiteral("Configure the default file name template for saved screenshots."),
         QStringLiteral("配置保存截图时的默认文件名模板。")},
        {QStringLiteral("Path Template"), QStringLiteral("路径模板")},
        {QStringLiteral("Recording Output"), QStringLiteral("录制输出")},
        {QStringLiteral("Configure where video and GIF recordings are saved."),
         QStringLiteral("配置视频和 GIF 录制文件的保存位置。")},
        {QStringLiteral("Video Directory"), QStringLiteral("视频目录")},
        {QStringLiteral("GIF Directory"), QStringLiteral("GIF 目录")},
        {QStringLiteral("Select Folder"), QStringLiteral("选择文件夹")},
        {QStringLiteral("Clipboard Image"), QStringLiteral("剪贴板图片")},
        {QStringLiteral("Choose how copied images are placed into the clipboard."),
         QStringLiteral("选择复制图片写入剪贴板的方式。")},
        {QStringLiteral("Clipboard Mode"), QStringLiteral("剪贴板模式")},
        {QStringLiteral("PNG Image"), QStringLiteral("PNG 图片")},
        {QStringLiteral("File URL"), QStringLiteral("文件 URL")},
        {QStringLiteral("Auto by Size"), QStringLiteral("按大小自动")},
        {QStringLiteral("Threshold"), QStringLiteral("阈值")},
        {QStringLiteral("Screenshot Export Appearance"), QStringLiteral("截图导出外观")},
        {QStringLiteral("Add a macOS-style transparent canvas and soft shadow to shared screenshots."),
         QStringLiteral("为分享用截图添加类似 macOS 的透明画布和柔和阴影。")},
        {QStringLiteral("Mac-style Frame"), QStringLiteral("Mac 风格外框")},
        {QStringLiteral("Apply only to saved, copied, uploaded, Open With, and extension-command images."),
         QStringLiteral("仅作用于保存、复制、上传、打开方式和扩展命令图片。")},
        {QStringLiteral("Transparent Padding"), QStringLiteral("透明边距")},
        {QStringLiteral("Corner Radius"), QStringLiteral("圆角半径")},
        {QStringLiteral("Shadow Blur"), QStringLiteral("阴影模糊")},
        {QStringLiteral("Shadow Drop"), QStringLiteral("阴影下移")},
        {QStringLiteral("Shadow Opacity"), QStringLiteral("阴影不透明度")},
        {QStringLiteral("Quit"), QStringLiteral("退出")},
        {QStringLiteral("GIF Recording"), QStringLiteral("GIF 录制")},
        {QStringLiteral("Video Recording"), QStringLiteral("视频录制")},
        {QStringLiteral("Recording Type"), QStringLiteral("录制类型")},
        {QStringLiteral("Frame Rate"), QStringLiteral("帧率")},
        {QStringLiteral("%1 fps"), QStringLiteral("%1 帧/秒")},
        {QStringLiteral("Record system default audio input"), QStringLiteral("录制系统默认音频输入")},
        {QStringLiteral("Audio"), QStringLiteral("音频")},
        {QStringLiteral("Record selected display"), QStringLiteral("录制所选显示器")},
        {QStringLiteral("Select region after this dialog"), QStringLiteral("关闭对话框后选择区域")},
        {QStringLiteral("Capture Area"), QStringLiteral("捕获区域")},
        {QStringLiteral("Output"), QStringLiteral("输出")},
        {QStringLiteral("Browse"), QStringLiteral("浏览")},
        {QStringLiteral("FFmpeg"), QStringLiteral("FFmpeg")},
        {QStringLiteral("Start"), QStringLiteral("开始")},
        {QStringLiteral("GIF Images (*.gif)"), QStringLiteral("GIF 图片 (*.gif)")},
        {QStringLiteral("MP4 Videos (*.mp4)"), QStringLiteral("MP4 视频 (*.mp4)")},
        {QStringLiteral("Save Recording"), QStringLiteral("保存录制")},
        {QStringLiteral("Select FFmpeg"), QStringLiteral("选择 FFmpeg")},
        {QStringLiteral("GIF recording does not include audio."), QStringLiteral("GIF 录制不包含音频。")},
        {QStringLiteral("No display is available for recording."), QStringLiteral("没有可用于录制的显示器。")},
        {QStringLiteral("Invalid recording region"), QStringLiteral("录制区域无效")},
        {QStringLiteral("Recording failed to start"), QStringLiteral("录制启动失败")},
        {QStringLiteral("Recording started"), QStringLiteral("录制已开始")},
        {QStringLiteral("Recording saved"), QStringLiteral("录制已保存")},
        {QStringLiteral("Recording failed"), QStringLiteral("录制失败")},
        {QStringLiteral("Recording in Progress"), QStringLiteral("正在录制")},
        {QStringLiteral("Video"), QStringLiteral("视频")},
        {QStringLiteral("%1 recording  %2  %3 fps"), QStringLiteral("%1 录制  %2  %3 帧/秒")},
        {QStringLiteral("Output: %1"), QStringLiteral("输出：%1")},
        {QStringLiteral("Hold S to stop recording, or continue selecting a screenshot region."),
         QStringLiteral("按住 S 停止录制，或继续选择截图区域。")},
        {QStringLiteral("Stop Recording"), QStringLiteral("停止录制")},
        {QStringLiteral("Start Recording"), QStringLiteral("开始录制")},
        {QStringLiteral("Recording: idle"), QStringLiteral("录制：空闲")},
        {QStringLiteral("Recording: %1 %2"), QStringLiteral("录制：%1 %2")},
        {QStringLiteral("Failed to start single-instance guard: %1"), QStringLiteral("启动单实例守护失败：%1")},
        {QStringLiteral("Global hotkeys are not supported on this platform. "
                        "Use the tray menu or bind a desktop shortcut instead."),
         QStringLiteral("当前平台不支持全局热键。请使用托盘菜单或绑定桌面快捷键。")},
        {QStringLiteral("Global shortcuts portal is not available. "
                        "Use the tray menu or bind a desktop shortcut instead."),
         QStringLiteral("当前平台未提供全局快捷键 Portal。请使用托盘菜单或绑定桌面快捷键。")},
        {QStringLiteral("No global shortcuts are configured."), QStringLiteral("未配置全局快捷键。")},
        {QStringLiteral("Failed to create global shortcut session: %1"),
         QStringLiteral("创建全局快捷键会话失败：%1")},
        {QStringLiteral("Global shortcuts portal returned an invalid session."),
         QStringLiteral("全局快捷键 Portal 返回了无效会话。")},
        {QStringLiteral("Failed to bind global shortcuts: %1"),
         QStringLiteral("绑定全局快捷键失败：%1")},
        {QStringLiteral("Global shortcuts portal did not bind any shortcuts."),
         QStringLiteral("全局快捷键 Portal 未绑定任何快捷键。")},
        {QStringLiteral("Failed to connect to global shortcut activation signal."),
         QStringLiteral("连接全局快捷键激活信号失败。")},
        {QStringLiteral("Height %1 px"), QStringLiteral("高度 %1 px")},
        {QStringLiteral("Save Scrolling Screenshot"), QStringLiteral("保存滚动截图")},
        {QStringLiteral("Dir: Vertical"), QStringLiteral("方向：纵向")},
        {QStringLiteral("Dir: Horizontal"), QStringLiteral("方向：横向")},
    };
    return table;
}

}  // namespace

void initialize()
{
    g_language = detectLanguage();
}

void setLanguage(Language language)
{
    g_language = language;
}

Language language()
{
    return g_language;
}

QString translate(const QString &source)
{
    if (g_language == Language::Chinese) {
        const auto it = chineseTable().constFind(source);
        if (it != chineseTable().constEnd()) {
            return it.value();
        }
    }
    return source;
}

}  // namespace markshot::i18n
