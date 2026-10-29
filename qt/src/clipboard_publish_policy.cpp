#include "clipboard_publish_policy.h"

namespace markshot {

ClipboardPublishPath chooseClipboardPublishPath(bool persistentOwnerAvailable,
                                                bool qtClipboardAvailable)
{
    // 1. Wayland 上必须由独立进程持有选择，否则本进程退出后内容会丢失
    if (persistentOwnerAvailable) {
        return ClipboardPublishPath::PersistentOwner;
    }
    // 2. 没有 wl-copy / xclip 时退回 Qt 剪贴板
    if (qtClipboardAvailable) {
        return ClipboardPublishPath::QtClipboard;
    }
    return ClipboardPublishPath::None;
}

ClipboardPublishPath chooseImageClipboardPublishPath(bool persistentOwnerAvailable,
                                                     bool qtClipboardAvailable,
                                                     bool longLivedProcess)
{
    // 1. 常驻进程自己持有选择，才能同时声明 image/png 与 image/bmp
    if (longLivedProcess && qtClipboardAvailable) {
        return ClipboardPublishPath::QtClipboard;
    }
    // 2. 单次进程沿用文字与链接的发布规则
    return chooseClipboardPublishPath(persistentOwnerAvailable, qtClipboardAvailable);
}

}  // namespace markshot
