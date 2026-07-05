#include "zxing_code_scan_plugin.h"

#include <QImage>

#include <ZXing/Barcode.h>
#include <ZXing/BarcodeFormat.h>
#include <ZXing/ImageView.h>
#include <ZXing/ReadBarcode.h>

namespace markshot::code_scan_zxing {

QString ZxingCodeScanPlugin::providerId() const
{
    return QStringLiteral("zxing-cpp");
}

QString ZxingCodeScanPlugin::displayName() const
{
    return QStringLiteral("ZXing-C++");
}

bool ZxingCodeScanPlugin::isAvailable(QString *error) const
{
    Q_UNUSED(error)
    return true;
}

bool ZxingCodeScanPlugin::scan(const QImage &image,
                               QVector<markshot::plugin::CodeScanResult> *results,
                               QString *error)
{
    if (!results) {
        if (error) {
            *error = QStringLiteral("code scan output target is missing");
        }
        return false;
    }
    results->clear();

    // 1. 灰度化后交给 zxing，Lum 格式在各版本间行为最稳定
    const QImage gray = image.convertToFormat(QImage::Format_Grayscale8);
    if (gray.isNull()) {
        if (error) {
            *error = QStringLiteral("cannot load image for code scan");
        }
        return false;
    }

    ZXing::ImageView view(gray.constBits(),
                          gray.width(),
                          gray.height(),
                          ZXing::ImageFormat::Lum,
                          static_cast<int>(gray.bytesPerLine()));
    ZXing::ReaderOptions options;
    options.setTryHarder(true);
    options.setTryRotate(true);
    const auto barcodes = ZXing::ReadBarcodes(view, options);

    for (const auto &barcode : barcodes) {
        markshot::plugin::CodeScanResult result;
        result.format = QString::fromStdString(ZXing::ToString(barcode.format()));
        result.text = QString::fromStdString(barcode.text());
        if (result.text.trimmed().isEmpty()) {
            continue;
        }
        const auto &position = barcode.position();
        for (const auto &point : {position.topLeft(), position.topRight(),
                                  position.bottomRight(), position.bottomLeft()}) {
            result.points.append(QPointF(point.x, point.y));
        }
        results->append(result);
    }
    return true;
}

}  // namespace markshot::code_scan_zxing
