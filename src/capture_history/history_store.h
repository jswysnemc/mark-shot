#pragma once

#include <QDateTime>
#include <QImage>
#include <QJsonObject>
#include <QString>
#include <QVector>

namespace markshot::history {

/// @brief 截图历史的记录策略
struct HistoryConfig {
    bool enabled = true;
    int maximumEntries = 50;
};

/// @brief 历史截图的文件标识和预览信息
struct HistoryEntry {
    QString fileName;
    QDateTime createdAt;
    QSize imageSize;
};

/// @brief 从配置读取历史开关和保留数量
/// @param root 应用配置根对象
/// @return 保留数量限制在 1 至 200 的历史配置
HistoryConfig historyConfigFromRoot(const QJsonObject &root);

/// @brief 管理当前用户的历史截图文件
class HistoryStore final {
public:
    /// @brief 创建历史存储访问器
    /// @param directory 存储目录，空字符串使用应用数据目录
    /// @param byteLimit 历史文件总大小上限，单位为字节
    explicit HistoryStore(QString directory = {}, qint64 byteLimit = 256 * 1024 * 1024);

    /// @brief 返回存储目录
    /// @return 历史截图的绝对目录
    QString directory() const;

    /// @brief 读取可用的截图历史，自动忽略损坏文件与符号链接
    /// @return 按新到旧排序的历史项
    QVector<HistoryEntry> entries() const;

    /// @brief 保存最终截图，并清理超出数量或大小上限的旧记录
    /// @param image 已完成的截图图像
    /// @param limit 最多保留的记录数量
    /// @param error 可选错误信息输出
    /// @return 保存和清理成功时返回 true
    bool append(const QImage &image, int limit = 50, QString *error = nullptr) const;

    /// @brief 读取历史图片，可按目标尺寸读取预览
    /// @param fileName 历史项的文件标识
    /// @param previewSize 预览尺寸，空尺寸读取原图
    /// @return 图片或空图像
    QImage image(const QString &fileName, QSize previewSize = {}) const;

    /// @brief 删除指定历史截图
    /// @param fileName 历史项的文件标识
    /// @param error 可选错误信息输出
    /// @return 删除成功或文件已不存在时返回 true
    bool remove(const QString &fileName, QString *error = nullptr) const;

    /// @brief 清空历史截图，保留目录内其他文件
    /// @param error 可选错误信息输出
    /// @return 所有历史截图删除成功时返回 true
    bool clear(QString *error = nullptr) const;

private:
    QString m_directory;
    qint64 m_byteLimit;
};

/// @brief 按当前配置记录已完成的截图，不影响原有导出结果
/// @param image 已完成的截图图像
/// @return 保存成功或关闭自动记录时返回 true
bool rememberScreenshot(const QImage &image);

}
