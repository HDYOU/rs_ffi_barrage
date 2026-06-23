cmake_minimum_required(VERSION 3.14)

set(BINARY_NAME "rs_ffi_barrage_example")

add_executable(${BINARY_NAME} WIN32
    flutter_window.cpp
    main.cpp
    win32_window.cpp
    "${CMAKE_CURRENT_SOURCE_DIR}/resources/runner.rc"
)

target_compile_definitions(${BINARY_NAME} PRIVATE "NOMINMAX")
target_link_libraries(${BINARY_NAME} PRIVATE "flutter")