#pragma once

// zxing-cpp 跨主版本的接口差异集中在这里。
//
// 读码参数类在 1.x 叫 DecodeHints，2.0 起改名为 ReaderOptions；
// 条码结果类在 3.0 起从 Result 改名为 Barcode，但两版都能用 auto 推导。
// 文本取值在定义 ZX_USE_UTF8 后各版本统一返回 std::string，该宏由构建系统
// 全局定义，保证在包含 ZXing 头之前生效。

#include <ZXing/BarcodeFormat.h>
#include <ZXing/ImageView.h>
#include <ZXing/ReadBarcode.h>

namespace markshot::zxing {

#if defined(MARK_SHOT_ZXING_VERSION_MAJOR) && MARK_SHOT_ZXING_VERSION_MAJOR >= 2
using ReaderOptions = ZXing::ReaderOptions;
#else
using ReaderOptions = ZXing::DecodeHints;
#endif

/**
 * 【扫码】【版本兼容】创建屏幕扫码使用的读码参数。
 * @return 打开加强识别与旋转识别的读码参数。
 */
inline ReaderOptions screenReaderOptions()
{
    ReaderOptions options;
    options.setTryHarder(true);
    options.setTryRotate(true);
    return options;
}

}  // namespace markshot::zxing
