#pragma once

#include <QImage>
#include <QString>

namespace markshot {

bool copyImageToClipboard(const QImage &image);
bool copyTextToClipboard(const QString &text);

/**
 * 【剪贴板】【常驻持有】声明当前进程会在截图窗口关闭后继续运行。
 *
 * 托盘模式调用后，图片改由本进程通过 QClipboard 持有，可同时提供 image/png 与
 * image/bmp（VirtualBox 共享剪贴板等只识别 BMP）；进程退出前会转交给 wl-copy / xclip。
 * @param longLived 进程是否常驻。
 * @return 无返回值。
 */
void setClipboardOwnershipLongLived(bool longLived);

} // namespace markshot
