# Google Gemini 翻译插件

qt_add_plugin(mark-shot-translate-gemini SHARED)

target_sources(mark-shot-translate-gemini PRIVATE
    gemini_translate_config.cpp
    gemini_translate_config.h
    gemini_translate_plugin.cpp
    gemini_translate_plugin.h
    gemini_translate_request.cpp
    gemini_translate_request.h
)

target_include_directories(mark-shot-translate-gemini PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}
    ${CMAKE_SOURCE_DIR}/plugin-sdk
)
target_link_libraries(mark-shot-translate-gemini
    PRIVATE
        mark-shot-translate-common
        Qt6::Core
        Qt6::Network
)

install(TARGETS mark-shot-translate-gemini
        LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}/mark-shot/plugins
        RUNTIME DESTINATION ${CMAKE_INSTALL_LIBDIR}/mark-shot/plugins)

message(STATUS "mark-shot: translate-gemini plugin enabled")
