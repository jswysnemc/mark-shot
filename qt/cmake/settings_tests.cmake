qt_add_executable(mark-shot-provider-preference-config-test
    tests/provider_preference_config_test.cpp
    src/settings/provider_preference_config.cpp
    src/settings/provider_preference_config.h
    src/config_value.cpp
    src/config_value.h
)
target_include_directories(mark-shot-provider-preference-config-test PRIVATE src)
target_link_libraries(mark-shot-provider-preference-config-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
)
add_test(NAME provider-preference-config COMMAND mark-shot-provider-preference-config-test)

qt_add_executable(mark-shot-settings-wheel-guard-test
    tests/settings_wheel_guard_test.cpp
    src/settings/settings_wheel_guard.cpp
    src/settings/settings_wheel_guard.h
)
target_include_directories(mark-shot-settings-wheel-guard-test PRIVATE src)
target_link_libraries(mark-shot-settings-wheel-guard-test
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Test
        Qt6::Widgets
)
add_test(NAME settings-wheel-guard COMMAND mark-shot-settings-wheel-guard-test)

qt_add_executable(mark-shot-settings-form-layout-test
    tests/settings_form_layout_test.cpp
    src/settings/settings_form_layout.cpp
    src/settings/settings_form_layout.h
)
target_include_directories(mark-shot-settings-form-layout-test PRIVATE src)
target_link_libraries(mark-shot-settings-form-layout-test PRIVATE Qt6::Core Qt6::Gui Qt6::Widgets Qt6::Test)
add_test(NAME settings-form-layout COMMAND mark-shot-settings-form-layout-test)

qt_add_executable(mark-shot-settings-card-test
    tests/settings_card_test.cpp
    src/settings/settings_form_layout.cpp
    src/settings/settings_form_layout.h
    src/ui/disclosure_section.cpp
    src/ui/disclosure_section.h
    src/ui/settings_card.cpp
    src/ui/settings_card.h
    src/ui/settings_form_row.cpp
    src/ui/settings_form_row.h
    src/ui/theme.cpp
)
target_include_directories(mark-shot-settings-card-test PRIVATE src)
target_link_libraries(mark-shot-settings-card-test PRIVATE Qt6::Core Qt6::Gui Qt6::Widgets Qt6::Test)
add_test(NAME settings-card COMMAND mark-shot-settings-card-test)

# 【设置测试】【命令路径】浏览命令程序时只替换程序片段的纯函数测试
qt_add_executable(mark-shot-settings-command-path-test
    tests/settings_command_path_test.cpp
    src/settings/settings_command_path.cpp
    src/settings/settings_command_path.h
)
target_include_directories(mark-shot-settings-command-path-test PRIVATE src)
target_link_libraries(mark-shot-settings-command-path-test PRIVATE Qt6::Core Qt6::Test)
add_test(NAME settings-command-path COMMAND mark-shot-settings-command-path-test)

# 【设置测试】【字段控件】复用主程序目标文件，检查快捷键胶囊、颜色胶囊、路径字段与底部按钮
qt_add_executable(mark-shot-settings-field-widgets-test
    tests/settings_field_widgets_test.cpp
    "$<FILTER:$<TARGET_OBJECTS:mark-shot>,EXCLUDE,/main\\.cpp\\.(o|obj)$>"
)
target_include_directories(mark-shot-settings-field-widgets-test PRIVATE "$<TARGET_PROPERTY:mark-shot,INCLUDE_DIRECTORIES>")
target_compile_definitions(mark-shot-settings-field-widgets-test PRIVATE "$<TARGET_PROPERTY:mark-shot,COMPILE_DEFINITIONS>")
target_link_libraries(mark-shot-settings-field-widgets-test PRIVATE "$<TARGET_PROPERTY:mark-shot,LINK_LIBRARIES>" Qt6::Test)
get_target_property(settings_field_test_standard mark-shot CXX_STANDARD)
set_target_properties(mark-shot-settings-field-widgets-test PROPERTIES CXX_STANDARD ${settings_field_test_standard})
if(WIN32)
    # 【测试】【Windows 入口】同步主程序类型，为继承的 QT_NEEDS_QMAIN 定义链接 Qt 入口库
    set_target_properties(mark-shot-settings-field-widgets-test PROPERTIES WIN32_EXECUTABLE TRUE)
endif()
add_dependencies(mark-shot-settings-field-widgets-test mark-shot)
add_test(NAME settings-field-widgets COMMAND mark-shot-settings-field-widgets-test -o -,txt)
set_tests_properties(settings-field-widgets PROPERTIES TIMEOUT 30
    ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_STYLE_OVERRIDE=Fusion;QT_SCALE_FACTOR=1")

# 【托盘测试】【全局快捷键】设置保存后只在快捷键变化时重新注册
qt_add_executable(mark-shot-windows-tray-hotkey-diff-test
    tests/windows_tray_hotkey_diff_test.cpp
    src/windows_tray_hotkey_diff.cpp
    src/windows_tray_hotkey_diff.h
)
target_include_directories(mark-shot-windows-tray-hotkey-diff-test PRIVATE src)
target_link_libraries(mark-shot-windows-tray-hotkey-diff-test PRIVATE Qt6::Core Qt6::Gui Qt6::Test)
add_test(NAME windows-tray-hotkey-diff COMMAND mark-shot-windows-tray-hotkey-diff-test)
