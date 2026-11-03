qt_add_executable(mark-shot-capture-history-store-test
    tests/capture_history_store_test.cpp
    src/capture_history/history_store.cpp
    src/capture_history/history_store.h
)
target_include_directories(mark-shot-capture-history-store-test PRIVATE src)
target_link_libraries(mark-shot-capture-history-store-test PRIVATE Qt6::Core Qt6::Gui Qt6::Test)
add_test(NAME capture-history-store COMMAND mark-shot-capture-history-store-test)

qt_add_executable(mark-shot-history-preview-cache-test
    tests/history_preview_cache_test.cpp
    src/capture_history/history_preview_cache.cpp
    src/capture_history/history_store.cpp)
target_include_directories(mark-shot-history-preview-cache-test PRIVATE src)
target_link_libraries(mark-shot-history-preview-cache-test PRIVATE Qt6::Core Qt6::Gui Qt6::Test)
add_test(NAME history-preview-cache COMMAND mark-shot-history-preview-cache-test)
