#include "ui/i18n_annotation_tools.h"

namespace markshot::i18n {

const QHash<QString, QString> &annotationToolChineseTable()
{
    static const QHash<QString, QString> table = {
        {QStringLiteral("Length"), QStringLiteral("长度")},
        {QStringLiteral("Angle"), QStringLiteral("角度")},
        {QStringLiteral("Measure mode"), QStringLiteral("测量方式")},
        {QStringLiteral("Measure length"), QStringLiteral("测量长度")},
        {QStringLiteral("Measure angle"), QStringLiteral("测量角度")},
    };
    return table;
}

}
