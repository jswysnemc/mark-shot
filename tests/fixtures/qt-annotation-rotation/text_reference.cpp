// 【旋转文字对照】【实际测量】存储宽高、自然字形和比例分别作为输入，不在探针重写测量算法
#include "text_probe.h"
#include "text-production.inc"
#include <QApplication>
#include <QFile>
#include <QFontInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QScreen>

/** 【旋转文字对照】【测量入口】按argc、argv指定输出文件，返回零表示所有生产函数样本已写入 */
int main(int argc, char **argv)
{
    QApplication application(argc, argv);
    if (argc != 2) return 2;
    QJsonArray samples;
    // 1. 【旋转文字对照】【独立维度】每种文本覆盖宽度下限、单行扩宽、三种存储高度及三种字号比例
    for (QString text : {QStringLiteral("Rotation"), QStringLiteral("alpha beta gamma"), QStringLiteral("first line\nnext")}) {
        for (qreal width : {400.0, 180.0, 100.0, 70.0, 40.0, 24.0}) {
            for (qreal height : {8.0, 40.0, 140.0}) {
                for (qreal scale : {1.0, 1.5, 2.0}) {
                    markshot::shot::types::Annotation annotation;
                    annotation.text = text;
                    annotation.rect = QRectF(80.0, 90.0, width, height);
                    const QSizeF size = markshot::shot::textAnnotationBoxSize(annotation, scale);
                    samples.append(QJsonObject{{"text", text}, {"family", annotation.fontFamily},
                        {"stored_width", width}, {"stored_height", height}, {"scale", scale},
                        {"wraps", markshot::shot::textAnnotationWraps(annotation)},
                        {"size", QJsonArray{size.width(), size.height()}}});
                }
            }
        }
    }
    QFile output(QString::fromLocal8Bit(argv[1]));
    if (!output.open(QIODevice::WriteOnly)) return 3;
    output.write(QJsonDocument(QJsonObject{{"qt", QT_VERSION_STR}, {"dpi", application.primaryScreen()->logicalDotsPerInch()},
        {"family", QFontInfo(markshot::theme::textFont(20, QFont::DemiBold, QStringLiteral("Noto Sans"))).family()},
        {"samples", samples}}).toJson());
    return 0;
}
