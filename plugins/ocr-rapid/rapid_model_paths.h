#pragma once

#include <QString>
#include <QStringList>

namespace markshot::ocr_rapid {

struct RapidModelPaths {
    QString detModel;
    QString recModel;
    QString recDictionary;

    /**
     * 判断模型文件是否齐备。
     * @return 检测、识别模型与字典都存在时返回 true。
     */
    bool isComplete() const;
};

/**
 * 读取模型搜索目录列表。
 * @return 依次为环境变量目录、用户数据目录与旧 rapidocr venv 模型目录。
 */
QStringList rapidModelSearchDirs();

/**
 * 在搜索目录中定位 PP-OCR 检测/识别模型与识别字典。
 * @return 模型路径集合，找不到的项为空串。
 */
RapidModelPaths locateRapidModels();

}  // namespace markshot::ocr_rapid
