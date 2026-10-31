# 1. 【窗口测试】【OCR数据】独立控制台入口负责输出，避免窗口程序的标准输出差异
qt_add_executable(mark-shot-ocr-fixture tests/fixtures/ocr_fixture.cpp)
set_target_properties(mark-shot-ocr-fixture PROPERTIES WIN32_EXECUTABLE FALSE)
target_link_libraries(mark-shot-ocr-fixture PRIVATE Qt6::Core)

qt_add_executable(mark-shot-shell-command-test tests/shell_command_test.cpp src/shell_command.cpp)
target_include_directories(mark-shot-shell-command-test PRIVATE src)
target_link_libraries(mark-shot-shell-command-test PRIVATE Qt6::Core Qt6::Test)
target_compile_definitions(mark-shot-shell-command-test PRIVATE
    MARK_SHOT_TEST_OCR_FIXTURE_PATH="$<TARGET_FILE:mark-shot-ocr-fixture>")
add_dependencies(mark-shot-shell-command-test mark-shot-ocr-fixture)
add_test(NAME shell-command COMMAND mark-shot-shell-command-test -o -,txt)

# 2. 【窗口测试】【真实入口】复用应用对象，按功能分别检查实际窗口行为
foreach(window_test IN ITEMS pinned-text-selection capture-history-window settings-idle-repaint settings-card-pages arrow-properties selection-hover-repaint selection-history-restore text-annotation-editing ruler-annotation ruler-rotation toolbar-dock-window toolbar-theme capture-memory canvas-image-memory)
    string(REPLACE "-" "_" source_name "${window_test}")
    qt_add_executable(mark-shot-${window_test}-test
        tests/${source_name}_test.cpp
        "$<FILTER:$<TARGET_OBJECTS:mark-shot>,EXCLUDE,/main\\.cpp\\.(o|obj)$>"
    )
    target_include_directories(mark-shot-${window_test}-test PRIVATE "$<TARGET_PROPERTY:mark-shot,INCLUDE_DIRECTORIES>")
    target_compile_definitions(mark-shot-${window_test}-test PRIVATE "$<TARGET_PROPERTY:mark-shot,COMPILE_DEFINITIONS>")
    target_link_libraries(mark-shot-${window_test}-test PRIVATE "$<TARGET_PROPERTY:mark-shot,LINK_LIBRARIES>" Qt6::Test)
    get_target_property(window_test_standard mark-shot CXX_STANDARD)
    set_target_properties(mark-shot-${window_test}-test PROPERTIES CXX_STANDARD ${window_test_standard})
    if(WIN32)
        set_target_properties(mark-shot-${window_test}-test PROPERTIES WIN32_EXECUTABLE TRUE)
    endif()
    add_dependencies(mark-shot-${window_test}-test mark-shot)
    add_test(NAME ${window_test} COMMAND mark-shot-${window_test}-test -o -,txt)
    set_tests_properties(${window_test} PROPERTIES TIMEOUT 30
        ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_STYLE_OVERRIDE=Fusion;QT_SCALE_FACTOR=1")
endforeach()

add_test(NAME settings-idle-repaint-hidpi COMMAND mark-shot-settings-idle-repaint-test -o -,txt)
set_tests_properties(settings-idle-repaint-hidpi PROPERTIES TIMEOUT 30
    ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_STYLE_OVERRIDE=Fusion;QT_SCALE_FACTOR=2")

add_test(NAME canvas-image-memory-hidpi COMMAND mark-shot-canvas-image-memory-test -o -,txt)
set_tests_properties(canvas-image-memory-hidpi PROPERTIES TIMEOUT 30
    ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_STYLE_OVERRIDE=Fusion;QT_SCALE_FACTOR=2")

target_compile_definitions(mark-shot-pinned-text-selection-test PRIVATE
    MARK_SHOT_TEST_OCR_FIXTURE_PATH="$<TARGET_FILE:mark-shot-ocr-fixture>")
add_dependencies(mark-shot-pinned-text-selection-test mark-shot-ocr-fixture)
