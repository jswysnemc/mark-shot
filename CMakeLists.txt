cmake_minimum_required(VERSION 3.20)
project(mark-shot VERSION 0.1.61 LANGUAGES C CXX)
include(CTest)
option(MARK_SHOT_BUILD_QT "Build the Qt desktop application" ON)
include(cmake/release_optimization.cmake)
include(cmake/rust_core.cmake)
if(MARK_SHOT_BUILD_QT)
    # 【构建】【核心边界】应用及回归测试统一链接一个 Rust 运行时归档
    link_libraries(shot_rust_core)
    add_subdirectory(qt)
endif()
if(BUILD_TESTING)
    foreach(crate IN ITEMS shot-geometry shot-image shot-stitch shot-config shot-editor shot-ffi)
        add_test(NAME rust-${crate} COMMAND "${CMAKE_COMMAND}" -E env ${MARK_SHOT_CARGO_ENV}
            "${CARGO_EXECUTABLE}" test ${MARK_SHOT_CARGO_ARGS}
            --manifest-path "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p ${crate})
        set_tests_properties(rust-${crate} PROPERTIES LABELS "rust;unit" TIMEOUT 180)
    endforeach()
endif()
