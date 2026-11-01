find_package(Python3 REQUIRED COMPONENTS Interpreter)

add_test(NAME release-ffmpeg-policy
    COMMAND ${CMAKE_COMMAND} -E env PYTHONDONTWRITEBYTECODE=1
        ${Python3_EXECUTABLE} ${CMAKE_CURRENT_SOURCE_DIR}/tests/release_ffmpeg_policy_test.py
)

add_test(NAME debian-ocr-runtime-dependency
    COMMAND ${CMAKE_COMMAND} -E env PYTHONDONTWRITEBYTECODE=1
        ${Python3_EXECUTABLE} ${CMAKE_CURRENT_SOURCE_DIR}/tests/debian_ocr_dependency_test.py)
