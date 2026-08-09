#include "scroll/stitcher_long_image.h"

#include <algorithm>
#include <cstring>

namespace markshot::scroll {

namespace {

// 重新分配时在目标高度之外至少追加的空闲行数
constexpr int kMinSpareRows = 256;

}  // namespace

void LongImage::reset(const QImage &seed, const ColSamples &seedCols)
{
    // 种子直接共享像素数据;后续增长会重新分配缓冲,不会写穿共享内存
    m_buffer = seed;
    m_top = 0;
    m_used = seed.isNull() ? 0 : seed.height();
    m_reserveTop = false;
    m_cols = seedCols;
    ++m_revision;
    m_cacheValid = false;
}

void LongImage::clear()
{
    m_buffer = QImage();
    m_top = 0;
    m_used = 0;
    m_reserveTop = false;
    m_cols.clear();
    ++m_revision;
    m_cacheValid = false;
}

bool LongImage::isNull() const
{
    return m_used <= 0 || m_buffer.isNull();
}

int LongImage::width() const
{
    return m_buffer.width();
}

int LongImage::height() const
{
    return m_used;
}

const ColSamples &LongImage::cols() const
{
    return m_cols;
}

const QRgb *LongImage::scanLine(int y) const
{
    if (y < 0 || y >= m_used) {
        return nullptr;
    }
    return reinterpret_cast<const QRgb *>(m_buffer.constScanLine(m_top + y));
}

quint64 LongImage::revision() const
{
    return m_revision;
}

QImage LongImage::image() const
{
    if (isNull()) {
        return QImage();
    }
    // 紧凑拷贝按修订号缓存,预览与导出重复读取时不再逐次复制
    if (!m_cacheValid || m_cacheRevision != m_revision) {
        m_cache = m_buffer.copy(0, m_top, m_buffer.width(), m_used);
        m_cache.setDevicePixelRatio(1.0);
        m_cacheRevision = m_revision;
        m_cacheValid = true;
    }
    return m_cache;
}

void LongImage::ensureRoom(int growTop, int growBottom)
{
    const int capacity = m_buffer.height();
    if (m_top >= growTop && m_top + m_used + growBottom <= capacity) {
        return;
    }

    // 1. 目标高度按 1.5 倍预留增长空间,均摊掉逐帧追加的分配成本
    const int targetUsed = m_used + growTop + growBottom;
    const int newCap = std::max(targetUsed + targetUsed / 2, targetUsed + kMinSpareRows);

    // 2. 发生过前插的会话在顶部保留一部分余量,其余留给底部增长
    const int spare = newCap - targetUsed;
    const int newTop = growTop + ((growTop > 0 || m_reserveTop) ? spare / 4 : 0);

    QImage grown(m_buffer.width(), newCap, QImage::Format_ARGB32_Premultiplied);
    grown.setDevicePixelRatio(1.0);

    // 3. 逐行搬移已用区域,未用行不初始化也不会被读取
    const qsizetype lineBytes = static_cast<qsizetype>(m_buffer.width()) * 4;
    for (int i = 0; i < m_used; ++i) {
        std::memcpy(grown.scanLine(newTop + i), m_buffer.constScanLine(m_top + i), lineBytes);
    }

    m_buffer = std::move(grown);
    m_top = newTop;
}

void LongImage::appendBottom(const QImage &frame, const ColSamples &frameCols, int rows, int trimBottom)
{
    if (isNull() || frame.isNull()) {
        return;
    }
    const int fh = frame.height();
    rows = std::clamp(rows, 0, std::min(fh, static_cast<int>(frameCols.size())));
    trimBottom = std::clamp(trimBottom, 0, std::max(0, m_used - 1));
    if (rows <= 0 && trimBottom <= 0) {
        return;
    }

    // 1. 先裁掉尾部的旧固定带行,签名同步截断
    m_used -= trimBottom;
    m_cols.resize(m_used);

    // 2. 把帧底部 rows 行拷入缓冲尾部,签名直接切片复用帧的采样
    ensureRoom(0, rows);
    const qsizetype lineBytes = static_cast<qsizetype>(m_buffer.width()) * 4;
    for (int i = 0; i < rows; ++i) {
        std::memcpy(m_buffer.scanLine(m_top + m_used + i),
                    frame.constScanLine(fh - rows + i),
                    lineBytes);
        m_cols.append(frameCols[fh - rows + i]);
    }
    m_used += rows;

    ++m_revision;
    m_cacheValid = false;
}

void LongImage::prependTop(const QImage &frame, const ColSamples &frameCols, int rows, int trimTop)
{
    if (isNull() || frame.isNull()) {
        return;
    }
    rows = std::clamp(rows, 0, std::min(frame.height(), static_cast<int>(frameCols.size())));
    trimTop = std::clamp(trimTop, 0, std::max(0, m_used - 1));
    if (rows <= 0 && trimTop <= 0) {
        return;
    }

    // 1. 先裁掉顶部的旧固定带行,签名同步移除
    m_top += trimTop;
    m_used -= trimTop;
    m_cols.remove(0, trimTop);

    // 2. 把帧顶部 rows 行写入使用区上方
    ensureRoom(rows, 0);
    const qsizetype lineBytes = static_cast<qsizetype>(m_buffer.width()) * 4;
    for (int i = 0; i < rows; ++i) {
        std::memcpy(m_buffer.scanLine(m_top - rows + i), frame.constScanLine(i), lineBytes);
    }
    m_top -= rows;
    m_used += rows;
    m_reserveTop = true;

    // 3. 签名整体重建为帧前缀加原有内容
    ColSamples merged;
    merged.reserve(rows + m_cols.size());
    for (int i = 0; i < rows; ++i) {
        merged.append(frameCols[i]);
    }
    merged += m_cols;
    m_cols = std::move(merged);

    ++m_revision;
    m_cacheValid = false;
}

}  // namespace markshot::scroll
