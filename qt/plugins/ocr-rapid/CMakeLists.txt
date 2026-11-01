# ocr-rapid 插件：PP-OCR ONNX 推理 provider，onnxruntime 缺失时整体跳过

find_package(PkgConfig QUIET)
if(PkgConfig_FOUND)
    pkg_check_modules(OnnxRuntime QUIET IMPORTED_TARGET libonnxruntime)
endif()
if(NOT OnnxRuntime_FOUND)
    find_package(onnxruntime CONFIG QUIET)
endif()

if(NOT OnnxRuntime_FOUND AND NOT onnxruntime_FOUND)
    message(STATUS "mark-shot: ocr-rapid plugin disabled (onnxruntime not found)")
    return()
endif()

# qt_add_plugin 在 Qt 6.2 会忽略位置参数形式的源文件列表，链接出没有插件符号的空库，
# 源文件必须用 target_sources 显式添加
qt_add_plugin(mark-shot-ocr-rapid SHARED)

# 【OCR】【编译边界】只复用构建所选版本的公开头文件，运行库由每次推理的句柄持有
add_library(mark-shot-onnx-headers INTERFACE)
if(OnnxRuntime_FOUND)
    set(mark_shot_onnx_target PkgConfig::OnnxRuntime)
else()
    set(mark_shot_onnx_target onnxruntime::onnxruntime)
endif()
target_include_directories(mark-shot-onnx-headers SYSTEM INTERFACE
    "$<TARGET_PROPERTY:${mark_shot_onnx_target},INTERFACE_INCLUDE_DIRECTORIES>")

target_sources(mark-shot-ocr-rapid PRIVATE
    rapid_det_model.cpp
    rapid_det_model.h
    rapid_image_input.cpp
    rapid_image_input.h
    rapid_model_paths.cpp
    rapid_model_paths.h
    rapid_runtime_paths.cpp
    rapid_runtime_paths.h
    rapid_onnx_runtime.cpp
    rapid_onnx_runtime.h
    rapid_onnx_handle.h
    rapid_ocr_engine.cpp
    rapid_ocr_engine.h
    rapid_ocr_plugin.cpp
    rapid_ocr_plugin.h
    rapid_ocr_word_segments.cpp
    rapid_ocr_word_segments.h
    rapid_onnx_session.cpp
    rapid_onnx_session.h
    rapid_rec_model.cpp
    rapid_rec_model.h
)

target_include_directories(mark-shot-ocr-rapid PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}
    ${MARK_SHOT_QT_SOURCE_DIR}/plugin-sdk
)
target_link_libraries(mark-shot-ocr-rapid
    PRIVATE
        Qt6::Core
        Qt6::Gui
        mark-shot-onnx-headers
        ${CMAKE_DL_LIBS}
)

install(TARGETS mark-shot-ocr-rapid
        LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}/mark-shot/plugins
        RUNTIME DESTINATION ${CMAKE_INSTALL_LIBDIR}/mark-shot/plugins)

message(STATUS "mark-shot: ocr-rapid plugin enabled")
