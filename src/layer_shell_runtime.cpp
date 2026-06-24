#include "layer_shell_runtime.h"

#include "debug_log.h"

#include <QCoreApplication>
#include <QDir>
#include <QFileInfo>
#include <QPluginLoader>

#include <memory>

namespace markshot::layershell {
namespace {

/// @brief Gets the platform-specific library name of the layer shell plugin.
/// @return The filename of the plugin.
QString pluginFileName()
{
#if defined(Q_OS_WIN)
    return QStringLiteral("mark-shot-layer-shell.dll");
#elif defined(Q_OS_DARWIN)
    return QStringLiteral("libmark-shot-layer-shell.dylib");
#else
    return QStringLiteral("libmark-shot-layer-shell.so");
#endif
}

/// @brief Adds a search directory to the list if it is not already present.
/// @param dirs Pointer to the list of directory paths.
/// @param path The directory path to clean and add.
void addSearchDir(QStringList *dirs, const QString &path)
{
    if (!dirs || path.isEmpty()) {
        return;
    }

    const QString cleaned = QDir::cleanPath(path);
    if (!dirs->contains(cleaned)) {
        dirs->append(cleaned);
    }
}

/// @brief Generates a list of candidate directories to search for the plugin.
/// @return A list of directory paths to search.
QStringList pluginSearchDirs()
{
    QStringList dirs;
    const QString appDir = QCoreApplication::applicationDirPath();
    addSearchDir(&dirs, appDir);
    addSearchDir(&dirs, QDir(appDir).filePath(QStringLiteral("plugins")));
    addSearchDir(&dirs, QDir(appDir).filePath(QStringLiteral("../lib/mark-shot")));
    addSearchDir(&dirs, QDir(appDir).filePath(QStringLiteral("../lib64/mark-shot")));

    const QStringList libraryPaths = QCoreApplication::libraryPaths();
    for (const QString &path : libraryPaths) {
        addSearchDir(&dirs, path);
        addSearchDir(&dirs, QDir(path).filePath(QStringLiteral("mark-shot")));
    }
    return dirs;
}

/// @brief Dynamically loads the mark-shot layer shell plugin if not already loaded.
/// @return Pointer to the loaded PluginInterface, or nullptr on failure.
PluginInterface *loadPlugin()
{
    /// @brief Struct maintaining the state of the loaded plugin.
    struct LoaderState {
        /// @brief Flag indicating whether a plugin loading attempt has been made.
        bool attempted = false;
        /// @brief Unique pointer to the QPluginLoader used for loading the plugin.
        std::unique_ptr<QPluginLoader> loader;
        /// @brief Pointer to the loaded plugin interface instance.
        PluginInterface *plugin = nullptr;
    };

    static LoaderState state;
    if (state.attempted) {
        return state.plugin;
    }
    state.attempted = true;

    const QString fileName = pluginFileName();
    for (const QString &dir : pluginSearchDirs()) {
        const QString path = QDir(dir).filePath(fileName);
        if (!QFileInfo::exists(path)) {
            continue;
        }

        auto loader = std::make_unique<QPluginLoader>(path);
        QObject *instance = loader->instance();
        if (!instance) {
            markshot::debugLog("layershell",
                               "failed to load optional layer-shell plugin %s: %s",
                               path.toUtf8().constData(),
                               loader->errorString().toUtf8().constData());
            continue;
        }

        PluginInterface *plugin = qobject_cast<PluginInterface *>(instance);
        if (!plugin) {
            markshot::debugLog("layershell",
                               "optional layer-shell plugin %s has an incompatible interface",
                               path.toUtf8().constData());
            loader->unload();
            continue;
        }

        markshot::debugLog("layershell",
                           "loaded optional layer-shell plugin %s",
                           path.toUtf8().constData());
        state.plugin = plugin;
        state.loader = std::move(loader);
        return state.plugin;
    }

    markshot::debugLog("layershell", "optional layer-shell plugin not found");
    return nullptr;
}

} // namespace

bool configureOverlay(QWidget *widget, QScreen *screen, const OverlayConfig &config)
{
    PluginInterface *plugin = loadPlugin();
    return plugin && plugin->configureOverlay(widget, screen, config);
}

bool configureFloatingOverlay(QWidget *widget, QScreen *screen, const FloatingOverlayConfig &config)
{
    PluginInterface *plugin = loadPlugin();
    return plugin && plugin->configureFloatingOverlay(widget, screen, config);
}

bool updateFloatingOverlay(QWidget *widget, QScreen *screen, const FloatingOverlayConfig &config)
{
    PluginInterface *plugin = loadPlugin();
    return plugin && plugin->updateFloatingOverlay(widget, screen, config);
}

bool setLayer(QWidget *widget, Layer layer)
{
    PluginInterface *plugin = loadPlugin();
    return plugin && plugin->setLayer(widget, layer);
}

} // namespace markshot::layershell
