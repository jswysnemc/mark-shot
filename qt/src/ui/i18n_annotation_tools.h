#pragma once

#include <QHash>
#include <QString>

namespace markshot::i18n {

/// @brief 提供标注工具及其属性栏的中文文案
/// @return 以英文源文本为键的只读中文词典
const QHash<QString, QString> &annotationToolChineseTable();

}
