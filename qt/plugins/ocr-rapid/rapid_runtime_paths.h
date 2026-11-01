#pragma once

#include <QStringList>

namespace markshot::ocr_rapid {

/** 【OCR】【运行库定位】返回显式指定或按私有目录、加载器与系统目录排序的候选路径，无参数。 */
QStringList rapidOnnxRuntimeCandidates();

}  // namespace markshot::ocr_rapid
