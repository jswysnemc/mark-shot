// 【普通标注旋转对照】【生产计算】对完整生产函数生成角度边界、反向缩放及小数比例的实际结果
#include "probe.h"
#include "production.inc"
#include <QFile>
#include <QJsonDocument>

/** 【普通标注旋转对照】【坐标编码】按 value 返回双精度二维坐标，不在采集器中改写几何 */
QJsonArray point(QPointF value) { return {value.x(), value.y()}; }

/** 【普通标注旋转对照】【矩形编码】按 value 返回左、上、宽、高，不执行替代计算 */
QJsonArray rect(QRectF value) { return {value.x(), value.y(), value.width(), value.height()}; }

/** 【普通标注旋转对照】【样本采集】argc、argv 提供输出路径，返回零代表所有结果写入成功 */
int main(int argc, char **argv)
{
    if (argc != 2) return 2;
    QJsonArray angles, drags, resize;
    // 1. 【普通标注旋转对照】【半圈边界】保留归一化和吸附在半圈符号上的实际差别
    for (qreal value : {-1080.0, -721.0, -540.0, -360.0, -180.0, -22.50000001,
                        -22.5, -22.49999999, 0.0, 22.49999999, 22.5, 22.50000001,
                        179.99999999, 180.0, 360.0, 540.0, 721.0, 1080.0}) {
        angles.append(QJsonObject{{"input", value}, {"normalized", normalizedRotationDegrees(value)},
                                  {"snapped", snappedRotationDegrees(value)}});
    }
    // 2. 【普通标注旋转对照】【方向阈值】直接测试0.1图像像素及相邻输入，包含非整数轴心
    for (QPointF center : {QPointF(0, 0), QPointF(123.125, 40.25), QPointF(640.5, 480.75)}) {
        for (qreal radius : {0.0, 0.0999999999, 0.1, 0.1000000001, 1.0, 40.0}) {
            for (QPointF next : {QPointF(0, radius), QPointF(0, -radius), QPointF(-radius, 0), QPointF(radius, radius)}) {
                const QPointF start = center + QPointF(radius, 0), current = center + next;
                const auto result = rotationDragDegrees(center, start, current);
                drags.append(QJsonObject{{"center", point(center)}, {"start", point(start)},
                    {"current", point(current)}, {"degrees", result ? QJsonValue(*result) : QJsonValue(QJsonValue::Null)}});
            }
        }
    }
    ShotWindow window;
    const ShotWindow::SelectionDrag handles[] = {ShotWindow::SelectionDrag::TopLeft,
        ShotWindow::SelectionDrag::Top, ShotWindow::SelectionDrag::TopRight,
        ShotWindow::SelectionDrag::Right, ShotWindow::SelectionDrag::BottomRight,
        ShotWindow::SelectionDrag::Bottom, ShotWindow::SelectionDrag::BottomLeft,
        ShotWindow::SelectionDrag::Left};
    // 3. 【普通标注旋转对照】【八向缩放】覆盖超出画布、跨越对边、最小尺寸及比例约束，不在探针复制缩放算法
    for (QRectF before : {QRectF(120, 90, 160, 80), QRectF(0, 0, 24, 40), QRectF(610, 444, 30, 36)}) {
        for (QPointF pointer : {QPointF(-20, -20), QPointF(0, 0), QPointF(125.125, 93.75),
                               QPointF(200, 130), QPointF(279.5, 169.5), QPointF(400.25, 330.75),
                               QPointF(639.0, 479.0), QPointF(700, 520)}) {
            for (int index = 0; index < 8; ++index) {
                for (bool aspect : {false, true}) {
                    resize.append(QJsonObject{{"before", rect(before)}, {"pointer", point(pointer)},
                        {"clamped", point(window.clampImagePoint(pointer))}, {"handle", index}, {"aspect", aspect},
                        {"result", rect(window.resizedBounds(before, handles[index], pointer, aspect))}});
                }
            }
        }
    }
    QFile output(QString::fromLocal8Bit(argv[1]));
    if (!output.open(QIODevice::WriteOnly)) return 3;
    output.write(QJsonDocument(QJsonObject{{"angles", angles}, {"drags", drags}, {"resize", resize},
                                         {"qt", QT_VERSION_STR}}).toJson());
    return 0;
}
