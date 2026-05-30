#include "ui/i18n.h"

#include <QHash>
#include <QLocale>
#include <QProcessEnvironment>

namespace markshot::i18n {

namespace {

Language g_language = Language::English;

Language languageFromString(const QString &raw)
{
    const QString value = raw.trimmed().toLower();
    if (value.startsWith(QStringLiteral("zh")) || value == QStringLiteral("chinese")
        || value == QStringLiteral("cn")) {
        return Language::Chinese;
    }
    return Language::English;
}

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
        {QStringLiteral("Laser"), QStringLiteral("激光笔")},
        {QStringLiteral("Scope"), QStringLiteral("范围")},
        {QStringLiteral("Layout"), QStringLiteral("布局")},
        {QStringLiteral("Clear"), QStringLiteral("清除")},
        {QStringLiteral("Undo"), QStringLiteral("撤销")},
        {QStringLiteral("Redo"), QStringLiteral("重做")},
        {QStringLiteral("Open With"), QStringLiteral("打开方式")},
        {QStringLiteral("Extensions"), QStringLiteral("扩展")},
        {QStringLiteral("Pin"), QStringLiteral("钉住")},
        {QStringLiteral("OCR Copy"), QStringLiteral("OCR 复制")},
        {QStringLiteral("Copy"), QStringLiteral("复制")},
        {QStringLiteral("Save"), QStringLiteral("保存")},
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
        {QStringLiteral("Text font"), QStringLiteral("文字字体")},
        {QStringLiteral("Edit selected text"), QStringLiteral("编辑所选文字")},

        // Text editor.
        {QStringLiteral("Type text"), QStringLiteral("输入文字")},
        {QStringLiteral("Enter inserts newline, click outside commits, Esc cancels"),
         QStringLiteral("回车换行，点击外部确认，Esc 取消")},

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
        {QStringLiteral("OCR timed out"), QStringLiteral("OCR 超时")},
        {QStringLiteral("OCR failed"), QStringLiteral("OCR 失败")},
        {QStringLiteral("No text recognized"), QStringLiteral("未识别到文字")},
        {QStringLiteral("OCR text copied"), QStringLiteral("已复制 OCR 文字")},
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
