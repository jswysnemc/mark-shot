#include "shortcuts/global_shortcut_manager.h"

#include "debug_log.h"
#include "ui/i18n.h"

#if defined(MARK_SHOT_WITH_DBUS)
#include "global_shortcut_portal.h"
#endif
#if defined(HAVE_XCB)
#include "shortcuts/global_shortcut_x11.h"
#endif

#include <QGuiApplication>
#include <QStringList>

namespace markshot::shortcuts {
namespace {

/// @brief X11 原生后端的标识。
const QString kBackendX11 = QStringLiteral("x11");
/// @brief xdg-desktop-portal 后端的标识。
const QString kBackendPortal = QStringLiteral("portal");

#if defined(MARK_SHOT_WITH_DBUS)

/**
 * 把 Portal 后端适配到统一的后端接口。
 */
class PortalBackendAdapter final : public GlobalShortcutBackend {
public:
    PortalBackendAdapter()
        : m_portal(std::make_unique<GlobalShortcutPortal>())
    {
    }

    bool registerShortcuts(const QList<Shortcut> &shortcuts) override
    {
        QList<GlobalShortcutPortal::Shortcut> portalShortcuts;
        portalShortcuts.reserve(shortcuts.size());
        for (const Shortcut &shortcut : shortcuts) {
            portalShortcuts.append(
                {shortcut.id, shortcut.description, shortcut.sequence, shortcut.callback});
        }
        return m_portal->registerShortcuts(portalShortcuts);
    }

    void unregisterShortcuts() override { m_portal->unregisterShortcuts(); }

    QString errorString() const override { return m_portal->errorString(); }

    QString backendName() const override { return kBackendPortal; }

private:
    std::unique_ptr<GlobalShortcutPortal> m_portal;
};

#endif

/**
 * 判断当前是否为 X11 会话。
 *
 * @return Qt 平台插件为 xcb 时返回 true，否则返回 false。
 */
bool isX11Session()
{
    return qGuiApp && qGuiApp->platformName().compare(QStringLiteral("xcb"), Qt::CaseInsensitive) == 0;
}

/**
 * 返回按当前环境排序的后端候选列表。
 *
 * X11 会话优先原生抓键：xdg-desktop-portal 的 GlobalShortcuts 接口面向 Wayland，
 * Cinnamon、Xfce、MATE 等 X11 桌面普遍不实现，直接调用会报接口不存在。
 *
 * @return 后端标识列表，按优先级从高到低排列。
 */
QStringList backendPriority()
{
    if (isX11Session()) {
        return {kBackendX11, kBackendPortal};
    }
    return {kBackendPortal, kBackendX11};
}

}  // namespace

GlobalShortcutManager::GlobalShortcutManager(QObject *parent)
    : QObject(parent)
{
}

GlobalShortcutManager::~GlobalShortcutManager()
{
    unregisterShortcuts();
}

std::unique_ptr<GlobalShortcutBackend> GlobalShortcutManager::createBackend(const QString &name)
{
#if defined(HAVE_XCB)
    if (name == kBackendX11 && X11GlobalShortcutBackend::isAvailable()) {
        return std::make_unique<X11GlobalShortcutBackend>();
    }
#endif
#if defined(MARK_SHOT_WITH_DBUS)
    if (name == kBackendPortal && GlobalShortcutPortal::isAvailable()) {
        return std::make_unique<PortalBackendAdapter>();
    }
#endif
    return nullptr;
}

bool GlobalShortcutManager::isAvailable()
{
#if defined(HAVE_XCB)
    if (X11GlobalShortcutBackend::isAvailable()) {
        return true;
    }
#endif
#if defined(MARK_SHOT_WITH_DBUS)
    if (GlobalShortcutPortal::isAvailable()) {
        return true;
    }
#endif
    return false;
}

bool GlobalShortcutManager::registerShortcuts(const QList<Shortcut> &shortcuts)
{
    m_errorString.clear();
    unregisterShortcuts();

    if (shortcuts.isEmpty()) {
        m_errorString = MS_TR("No global shortcuts are configured.");
        return false;
    }

    // 1. 按优先级依次尝试，首选后端失败时回退到下一个
    QStringList failures;
    for (const QString &name : backendPriority()) {
        std::unique_ptr<GlobalShortcutBackend> backend = createBackend(name);
        if (!backend) {
            continue;
        }
        if (backend->registerShortcuts(shortcuts)) {
            markshot::debugLog("shortcuts",
                               "【全局快捷键】【注册】backend=%s registered %lld shortcuts",
                               name.toUtf8().constData(),
                               static_cast<long long>(shortcuts.size()));
            m_backend = std::move(backend);
            return true;
        }

        QString failure = backend->errorString();
        if (failure.isEmpty()) {
            failure = MS_TR("registration failed");
        }
        failures.append(QStringLiteral("%1: %2").arg(name, failure));
        markshot::debugLog("shortcuts",
                           "【全局快捷键】【注册】backend=%s failed: %s",
                           name.toUtf8().constData(),
                           failure.toUtf8().constData());
    }

    // 2. 全部后端失败时汇总各自的错误，便于用户判断缺哪一环
    if (failures.isEmpty()) {
        m_errorString = MS_TR("Global hotkeys are not supported on this platform. "
                              "Use the tray menu or bind a desktop shortcut instead.");
    } else {
        m_errorString = failures.join(QStringLiteral("; "));
    }
    return false;
}

void GlobalShortcutManager::unregisterShortcuts()
{
    if (m_backend) {
        m_backend->unregisterShortcuts();
        m_backend.reset();
    }
}

QString GlobalShortcutManager::errorString() const
{
    return m_errorString;
}

QString GlobalShortcutManager::activeBackendName() const
{
    return m_backend ? m_backend->backendName() : QString();
}

}  // namespace markshot::shortcuts
