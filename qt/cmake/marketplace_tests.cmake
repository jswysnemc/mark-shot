qt_add_executable(mark-shot-plugin-index-parser-test
    tests/plugin_index_parser_test.cpp
    src/marketplace/plugin_index_parser.cpp
    src/marketplace/plugin_index_parser.h
    src/marketplace/plugin_qt_compat.cpp
    src/marketplace/plugin_qt_compat.h
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
    src/marketplace/plugin_updates.cpp
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
add_test(NAME plugin-installer COMMAND mark-shot-plugin-installer-test -o -,txt)

foreach(revision IN ITEMS 1 2)
    add_library(mark-shot-update-fixture-${revision} SHARED tests/fixtures/update_library.cpp)
    target_link_libraries(mark-shot-update-fixture-${revision} PRIVATE Qt6::Core Qt6::Gui)
    target_include_directories(mark-shot-update-fixture-${revision} PRIVATE plugin-sdk)
    target_compile_definitions(mark-shot-update-fixture-${revision} PRIVATE MARK_SHOT_TEST_PLUGIN_REVISION=${revision})
    qt_add_plugin(mark-shot-registry-fixture-${revision} SHARED)
    target_sources(mark-shot-registry-fixture-${revision} PRIVATE tests/fixtures/registry_update_plugin.cpp)
    target_link_libraries(mark-shot-registry-fixture-${revision} PRIVATE Qt6::Core Qt6::Gui)
    target_include_directories(mark-shot-registry-fixture-${revision} PRIVATE plugin-sdk)
    target_compile_definitions(mark-shot-registry-fixture-${revision} PRIVATE MARK_SHOT_TEST_PLUGIN_REVISION=${revision})
endforeach()
target_compile_definitions(mark-shot-plugin-installer-test PRIVATE
    MARK_SHOT_TEST_OLD_PLUGIN="$<TARGET_FILE:mark-shot-update-fixture-1>"
    MARK_SHOT_TEST_NEW_PLUGIN="$<TARGET_FILE:mark-shot-update-fixture-2>")
add_dependencies(mark-shot-plugin-installer-test mark-shot-update-fixture-1 mark-shot-update-fixture-2)

qt_add_executable(mark-shot-plugin-registry-update-test
    tests/plugin_registry_update_test.cpp
    src/providers/provider_plugin_registry.cpp
    src/providers/provider_runtime_dependencies.cpp
    src/providers/provider_plugin_paths.cpp
    src/marketplace/plugin_installer.cpp
    src/marketplace/plugin_updates.cpp
    src/debug_log.cpp
)
target_include_directories(mark-shot-plugin-registry-update-test PRIVATE src plugin-sdk)
target_link_libraries(mark-shot-plugin-registry-update-test PRIVATE Qt6::Core Qt6::Gui Qt6::Test)
target_compile_definitions(mark-shot-plugin-registry-update-test PRIVATE
    MARK_SHOT_TEST_OLD_PLUGIN="$<TARGET_FILE:mark-shot-registry-fixture-1>"
    MARK_SHOT_TEST_NEW_PLUGIN="$<TARGET_FILE:mark-shot-registry-fixture-2>")
add_dependencies(mark-shot-plugin-registry-update-test mark-shot-registry-fixture-1 mark-shot-registry-fixture-2)
add_test(NAME plugin-registry-update COMMAND mark-shot-plugin-registry-update-test -o -,txt)

# 【插件测试】【按需加载】测试程序与夹具插件放在独立目录，插件目录中只有该夹具
set(MARK_SHOT_CAPABILITY_TEST_DIR "${CMAKE_CURRENT_BINARY_DIR}/capability-registry-test")
qt_add_plugin(mark-shot-capability-fixture SHARED)
target_sources(mark-shot-capability-fixture PRIVATE tests/fixtures/capability_translate_plugin.cpp)
target_link_libraries(mark-shot-capability-fixture PRIVATE Qt6::Core)
target_include_directories(mark-shot-capability-fixture PRIVATE plugin-sdk)
set_target_properties(mark-shot-capability-fixture PROPERTIES
    LIBRARY_OUTPUT_DIRECTORY "${MARK_SHOT_CAPABILITY_TEST_DIR}/plugins"
    RUNTIME_OUTPUT_DIRECTORY "${MARK_SHOT_CAPABILITY_TEST_DIR}/plugins")

qt_add_executable(mark-shot-plugin-registry-capability-test
    tests/plugin_registry_capability_test.cpp
    src/providers/provider_plugin_registry.cpp
    src/providers/provider_runtime_dependencies.cpp
    src/providers/provider_plugin_paths.cpp
    src/marketplace/plugin_installer.cpp
    src/marketplace/plugin_updates.cpp
    src/debug_log.cpp
)
target_include_directories(mark-shot-plugin-registry-capability-test PRIVATE src plugin-sdk)
target_link_libraries(mark-shot-plugin-registry-capability-test PRIVATE Qt6::Core Qt6::Gui Qt6::Test)
set_target_properties(mark-shot-plugin-registry-capability-test PROPERTIES
    RUNTIME_OUTPUT_DIRECTORY "${MARK_SHOT_CAPABILITY_TEST_DIR}")
add_dependencies(mark-shot-plugin-registry-capability-test mark-shot-capability-fixture)
add_test(NAME plugin-registry-capability COMMAND mark-shot-plugin-registry-capability-test -o -,txt)
