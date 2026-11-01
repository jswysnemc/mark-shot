# 1. 【OCR】【发布依赖】读取插件直接依赖，避免系统升级后因无用的版本绑定而加载失败
execute_process(
    COMMAND "${READELF}" -d "${PLUGIN_PATH}"
    RESULT_VARIABLE result
    OUTPUT_VARIABLE dynamic_section
    ERROR_VARIABLE error
)
if(NOT result EQUAL 0)
    message(FATAL_ERROR "Cannot inspect OCR plugin dependencies: ${error}")
endif()

# 2. 【OCR】【发布依赖】Protobuf、Abseil 和 UTF-8 支持库应由 ONNX Runtime 管理
if(dynamic_section MATCHES "\\(NEEDED\\)[^\n]*\\[lib(protobuf|absl_|utf8_)")
    message(FATAL_ERROR
        "OCR plugin directly links an ONNX Runtime implementation dependency: ${CMAKE_MATCH_0}")
endif()

# 3. 【OCR】【运行库寿命】插件常驻不能继续通过直接链接固定推理运行库
if(dynamic_section MATCHES "\\(NEEDED\\)[^\n]*\\[libonnxruntime")
    message(FATAL_ERROR "OCR plugin retains ONNX Runtime through a direct dependency")
endif()
