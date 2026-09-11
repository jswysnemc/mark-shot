qt_add_executable(mark-shot-ocr-result-window-geometry-test
    tests/ocr_result_window_geometry_test.cpp
    src/ocr_result_window_geometry.cpp
    src/ocr_result_window_geometry.h
)
target_include_directories(mark-shot-ocr-result-window-geometry-test PRIVATE src)
target_link_libraries(mark-shot-ocr-result-window-geometry-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME ocr-result-window-geometry COMMAND mark-shot-ocr-result-window-geometry-test)

qt_add_executable(mark-shot-selection-adjustment-test
    tests/selection_adjustment_test.cpp
    src/selection_adjustment.cpp
    src/selection_cursor_nudge.cpp
)
target_include_directories(mark-shot-selection-adjustment-test PRIVATE src)
target_link_libraries(mark-shot-selection-adjustment-test PRIVATE Qt6::Core Qt6::Gui Qt6::Test)
add_test(NAME selection-adjustment COMMAND mark-shot-selection-adjustment-test)

qt_add_executable(mark-shot-ocr-result-window-config-test
    tests/ocr_result_window_config_test.cpp
    src/ocr_result_window_config.cpp
)
target_include_directories(mark-shot-ocr-result-window-config-test PRIVATE src)
target_link_libraries(mark-shot-ocr-result-window-config-test PRIVATE Qt6::Core Qt6::Test)
add_test(NAME ocr-result-window-config COMMAND mark-shot-ocr-result-window-config-test)
