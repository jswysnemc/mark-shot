if(CMAKE_SYSTEM_NAME STREQUAL "Linux")
    find_package(Python3 QUIET COMPONENTS Interpreter)
    if(Python3_Interpreter_FOUND)
        # 【剪贴板测试】【真实协议】不使用 offscreen 替代 Wayland，缺少独立桌面工具时显式跳过
        qt_add_executable(mark-shot-clipboard-reader-test
            tests/clipboard_reader_test.cpp
            src/clipboard_paste/clipboard_reader.cpp)
        target_include_directories(mark-shot-clipboard-reader-test PRIVATE src)
        target_link_libraries(mark-shot-clipboard-reader-test PRIVATE Qt6::Widgets Qt6::Test)
        add_test(NAME clipboard-reader-wayland
            COMMAND ${Python3_EXECUTABLE} ${CMAKE_CURRENT_SOURCE_DIR}/tests/clipboard_wayland_session_test.py
                $<TARGET_FILE:mark-shot-clipboard-reader-test>)
        set_tests_properties(clipboard-reader-wayland PROPERTIES
            TIMEOUT 90 SKIP_RETURN_CODE 77 LABELS "wayland;integration")
    endif()
endif()
