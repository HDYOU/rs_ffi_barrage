cmake_minimum_required(VERSION 3.10)

set(BINARY_NAME "rs_ffi_barrage_example")

add_executable(${BINARY_NAME}
    main.cc
    my_application.cc
)

target_link_libraries(${BINARY_NAME} PRIVATE
    flutter
    gtk+-3.0
)