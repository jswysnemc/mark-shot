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
