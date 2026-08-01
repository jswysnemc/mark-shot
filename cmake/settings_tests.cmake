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

