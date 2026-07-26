# 内置扫码后端：优先 CMake config 包（Windows/vcpkg），回退 pkg-config（Linux）
#
# zxing-cpp 的接口跨主版本有断裂：读码参数类在 2.0 从 DecodeHints 改名为
# ReaderOptions，写码在 3.0 起改用 CreateBarcode/WriteBarcode。发行版仓库里的
# 版本参差不齐（Debian 12 是 1.4，Ubuntu 26.04 是 2.3），这里统一探测版本，
# 把主版本号与 ZX_USE_UTF8 传给编译单元由兼容层消化差异，只有写码测试因为
# 没有对应的旧接口实现而按版本关闭。
set(MARK_SHOT_ZXING_READER_MIN_VERSION "1.0")
set(MARK_SHOT_ZXING_WRITER_MIN_VERSION "3.0")

find_package(ZXing CONFIG QUIET)

set(MARK_SHOT_ZXING_VERSION "")
if(ZXing_FOUND)
    set(MARK_SHOT_ZXING_VERSION "${ZXing_VERSION}")
else()
    find_package(PkgConfig QUIET)
    if(PkgConfig_FOUND)
        pkg_check_modules(ZXingCpp QUIET IMPORTED_TARGET zxing)
        if(ZXingCpp_FOUND)
            set(MARK_SHOT_ZXING_VERSION "${ZXingCpp_VERSION}")
        endif()
    endif()
endif()

set(MARK_SHOT_ZXING_READER_SUPPORTED FALSE)
set(MARK_SHOT_ZXING_WRITER_SUPPORTED FALSE)
set(MARK_SHOT_ZXING_VERSION_MAJOR 0)
if(MARK_SHOT_ZXING_VERSION)
    string(REGEX MATCH "^[0-9]+" MARK_SHOT_ZXING_VERSION_MAJOR "${MARK_SHOT_ZXING_VERSION}")
    if(MARK_SHOT_ZXING_VERSION VERSION_GREATER_EQUAL MARK_SHOT_ZXING_READER_MIN_VERSION)
        set(MARK_SHOT_ZXING_READER_SUPPORTED TRUE)
        # ZX_USE_UTF8 让 1.x 的 text() 也返回 std::string，与 2.x/3.x 取值一致
        set(MARK_SHOT_ZXING_COMPILE_DEFINITIONS
            ZX_USE_UTF8
            MARK_SHOT_ZXING_VERSION_MAJOR=${MARK_SHOT_ZXING_VERSION_MAJOR})
    else()
        message(STATUS
            "mark-shot: zxing-cpp ${MARK_SHOT_ZXING_VERSION} is older than "
            "${MARK_SHOT_ZXING_READER_MIN_VERSION}; built-in code scan disabled")
    endif()
    if(MARK_SHOT_ZXING_VERSION VERSION_GREATER_EQUAL MARK_SHOT_ZXING_WRITER_MIN_VERSION)
        set(MARK_SHOT_ZXING_WRITER_SUPPORTED TRUE)
    else()
        message(STATUS
            "mark-shot: zxing-cpp ${MARK_SHOT_ZXING_VERSION} has no CreateBarcode API; "
            "code-scan plugin test disabled")
    endif()
endif()

# 版本过旧时不再对外暴露查找结果，内置后端与插件会一并跳过
if(NOT MARK_SHOT_ZXING_READER_SUPPORTED)
    set(ZXing_FOUND FALSE)
    set(ZXingCpp_FOUND FALSE)
endif()
