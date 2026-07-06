qt_add_executable(mark-shot-plugin-index-parser-test
    tests/plugin_index_parser_test.cpp
    src/marketplace/plugin_index_parser.cpp
    src/marketplace/plugin_index_parser.h
)
target_include_directories(mark-shot-plugin-index-parser-test PRIVATE src)
target_compile_definitions(mark-shot-plugin-index-parser-test
    PRIVATE
        MARK_SHOT_TEST_SOURCE_DIR="${CMAKE_CURRENT_SOURCE_DIR}"
)
target_link_libraries(mark-shot-plugin-index-parser-test
    PRIVATE
        Qt6::Core
        Qt6::Test
)
add_test(NAME plugin-index-parser COMMAND mark-shot-plugin-index-parser-test)

qt_add_executable(mark-shot-plugin-installer-test
    tests/plugin_installer_test.cpp
    src/marketplace/plugin_installer.cpp
    src/marketplace/plugin_installer.h
    src/providers/provider_plugin_paths.cpp
    src/providers/provider_plugin_paths.h
)
target_include_directories(mark-shot-plugin-installer-test PRIVATE src)
target_link_libraries(mark-shot-plugin-installer-test
    PRIVATE
        Qt6::Core
        Qt6::Test
)
add_test(NAME plugin-installer COMMAND mark-shot-plugin-installer-test)
