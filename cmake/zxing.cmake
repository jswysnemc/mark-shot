# 内置扫码后端：优先 CMake config 包（Windows/vcpkg），回退 pkg-config（Linux）
#
# zxing-cpp 的接口跨主版本有断裂：读码在 2.0 起改用 ReaderOptions/ReadBarcodes，
# 写码在 3.0 起改用 CreateBarcode/WriteBarcode。发行版仓库里的版本参差不齐
# （例如 Debian 12 仍是 1.x），这里统一探测版本并按能力开关对应目标，
# 让旧环境退回到不带内置扫码的构建，而不是让整个包构建失败。
set(MARK_SHOT_ZXING_READER_MIN_VERSION "2.0")
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
if(MARK_SHOT_ZXING_VERSION)
    if(MARK_SHOT_ZXING_VERSION VERSION_GREATER_EQUAL MARK_SHOT_ZXING_READER_MIN_VERSION)
        set(MARK_SHOT_ZXING_READER_SUPPORTED TRUE)
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
