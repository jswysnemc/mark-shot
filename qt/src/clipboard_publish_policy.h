#pragma once

namespace markshot {

enum class ClipboardPublishPath {
    PersistentOwner,
    QtClipboard,
    None,
};

/**
 * 选择剪贴板发布路径，避免 Qt 与 wl-copy 争抢同一选择。
 * @param persistentOwnerAvailable 是否能启动持久剪贴板进程。
 * @param qtClipboardAvailable 是否存在 QClipboard。
 * @return 应使用的发布路径。
 */
ClipboardPublishPath chooseClipboardPublishPath(bool persistentOwnerAvailable,
                                                bool qtClipboardAvailable);

/**
 * 选择图片的剪贴板发布路径。
 *
 * 托盘等常驻进程能一直持有剪贴板，交给 QClipboard 可同时提供 PNG 与 BMP；
 * 单次截图进程随后退出，仍由 wl-copy / xclip 持有，只能提供 PNG。
 * @param persistentOwnerAvailable 是否能启动持久剪贴板进程。
 * @param qtClipboardAvailable 是否存在 QClipboard。
 * @param longLivedProcess 当前进程是否会在截图窗口关闭后继续运行。
 * @return 应使用的发布路径。
 */
ClipboardPublishPath chooseImageClipboardPublishPath(bool persistentOwnerAvailable,
                                                     bool qtClipboardAvailable,
                                                     bool longLivedProcess);

}  // namespace markshot
