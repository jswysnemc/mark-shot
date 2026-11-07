#include "probe.h"
#include "constants.inc"
#include "production.inc"

#include <QCryptographicHash>
#include <QDir>
#include <QFile>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>

/// 【样式对照】【图像指纹】按path返回文件SHA256字符串，读取失败直接退出
QString digest(const QString &path) {
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly)) std::abort();
    return QString::fromLatin1(QCryptographicHash::hash(file.readAll(), QCryptographicHash::Sha256).toHex());
}

/// 【样式对照】【像素记录】按image、device与center返回水平和垂直像素扫描，记录实际生产绘制输出
QJsonObject pixels(const QImage &image, qreal device, QPointF center) {
    const int x = qRound(center.x() * device);
    const int y = qRound(center.y() * device);
    QJsonArray horizontal, vertical;
    for (int i = 0; i < image.width(); ++i) horizontal.append(image.pixelColor(i, y).name(QColor::HexArgb));
    for (int i = 0; i < image.height(); ++i) vertical.append(image.pixelColor(x, i).name(QColor::HexArgb));
    return {{"horizontal", horizontal}, {"vertical", vertical}};
}

/// 【样式对照】【生产绘制】按argc和argv指定的新目录采集完整生产函数，返回进程状态
int main(int argc, char **argv) {
    QApplication app(argc, argv);
    if (argc != 2) return 2;
    const QString root = QString::fromLocal8Bit(argv[1]);
    QJsonArray samples;
    const std::array<std::pair<ShotWindow::Tool, const char *>, 6> tools = {{
        {ShotWindow::Tool::Pen, "pen"}, {ShotWindow::Tool::Line, "line"},
        {ShotWindow::Tool::Highlighter, "highlighter"}, {ShotWindow::Tool::Ellipse, "ellipse"},
        {ShotWindow::Tool::Arrow, "arrow"}, {ShotWindow::Tool::Number, "number"}
    }};
    int sequence = 0;
    for (const auto &tool : tools) for (qreal scale : {0.125, 0.5, 1.0, 2.0}) for (qreal width : {1.0, 6.0, 72.0}) for (qreal device : {1.0, 1.5}) {
        ShotWindow window;
        window.m_frozenFrame = QImage(640, 480, QImage::Format_ARGB32_Premultiplied);
        window.m_frozenImageRect = QRectF(320.0 - 320.0 * scale, 240.0 - 240.0 * scale, 640.0 * scale, 480.0 * scale);
        QImage image(qRound(640 * device), qRound(480 * device), QImage::Format_ARGB32_Premultiplied);
        image.setDevicePixelRatio(device);
        image.fill(Qt::white);
        ShotWindow::Annotation annotation;
        annotation.tool = tool.first;
        annotation.width = width;
        annotation.color = QColor(239, 68, 68, 180);
        annotation.highlighterStyle = ShotWindow::HighlighterStyle::Freehand;
        annotation.number = 1;
        annotation.points = {QPointF(320.0 - 80.0 / scale, 240.0), QPointF(320.0 + 80.0 / scale, 240.0)};
        annotation.rect = QRectF(320.0 - 60.0 / scale, 240.0 - 40.0 / scale, 120.0 / scale, 80.0 / scale);
        if (tool.first == ShotWindow::Tool::Number) annotation.points = {QPointF(320, 240), QPointF(320, 240)};
        {
            QPainter painter(&image);
            painter.setRenderHint(QPainter::Antialiasing, true);
            if (tool.first == ShotWindow::Tool::Highlighter) painter.fillRect(QRectF(300, 180, 40, 120), Qt::black);
            window.drawAnnotation(painter, annotation, true);
        }
        const QString name = QString("sample-%1.png").arg(sequence++, 3, 10, QLatin1Char('0'));
        const QString path = QDir(root).filePath(name);
        if (!image.save(path)) return 3;
        QJsonObject sample = pixels(image, device, QPointF(320, 240));
        sample.insert("tool", QString::fromLatin1(tool.second));
        sample.insert("scale", scale);
        sample.insert("width", width);
        sample.insert("device", device);
        sample.insert("image", name);
        sample.insert("sha256", digest(path));
        samples.append(sample);
    }
    QFile output(QDir(root).filePath("reference.json"));
    if (!output.open(QIODevice::WriteOnly)) return 4;
    output.write(QJsonDocument(QJsonObject{{"qt", qVersion()}, {"dpi", 96}, {"font", markshot::theme::uiFontFamily()}, {"samples", samples}}).toJson());
    return 0;
}
