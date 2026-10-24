qt_add_executable(mark-shot-region-detection-config-test
    tests/region_detection_config_test.cpp
    src/region_detection/region_detection_config.cpp
    src/config_value.cpp)
target_include_directories(mark-shot-region-detection-config-test PRIVATE src)
target_link_libraries(mark-shot-region-detection-config-test PRIVATE Qt6::Core Qt6::Gui Qt6::Test)
add_test(NAME region-detection-config COMMAND mark-shot-region-detection-config-test)

# 【截图测试】【区域识别】复用应用对象验证设置持久化及实际窗口识别回退
qt_add_executable(mark-shot-region-detection-toggle-test
    tests/region_detection_toggle_test.cpp
    "$<FILTER:$<TARGET_OBJECTS:mark-shot>,EXCLUDE,/main\\.cpp\\.(o|obj)$>")
target_include_directories(mark-shot-region-detection-toggle-test PRIVATE
    "$<TARGET_PROPERTY:mark-shot,INCLUDE_DIRECTORIES>")
target_compile_definitions(mark-shot-region-detection-toggle-test PRIVATE
    "$<TARGET_PROPERTY:mark-shot,COMPILE_DEFINITIONS>")
target_link_libraries(mark-shot-region-detection-toggle-test PRIVATE
    "$<TARGET_PROPERTY:mark-shot,LINK_LIBRARIES>" Qt6::Test)
get_target_property(region_toggle_standard mark-shot CXX_STANDARD)
set_target_properties(mark-shot-region-detection-toggle-test PROPERTIES CXX_STANDARD ${region_toggle_standard})
if(WIN32)
    set_target_properties(mark-shot-region-detection-toggle-test PROPERTIES WIN32_EXECUTABLE TRUE)
endif()
add_dependencies(mark-shot-region-detection-toggle-test mark-shot)
add_test(NAME region-detection-toggle COMMAND mark-shot-region-detection-toggle-test -o -,txt)
set_tests_properties(region-detection-toggle PROPERTIES TIMEOUT 30
    ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_STYLE_OVERRIDE=Fusion;QT_SCALE_FACTOR=1")
