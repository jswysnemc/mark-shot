# 【截图测试】【议题回归】复用实际应用对象和隔离运行环境
set(issue_regression_tests startup-tool-screen-routing window-detection-bundle)
if(MARK_SHOT_LINUX)
    find_program(MARK_SHOT_DBUS_RUN_SESSION dbus-run-session)
    if(MARK_SHOT_DBUS_RUN_SESSION)
        list(APPEND issue_regression_tests gnome-cursor-screen)
    endif()
endif()

get_target_property(issue_regression_standard mark-shot CXX_STANDARD)
foreach(issue_test IN LISTS issue_regression_tests)
    string(REPLACE "-" "_" issue_source ${issue_test})
    qt_add_executable(mark-shot-${issue_test}-test
        tests/${issue_source}_test.cpp
        "$<FILTER:$<TARGET_OBJECTS:mark-shot>,EXCLUDE,/main\\.cpp\\.(o|obj)$>")
    target_include_directories(mark-shot-${issue_test}-test PRIVATE
        "$<TARGET_PROPERTY:mark-shot,INCLUDE_DIRECTORIES>")
    target_compile_definitions(mark-shot-${issue_test}-test PRIVATE
        "$<TARGET_PROPERTY:mark-shot,COMPILE_DEFINITIONS>")
    target_link_libraries(mark-shot-${issue_test}-test PRIVATE
        "$<TARGET_PROPERTY:mark-shot,LINK_LIBRARIES>" Qt6::Test)
    set_target_properties(mark-shot-${issue_test}-test PROPERTIES CXX_STANDARD ${issue_regression_standard})
    if(WIN32)
        set_target_properties(mark-shot-${issue_test}-test PROPERTIES WIN32_EXECUTABLE TRUE)
    endif()
    add_dependencies(mark-shot-${issue_test}-test mark-shot)
    if(issue_test STREQUAL "gnome-cursor-screen")
        add_test(NAME ${issue_test} COMMAND ${MARK_SHOT_DBUS_RUN_SESSION}
            -- $<TARGET_FILE:mark-shot-${issue_test}-test> -o -,txt)
    else()
        add_test(NAME ${issue_test} COMMAND mark-shot-${issue_test}-test -o -,txt)
    endif()
    set_tests_properties(${issue_test} PROPERTIES TIMEOUT 30)
endforeach()
