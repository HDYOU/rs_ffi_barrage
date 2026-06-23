cmake_minimum_required(VERSION 3.10)
project(rs_ffi_barrage_example LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_subdirectory(flutter)
add_subdirectory(runner)

target_link_libraries(${BINARY_NAME} PRIVATE flutter)

set_target_properties(${BINARY_NAME} PROPERTIES
    OUTPUT_NAME "rs_ffi_barrage_example"
)

install(TARGETS ${BINARY_NAME} RUNTIME DESTINATION "${CMAKE_INSTALL_BINDIR}")