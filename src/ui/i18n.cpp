#include "ui/i18n.h"

#include <QHash>
#include <QLocale>
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

    if (QLocale::system().language() == QLocale::Chinese) {
        return Language::Chinese;
    }
    return Language::English;
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
        {QStringLiteral("Copy Image Text"), QStringLiteral("复制图片文字")},
        {QStringLiteral("Copy Selected Text"), QStringLiteral("复制选中的文字")},
        {QStringLiteral("Translate"), QStringLiteral("翻译")},
        {QStringLiteral("Show Original Text"), QStringLiteral("显示原文")},
        {QStringLiteral("Show Translated Text"), QStringLiteral("显示译文")},
        {QStringLiteral("Save As"), QStringLiteral("另存为")},
        {QStringLiteral("Close"), QStringLiteral("关闭")},

        // Save dialogs.
        {QStringLiteral("Save Pinned Image"), QStringLiteral("保存钉住的图片")},
        {QStringLiteral("Save Screenshot"), QStringLiteral("保存截图")},
        {QStringLiteral("PNG Images (*.png)"), QStringLiteral("PNG 图片 (*.png)")},
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
        {QStringLiteral("Drag to select   Middle switches   Right/Esc cancels"),
         QStringLiteral("拖动选择区域   中键切换全屏标注   右键/Esc 取消")},
        {QStringLiteral("Drag to select   C color picker   R ruler   Q scan   D display   Middle switches   Right/Esc cancels"),
         QStringLiteral("拖动选择区域   C 取色   R 尺子   Q 扫码   D 显示器   中键切换全屏标注   右键/Esc 取消")},
        {QStringLiteral("Color picker: left click samples a pixel   Right/Esc returns"),
         QStringLiteral("取色器：左键采样像素   右键/Esc 返回")},
        {QStringLiteral("Ruler: hover reads coordinates, drag measures pixels   Right/Esc returns"),
         QStringLiteral("尺子：悬停读取坐标，拖动测量像素   右键/Esc 返回")},
        {QStringLiteral("Code scanner: drag to select a code region   Right/Esc returns"),
         QStringLiteral("扫码：拖动选择二维码或条形码区域   右键/Esc 返回")},
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
        {QStringLiteral("Quit"), QStringLiteral("退出")},
        {QStringLiteral("System tray is not available."), QStringLiteral("系统托盘不可用。")},
        {QStringLiteral("System tray is not available on this platform."), QStringLiteral("当前平台不支持系统托盘。")},
        {QStringLiteral("Failed to start single-instance guard: %1"), QStringLiteral("启动单实例守护失败：%1")},
        {QStringLiteral("Global hotkeys are not supported on this platform. "
                        "Use the tray menu or bind a desktop shortcut instead."),
         QStringLiteral("当前平台不支持全局热键。请使用托盘菜单或绑定桌面快捷键。")},
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
