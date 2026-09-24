# Anthropic Claude 翻译插件

qt_add_plugin(mark-shot-translate-anthropic SHARED)

target_sources(mark-shot-translate-anthropic PRIVATE
    anthropic_translate_config.cpp
    anthropic_translate_config.h
    anthropic_translate_plugin.cpp
    anthropic_translate_plugin.h
    anthropic_translate_request.cpp
    anthropic_translate_request.h
)

target_include_directories(mark-shot-translate-anthropic PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}
    ${CMAKE_SOURCE_DIR}/plugin-sdk
)
target_link_libraries(mark-shot-translate-anthropic
    PRIVATE
        mark-shot-translate-common
        Qt6::Core
        Qt6::Network
)

install(TARGETS mark-shot-translate-anthropic
        LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}/mark-shot/plugins
        RUNTIME DESTINATION ${CMAKE_INSTALL_LIBDIR}/mark-shot/plugins)

message(STATUS "mark-shot: translate-anthropic plugin enabled")
