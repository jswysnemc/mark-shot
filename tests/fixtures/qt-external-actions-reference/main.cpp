/** 【外部动作对照】【原版函数】采集只读提取的生产函数，窗口外围仅提供隔离目录和不执行外部命令的接收端 */
#include <QtWidgets>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QDirIterator>
#include <optional>
#include <iostream>
#include "ui/toolbar_palette.h"

static QString commandsPath;
static QStringList appDirectories;
/** 【外部动作对照】【隔离配置】无参数，返回本次临时扩展配置路径，不读取用户目录 */
static QString extensionCommandsConfigPath() { return commandsPath; }
/** 【外部动作对照】【隔离目录】无参数，返回本次桌面文件目录，不枚举用户应用 */
static QStringList desktopSearchDirs() { return appDirectories; }
#define MS_TR(value) QStringLiteral(value)

namespace markshot { enum class ToolbarDockEdge { Right, Top, Bottom }; }
class ShotWindow : public QWidget {
public:
    struct DesktopApp { QString name; QString desktopPath; QString exec; QString icon; };
    struct ExtensionCommand { QString name; QString command; QString workingDirectory; QString description; bool saveImage=false; bool closeOnStart=true; };
    struct Controls { QWidget *m_openWithPanel=nullptr, *m_extensionPanel=nullptr, *m_toolbar=nullptr, *m_actionToolbar=nullptr; } m_controls;
    bool m_fullscreenAnnotation=false, m_toolbarUserPlaced=false, m_actionToolbarUserPlaced=false;
    std::optional<markshot::ToolbarDockEdge> edge;
    static constexpr qreal kToolbarMargin=8;
    /** 【外部动作对照】【停靠输入】无参数，返回探针显式设置的工具栏停靠边界 */
    std::optional<markshot::ToolbarDockEdge> toolbarDockEdge() const { return edge; }
    QVector<DesktopApp> imageDesktopApps() const;
    QVector<ExtensionCommand> extensionCommands(QString *errorMessage) const;
    void updateOpenWithPanel(); void updateOpenWithPanelGeometry();
    void updateExtensionPanel(); void updateExtensionPanelGeometry();
    QRect sidePanelGeometry(QSize panelSize) const;
    /** 【外部动作对照】【应用接收端】接收列表传入的应用但不执行命令，无返回值 */
    void openSelectionWithDesktop(const DesktopApp &) {}
    /** 【外部动作对照】【扩展接收端】接收列表传入的扩展但不执行命令，无返回值 */
    void runExtensionCommand(const ExtensionCommand &) {}
};

// 1. 【外部动作对照】【生产实现】此标记由采集器替换为原版函数的完整未修改正文
/* PRODUCTION_FUNCTIONS */

/** 【外部动作对照】【范围读取】按 widget 返回原版实际矩形，不改变布局 */
static QJsonArray rectangle(QWidget *widget) { const QRect r=widget->geometry(); return {r.x(),r.y(),r.width(),r.height()}; }

/** 【外部动作对照】【面板采样】按 window、kind、count 读取真实列表尺寸、行和字体，返回 JSON 并保存原始图片 */
static QJsonObject sample(ShotWindow &window, const QString &kind, int count, const QString &root)
{
    QWidget *panel=kind=="open" ? window.m_controls.m_openWithPanel : window.m_controls.m_extensionPanel;
    if (kind=="open") { window.updateOpenWithPanel(); window.updateOpenWithPanelGeometry(); }
    else { window.updateExtensionPanel(); window.updateExtensionPanelGeometry(); }
    panel->show(); window.show(); QApplication::processEvents();
    QJsonObject state{{"kind",kind},{"count",count},{"panel",rectangle(panel)}};
    if (auto *title=panel->findChild<QLabel *>()) state["title"]=QJsonObject{{"text",title->text()},{"rect",rectangle(title)}, {"font_pixels",QFontInfo(title->font()).pixelSize()}};
    if (auto *list=panel->findChild<QListWidget *>()) {
        QJsonArray rows;
        for (int index=0; index<std::min(3,list->count()); ++index) { const QRect r=list->visualItemRect(list->item(index)); rows.append(QJsonArray{r.x(),r.y(),r.width(),r.height()}); }
        state["list"]=QJsonObject{{"rect",rectangle(list)},{"viewport",rectangle(list->viewport())},{"rows",rows},
            {"font_pixels",QFontInfo(list->font()).pixelSize()},{"scroll_max",list->verticalScrollBar()->maximum()}};
    }
    panel->grab().save(root+"/"+kind+QString::number(count)+".png"); panel->hide(); return state;
}

/** 【外部动作对照】【采集入口】按 argv 私有目录调用生产命令和布局函数，返回冻结样本，不启动外部应用 */
int main(int argc, char **argv)
{
    QApplication application(argc,argv);
    QFont font=application.font(); font.setFamily("Noto Sans"); application.setFont(font);
    const QString root=QString::fromLocal8Bit(argv[1]); QDir().mkpath(root);
    QTemporaryDir temporary;
    commandsPath=temporary.path()+"/extensions.json"; appDirectories={temporary.path()+"/applications"}; QDir().mkpath(appDirectories.first());
    ShotWindow window; window.resize(1067,667);
    window.m_controls.m_actionToolbar=new QWidget(&window); window.m_controls.m_actionToolbar->setGeometry(960,220,40,440);
    window.m_controls.m_toolbar=new QWidget(&window); window.m_controls.m_toolbar->setGeometry(300,540,600,40);
    for (const auto &name : {"openWithPanel", "extensionPanel"}) {
        auto *panel=new QWidget(&window); panel->setObjectName(name); panel->setStyleSheet(markshot::theme::openWithPanelStyleSheet({}));
        auto *layout=new QVBoxLayout(panel); layout->setContentsMargins(8,8,8,8); layout->setSpacing(4);
        if (QString(name)=="openWithPanel") window.m_controls.m_openWithPanel=panel; else window.m_controls.m_extensionPanel=panel;
    }
    QJsonArray panels;
    for (int count : {0,1,3,24}) {
        QJsonArray commands;
        for (int index=0;index<count;++index) commands.append(QJsonObject{{"name",QString("Command %1").arg(index)},{"command","viewer {image}"}});
        QFile file(commandsPath); file.open(QIODevice::WriteOnly); file.write(QJsonDocument(commands).toJson()); file.close();
        panels.append(sample(window,"extension",count,root));
        QDir(appDirectories.first()).removeRecursively(); QDir().mkpath(appDirectories.first());
        for (int index=0;index<count;++index) { QFile desktop(appDirectories.first()+QString("/%1.desktop").arg(index)); desktop.open(QIODevice::WriteOnly); desktop.write(QString("[Desktop Entry]\nType=Application\nName=Viewer %1\nExec=viewer %f\nMimeType=image/png\n").arg(index).toUtf8()); }
        panels.append(sample(window,"open",count,root));
    }
    QJsonArray exec;
    for (const auto &command : {"viewer %f", "viewer %U %c %k %i %% %d", "\"/opt/Image Editor\" \"a b\" %F", "viewer --full", "program \"\"\"quoted\"\"\"", "viewer 'a b'"}) {
        ShotWindow::DesktopApp app{"Image Editor","/apps/image editor.desktop",command,{}};
        exec.append(QJsonObject{{"exec",command},{"args",QJsonArray::fromStringList(expandDesktopExec(app,"/tmp/image name.png"))}});
    }
    QJsonArray urls;
    for (const QString &path : {QString("/tmp/image name.png"), QString("/tmp/a b%u'$(echo bad).png"), QString::fromUtf8("/tmp/中文 #?%.png"), QString("/tmp/new\nline.png")})
        urls.append(QJsonObject{{"path",path},{"url",QUrl::fromLocalFile(path).toString()}});
    QJsonArray configurations;
    for (const QString &text : {QString(R"({"name":" One ","command":" echo {slurp} ","cwd":" /tmp ","closeOnStart":false})"),
        QString(R"({"commands":[{"name":"Image","command":"viewer {imageUrl}"},{"name":"Alias","command":"viewer","needsImage":true,"closeOnStart":"false"},17,{"name":42,"command":"bad"}]})"), QString("[]"), QString("{}"), QString("null")}) {
        QFile file(commandsPath); file.open(QIODevice::WriteOnly); file.write(text.toUtf8()); file.close();
        QString error; QJsonArray commands;
        for (const auto &command : window.extensionCommands(&error)) commands.append(QJsonObject{{"name",command.name},{"command",command.command},
            {"working_directory",command.workingDirectory},{"description",command.description},{"save_image",command.saveImage},{"close_on_start",command.closeOnStart}});
        configurations.append(QJsonObject{{"source",text},{"commands",commands},{"error",error}});
    }
    QFile output(root+"/reference.json"); output.open(QIODevice::WriteOnly); output.write(QJsonDocument(QJsonObject{{"qt",qVersion()},{"panels",panels},{"exec",exec},{"urls",urls},{"configurations",configurations}}).toJson());
    std::cout<<panels.size()<<std::endl;
}
