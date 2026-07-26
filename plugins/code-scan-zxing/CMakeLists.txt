# zxing-cpp 扫码插件：zxing-cpp 缺失或版本过旧时整体跳过

if(DEFINED MARK_SHOT_ZXING_READER_SUPPORTED AND NOT MARK_SHOT_ZXING_READER_SUPPORTED)
    message(STATUS "mark-shot: code-scan-zxing plugin disabled (zxing-cpp is too old)")
    return()
endif()

if(NOT TARGET ZXing::ZXing AND NOT TARGET PkgConfig::ZXingCpp)
    find_package(ZXing CONFIG QUIET)
endif()
if(NOT TARGET ZXing::ZXing AND NOT TARGET PkgConfig::ZXingCpp)
    find_package(PkgConfig QUIET)
    if(PkgConfig_FOUND)
        pkg_check_modules(ZXingPlugin QUIET IMPORTED_TARGET zxing)
    endif()
endif()

if(NOT TARGET ZXing::ZXing AND NOT TARGET PkgConfig::ZXingCpp AND NOT TARGET PkgConfig::ZXingPlugin)
    message(STATUS "mark-shot: code-scan-zxing plugin disabled (zxing-cpp not found)")
    return()
endif()

qt_add_plugin(mark-shot-code-scan-zxing
    SHARED
    zxing_code_scan_plugin.cpp
    zxing_code_scan_plugin.h
)

target_include_directories(mark-shot-code-scan-zxing PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}
    ${CMAKE_SOURCE_DIR}/plugin-sdk
)
target_compile_definitions(mark-shot-code-scan-zxing PRIVATE ${MARK_SHOT_ZXING_COMPILE_DEFINITIONS})
target_link_libraries(mark-shot-code-scan-zxing
    PRIVATE
        Qt6::Core
        Qt6::Gui
)
if(TARGET ZXing::ZXing)
    target_link_libraries(mark-shot-code-scan-zxing PRIVATE ZXing::ZXing)
elseif(TARGET PkgConfig::ZXingCpp)
    target_link_libraries(mark-shot-code-scan-zxing PRIVATE PkgConfig::ZXingCpp)
else()
    target_link_libraries(mark-shot-code-scan-zxing PRIVATE PkgConfig::ZXingPlugin)
endif()

install(TARGETS mark-shot-code-scan-zxing
        LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}/mark-shot/plugins
        RUNTIME DESTINATION ${CMAKE_INSTALL_LIBDIR}/mark-shot/plugins)

message(STATUS "mark-shot: code-scan-zxing plugin enabled")
