cmake_minimum_required(VERSION 3.14)
project(rs_ffi_barrage_example LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_subdirectory(flutter)
add_subdirectory(runner)

target_link_libraries(${BINARY_NAME} PRIVATE flutter)
target_link_libraries(${BINARY_NAME} PRIVATE runner)

set_target_properties(${BINARY_NAME} PROPERTIES
    OUTPUT_NAME "rs_ffi_barrage_example"
    WIN32_EXECUTABLE TRUE
)

install(TARGETS ${BINARY_NAME} RUNTIME DESTINATION "${CMAKE_INSTALL_BINDIR}")