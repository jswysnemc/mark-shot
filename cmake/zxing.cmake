# 内置扫码后端：优先 CMake config 包（Windows/vcpkg），回退 pkg-config（Linux）
find_package(ZXing CONFIG QUIET)
if(NOT ZXing_FOUND)
    find_package(PkgConfig QUIET)
    if(PkgConfig_FOUND)
        pkg_check_modules(ZXingCpp QUIET IMPORTED_TARGET zxing)
    endif()
endif()
