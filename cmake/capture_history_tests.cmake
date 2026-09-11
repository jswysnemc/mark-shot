qt_add_executable(mark-shot-capture-history-store-test
    tests/capture_history_store_test.cpp
    src/capture_history/history_store.cpp
    src/capture_history/history_store.h
)
target_include_directories(mark-shot-capture-history-store-test PRIVATE src)
target_link_libraries(mark-shot-capture-history-store-test PRIVATE Qt6::Core Qt6::Gui Qt6::Test)
add_test(NAME capture-history-store COMMAND mark-shot-capture-history-store-test)
