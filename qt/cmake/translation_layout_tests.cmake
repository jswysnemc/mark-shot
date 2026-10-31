# 1. 【译文排版】【段落】识别结果合并段落与翻译输入输出，不依赖字体
qt_add_executable(mark-shot-translation-paragraph-blocks-test
    tests/translation_paragraph_blocks_test.cpp
    ${MARK_SHOT_TRANSLATION_LAYOUT_SOURCES}
    src/ocr_result.cpp
    src/ocr_result.h
)
target_include_directories(mark-shot-translation-paragraph-blocks-test PRIVATE src)
target_link_libraries(mark-shot-translation-paragraph-blocks-test PRIVATE Qt6::Core Qt6::Gui Qt6::Test)
add_test(NAME translation-paragraph-blocks COMMAND mark-shot-translation-paragraph-blocks-test)

# 2. 【译文排版】【排版绘制】取色、字号适配、底板延伸与转贴图换算，需要字体引擎
qt_add_executable(mark-shot-translation-block-layout-test
    tests/translation_block_layout_test.cpp
    ${MARK_SHOT_TRANSLATION_LAYOUT_SOURCES}
    src/ocr_result.cpp
    src/ocr_result.h
)
target_include_directories(mark-shot-translation-block-layout-test PRIVATE src)
target_link_libraries(mark-shot-translation-block-layout-test PRIVATE Qt6::Core Qt6::Gui Qt6::Test)
add_test(NAME translation-block-layout COMMAND mark-shot-translation-block-layout-test)
set_tests_properties(translation-block-layout PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen")
