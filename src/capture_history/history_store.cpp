#include "capture_history/history_store.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QImageReader>
#include <QImageWriter>
#include <QLockFile>
#include <QRegularExpression>
#include <QSaveFile>
#include <QStandardPaths>
#include <QTimeZone>
#include <QUuid>

#include <algorithm>
#include <utility>

namespace markshot::history {
namespace {

constexpr auto kDateFormat = "yyyyMMddHHmmsszzz";
constexpr auto kDprKey = "mark-shot-device-pixel-ratio";

/// @brief 从文件名解析截图时间
/// @param fileName 历史文件标识
/// @return UTC 截图时间，无法解析时返回无效时间
QDateTime entryTime(const QString &fileName)
{
    const QDate date = QDate::fromString(fileName.mid(8, 8), QStringLiteral("yyyyMMdd"));
    const QTime time = QTime::fromString(fileName.mid(16, 9), QStringLiteral("HHmmsszzz"));
    return QDateTime(date, time, QTimeZone("UTC"));
}

/// @brief 检查标识是否属于历史截图，拒绝任意路径和非历史文件
/// @param fileName 待检查的文件标识
/// @return 标识和日期均有效时返回 true
bool validEntryName(const QString &fileName)
{
    static const QRegularExpression pattern(QStringLiteral("^capture-[0-9]{17}-[0-9a-f]{32}\\.png$"));
    return pattern.match(fileName).hasMatch() && entryTime(fileName).isValid();
}

/// @brief 列出历史存储拥有的普通文件
/// @param directory 存储目录
/// @return 按新到旧排序的文件信息
QFileInfoList ownedFiles(const QString &directory)
{
    QFileInfoList result;
    const auto files = QDir(directory).entryInfoList({QStringLiteral("capture-*.png")},
        QDir::Files | QDir::NoSymLinks, QDir::Name | QDir::Reversed);
    for (const QFileInfo &file : files) {
        if (validEntryName(file.fileName())) {
            result.append(file);
        }
    }
    return result;
}

/// @brief 保存错误说明
/// @param error 可选输出指针
/// @param message 错误说明
/// @return false
bool fail(QString *error, const QString &message)
{
    if (error) {
        *error = message;
    }
    return false;
}

}

HistoryConfig historyConfigFromRoot(const QJsonObject &root)
{
    const QJsonObject object = root.value(QStringLiteral("captureHistory")).toObject();
    return {object.value(QStringLiteral("enabled")).toBool(true),
            std::clamp(object.value(QStringLiteral("limit")).toInt(50), 1, 200)};
}

HistoryStore::HistoryStore(QString directory, qint64 byteLimit)
    : m_directory(std::move(directory)), m_byteLimit(std::max<qint64>(1, byteLimit))
{
    if (m_directory.isEmpty()) {
        m_directory = QDir(QStandardPaths::writableLocation(QStandardPaths::AppDataLocation))
                          .filePath(QStringLiteral("capture-history"));
    }
    m_directory = QDir(m_directory).absolutePath();
}

QString HistoryStore::directory() const
{
    return m_directory;
}

QVector<HistoryEntry> HistoryStore::entries() const
{
    QVector<HistoryEntry> result;
    for (const QFileInfo &file : ownedFiles(m_directory)) {
        QImageReader reader(file.absoluteFilePath(), "png");
        const QSize size = reader.size();
        if (reader.canRead() && size.isValid() && !size.isEmpty()) {
            result.append({file.fileName(), entryTime(file.fileName()), size});
        }
    }
    return result;
}

bool HistoryStore::append(const QImage &image, int limit, QString *error) const
{
    if (error) {
        error->clear();
    }
    if (image.isNull() || limit < 1) {
        return fail(error, QStringLiteral("Invalid screenshot or history limit"));
    }

    // 1. 【截图历史】【保存】创建目录并串行化多个进程的写入和清理操作
    const bool existed = QDir(m_directory).exists();
    if (!QDir().mkpath(m_directory)) {
        return fail(error, QStringLiteral("Cannot create screenshot history directory"));
    }
    if (!existed) {
        QFile::setPermissions(m_directory, QFile::ReadOwner | QFile::WriteOwner | QFile::ExeOwner);
    }
    QLockFile lock(QDir(m_directory).filePath(QStringLiteral(".history.lock")));
    if (!lock.tryLock(1000)) {
        return fail(error, QStringLiteral("Screenshot history is busy"));
    }

    // 2. 【截图历史】【保存】生成严格递增的时间标识，避免同毫秒截图排序不稳定
    const auto previous = ownedFiles(m_directory);
    QDateTime timestamp = QDateTime::currentDateTimeUtc();
    if (!previous.isEmpty() && entryTime(previous.first().fileName()) >= timestamp) {
        timestamp = entryTime(previous.first().fileName()).addMSecs(1);
    }
    const QString fileName = QStringLiteral("capture-%1-%2.png")
        .arg(timestamp.toString(QString::fromLatin1(kDateFormat)),
             QUuid::createUuid().toString(QUuid::Id128));
    QSaveFile file(QDir(m_directory).filePath(fileName));
    if (!file.open(QIODevice::WriteOnly)) {
        return fail(error, file.errorString());
    }
    file.setPermissions(QFile::ReadOwner | QFile::WriteOwner);
    QImageWriter writer(&file, "png");
    writer.setCompression(50);
    writer.setText(QString::fromLatin1(kDprKey), QString::number(image.devicePixelRatio()));
    if (!writer.write(image)) {
        return fail(error, writer.errorString());
    }
    if (file.size() > m_byteLimit) {
        return fail(error, QStringLiteral("Screenshot exceeds history storage limit"));
    }
    if (!file.commit()) {
        return fail(error, file.errorString());
    }

    // 3. 【截图历史】【清理】保留最新记录，同时满足数量和总大小限制
    qint64 totalBytes = 0;
    int count = 0;
    for (const QFileInfo &entry : ownedFiles(m_directory)) {
        totalBytes += entry.size();
        if (++count > std::min(limit, 200) || totalBytes > m_byteLimit) {
            if (!QFile::remove(entry.absoluteFilePath())) {
                return fail(error, QStringLiteral("Cannot remove old screenshot history"));
            }
        }
    }
    return true;
}

QImage HistoryStore::image(const QString &fileName, QSize previewSize) const
{
    const QFileInfo file(QDir(m_directory).filePath(fileName));
    if (!validEntryName(fileName) || file.isSymLink() || !file.isFile()) {
        return {};
    }
    QImageReader reader(file.absoluteFilePath(), "png");
    if (previewSize.isValid() && !previewSize.isEmpty()) {
        reader.setScaledSize(reader.size().scaled(previewSize, Qt::KeepAspectRatio));
    }
    QImage result = reader.read();
    bool ok = false;
    const qreal dpr = result.text(QString::fromLatin1(kDprKey)).toDouble(&ok);
    if (ok && dpr >= 1.0 && dpr <= 16.0) {
        result.setDevicePixelRatio(dpr);
    }
    return result;
}

bool HistoryStore::remove(const QString &fileName, QString *error) const
{
    if (error) {
        error->clear();
    }
    const QFileInfo file(QDir(m_directory).filePath(fileName));
    if (!validEntryName(fileName) || file.isSymLink()) {
        return fail(error, QStringLiteral("Invalid screenshot history entry"));
    }
    if (!file.exists()) {
        return true;
    }
    QLockFile lock(QDir(m_directory).filePath(QStringLiteral(".history.lock")));
    if (!lock.tryLock(1000)) {
        return fail(error, QStringLiteral("Screenshot history is busy"));
    }
    return QFile::remove(file.absoluteFilePath())
        || fail(error, QStringLiteral("Cannot remove screenshot history entry"));
}

bool HistoryStore::clear(QString *error) const
{
    if (error) {
        error->clear();
    }
    if (!QDir(m_directory).exists()) {
        return true;
    }
    QLockFile lock(QDir(m_directory).filePath(QStringLiteral(".history.lock")));
    if (!lock.tryLock(1000)) {
        return fail(error, QStringLiteral("Screenshot history is busy"));
    }
    for (const QFileInfo &file : ownedFiles(m_directory)) {
        if (!QFile::remove(file.absoluteFilePath())) {
            return fail(error, QStringLiteral("Cannot clear screenshot history"));
        }
    }
    return true;
}

}
