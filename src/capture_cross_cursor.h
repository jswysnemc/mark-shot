#pragma once

#include <QCursor>

namespace markshot::shot {

/**
 * 创建截图选区与标注共用的高对比十字光标
 * @return 使用 64×64 透明画布且热点位于十字交点的光标
 */
QCursor captureCrossCursor();

}
