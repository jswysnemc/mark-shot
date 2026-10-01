#pragma once

#include "portal/screencast_restore_store.h"
#include "screen_capture.h"

#include <QVariantMap>

namespace markshot::portal {

/** 【屏幕共享】【授权恢复】管理一次门户协商中的令牌消费和轮换。 */
class ScreenCastPersistence final {
public:
    /**
     * 创建与桌面、输出和采集用途绑定的授权上下文。
     * @param request 采集目标和鼠标策略。
     * @param recording 是否为原始帧录制流。
     * @param storePath 可选存储文件路径，默认使用用户数据目录。
     */
    ScreenCastPersistence(const CaptureRequest &request, bool recording, QString storePath = {});

    /**
     * 【屏幕共享】【授权请求】按门户版本生成持久化参数并消费旧令牌。
     * @param portalVersion ScreenCast 接口版本；低于 4 时不启用持久化。
     * @return 可加入 SelectSources 的参数。
     */
    QVariantMap prepare(uint portalVersion);

    /**
     * 【屏幕共享】【令牌轮换】保存 Start 返回的新令牌，空值表示未授予持久授权。
     * @param token 门户返回的一次性恢复令牌。
     * @return 无返回值；存储失败不会中断已授权的截图。
     */
    void accept(const QString &token);

    /**
     * 【屏幕共享】【授权失效】清除不覆盖目标区域的流令牌，下次允许重新选择屏幕。
     * @return 无返回值。
     */
    void invalidate();

private:
    ScreenCastRestoreStore m_store;
    QString m_scope;
    QString m_savedToken;
    bool m_supported = false;
};

}  // namespace markshot::portal
