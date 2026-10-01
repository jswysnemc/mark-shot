#pragma once

#include <QString>

namespace markshot::portal {

/** 【屏幕共享】【授权存储】按授权范围保存和消费一次性恢复令牌。 */
class ScreenCastRestoreStore final {
public:
    /** @param path 存储文件路径，空值使用当前应用的用户数据目录。 */
    explicit ScreenCastRestoreStore(QString path = {});

    /**
     * 【屏幕共享】【令牌取用】从磁盘移除令牌后再交给调用方，避免重复使用。
     * @param scope 授权范围标识。
     * @param error 可选的存储错误信息，不包含令牌内容。
     * @return 已消费的令牌；无记录或存储失败时返回空字符串。
     */
    QString take(const QString &scope, QString *error = nullptr) const;

    /**
     * 【屏幕共享】【令牌更新】原子保存门户返回的新令牌。
     * @param scope 授权范围标识。
     * @param token 新恢复令牌。
     * @param error 可选的存储错误信息。
     * @return 写入成功时返回 true。
     */
    bool save(const QString &scope, const QString &token, QString *error = nullptr) const;

    /**
     * 【屏幕共享】【令牌失效】删除本次会话保存的令牌，保留其他会话的新记录。
     * @param scope 授权范围标识。
     * @param token 要删除的令牌，只有磁盘记录一致时才删除。
     * @param error 可选的存储错误信息。
     * @return 删除成功或记录已经改变时返回 true。
     */
    bool forget(const QString &scope, const QString &token, QString *error = nullptr) const;

private:
    QString m_path;
};

}  // namespace markshot::portal
