cmake_minimum_required(VERSION 3.10)

set(FLUTTER_APP_DIR "${CMAKE_CURRENT_SOURCE_DIR}/..")
set(BINARY_NAME "rs_ffi_barrage_example")

add_library(flutter INTERFACE)

target_include_directories(flutter INTERFACE
    "${FLUTTER_ROOT}/include"
)

target_link_libraries(flutter INTERFACE
    "${FLUTTER_ROOT}/lib/libflutter_linux_gtk.so"
)

add_custom_target(flutter_assemblies
    COMMAND "${FLUTTER_ROOT}/bin/flutter assemble"
    --output="${CMAKE_CURRENT_BINARY_DIR}"
    "${FLUTTER_APP_DIR}/pubspec.yaml"
    WORKING_DIRECTORY "${FLUTTER_APP_DIR}"
)