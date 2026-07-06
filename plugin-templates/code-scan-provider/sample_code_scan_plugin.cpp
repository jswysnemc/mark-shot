#include "sample_code_scan_plugin.h"

QString SampleCodeScanPlugin::providerId() const
{
    return QStringLiteral("sample-code-scan");
}

QString SampleCodeScanPlugin::displayName() const
{
    return QStringLiteral("Sample Code Scan");
}

bool SampleCodeScanPlugin::isAvailable(QString *error) const
{
    Q_UNUSED(error)
    return true;
}

bool SampleCodeScanPlugin::scan(const QImage &image,
                                QVector<markshot::plugin::CodeScanResult> *results,
                                QString *error)
{
    if (!results) {
        if (error) {
            *error = QStringLiteral("Output result container is null");
        }
        return false;
    }
    results->clear();
    if (image.isNull()) {
        if (error) {
            *error = QStringLiteral("Input image is empty");
        }
        return false;
    }

    // 1. 在这里调用真实扫码库，并把结果转换为 CodeScanResult
    return true;
}
