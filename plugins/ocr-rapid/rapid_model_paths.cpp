#include "rapid_model_paths.h"

#include <QDir>
#include <QFileInfo>

#include <algorithm>

namespace markshot::ocr_rapid {
namespace {

/**
 * 追加存在且未重复的目录。
 * @param dirs 目录列表。
 * @param path 待追加路径。
 * @return 无返回值。
 */
void addDir(QStringList *dirs, const QString &path)
{
    const QString absolute = QDir(path).absolutePath();
    if (!absolute.isEmpty() && QDir(absolute).exists() && !dirs->contains(absolute)) {
        dirs->append(absolute);
    }
}

/**
 * 读取用户数据根目录。
 * @return XDG_DATA_HOME 或 ~/.local/share。
 */
QString dataHomeDir()
{
    const QString dataHome = qEnvironmentVariable("XDG_DATA_HOME").trimmed();
    return dataHome.isEmpty() ? QDir::home().filePath(QStringLiteral(".local/share")) : dataHome;
}

/**
 * 在目录列表中查找同时包含全部关键字的 onnx 文件。
 * @param dirs 搜索目录。
 * @param keywords 文件名必须包含的关键字（小写比较）。
 * @return 首个匹配文件路径，找不到返回空串。
 */
QString findModel(const QStringList &dirs, const QStringList &keywords)
{
    for (const QString &dir : dirs) {
        const QFileInfoList entries =
            QDir(dir).entryInfoList({QStringLiteral("*.onnx")}, QDir::Files, QDir::Name);
        for (const QFileInfo &entry : entries) {
            const QString name = entry.fileName().toLower();
            const bool matched = std::all_of(keywords.cbegin(), keywords.cend(),
                                             [&name](const QString &keyword) {
                                                 return name.contains(keyword);
                                             });
            if (matched) {
                return entry.absoluteFilePath();
            }
        }
    }
    return {};
}

/**
 * 在目录列表中查找指定文件名的字典文件。
 * @param dirs 搜索目录。
 * @param fileName 字典文件名。
 * @return 首个匹配文件路径，找不到返回空串。
 */
QString findDictionary(const QStringList &dirs, const QString &fileName)
{
    for (const QString &dir : dirs) {
        const QString path = QDir(dir).filePath(fileName);
        if (QFileInfo::exists(path)) {
            return path;
        }
    }
    return {};
}

}  // namespace

bool RapidModelPaths::isComplete() const
{
    return !detModel.isEmpty() && !recModel.isEmpty() && !recDictionary.isEmpty();
}

QStringList rapidModelSearchDirs()
{
    QStringList dirs;
    // 1. 显式环境变量目录优先
    const QString envDir = qEnvironmentVariable("MARK_SHOT_OCR_MODEL_DIR").trimmed();
    if (!envDir.isEmpty()) {
        addDir(&dirs, envDir);
    }
    // 2. 用户数据模型目录
    addDir(&dirs, QDir(dataHomeDir()).filePath(QStringLiteral("mark-shot/models")));
    // 3. 复用旧 rapidocr venv 已下载的模型，老用户免重复下载
    const QDir venvLib(QDir(dataHomeDir()).filePath(QStringLiteral("mark-shot/ocr-venv/lib")));
    const QStringList pythonDirs =
        venvLib.entryList({QStringLiteral("python*")}, QDir::Dirs, QDir::Name);
    for (const QString &pythonDir : pythonDirs) {
        addDir(&dirs,
               venvLib.filePath(pythonDir + QStringLiteral("/site-packages/rapidocr/models")));
    }
    return dirs;
}

RapidModelPaths locateRapidModels()
{
    RapidModelPaths paths;
    const QStringList dirs = rapidModelSearchDirs();

    // 1. 环境变量可逐项覆盖
    paths.detModel = qEnvironmentVariable("MARK_SHOT_RAPID_DET_MODEL").trimmed();
    paths.recModel = qEnvironmentVariable("MARK_SHOT_RAPID_REC_MODEL").trimmed();
    paths.recDictionary = qEnvironmentVariable("MARK_SHOT_RAPID_REC_DICT").trimmed();

    // 2. 优先 PP-OCRv5，回退任意 det/rec 模型
    if (paths.detModel.isEmpty()) {
        paths.detModel = findModel(dirs, {QStringLiteral("det"), QStringLiteral("v5")});
        if (paths.detModel.isEmpty()) {
            paths.detModel = findModel(dirs, {QStringLiteral("det")});
        }
    }
    if (paths.recModel.isEmpty()) {
        paths.recModel = findModel(dirs, {QStringLiteral("rec"), QStringLiteral("v5")});
        if (paths.recModel.isEmpty()) {
            paths.recModel = findModel(dirs, {QStringLiteral("rec")});
        }
    }

    // 3. 字典按识别模型版本匹配
    if (paths.recDictionary.isEmpty()) {
        const bool recIsV5 = QFileInfo(paths.recModel).fileName().toLower().contains(QStringLiteral("v5"));
        paths.recDictionary = findDictionary(
            dirs, recIsV5 ? QStringLiteral("ppocrv5_dict.txt") : QStringLiteral("ppocr_keys_v1.txt"));
        if (paths.recDictionary.isEmpty()) {
            paths.recDictionary = findDictionary(dirs, QStringLiteral("ppocrv5_dict.txt"));
        }
        if (paths.recDictionary.isEmpty()) {
            paths.recDictionary = findDictionary(dirs, QStringLiteral("ppocr_keys_v1.txt"));
        }
    }
    return paths;
}

}  // namespace markshot::ocr_rapid
