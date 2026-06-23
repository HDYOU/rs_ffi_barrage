cmake_minimum_required(VERSION 3.14)

set(EPHEMERAL_DIR "${CMAKE_CURRENT_SOURCE_DIR}/ephemeral")

if(EXISTS "${EPHEMERAL_DIR}/Generated.xcconfig")
    file(STRINGS "${EPHEMERAL_DIR}/Generated.xcconfig" FLUTTER_EPHEMERAL_CONFIG)
    foreach(LINE ${FLUTTER_EPHEMERAL_CONFIG})
        if(LINE MATCHES "^FLUTTER_ROOT=(.*)$")
            set(FLUTTER_ROOT "${CMAKE_MATCH_1}")
        endif()
    endforeach()
endif()

set(FLUTTER_APP_DIR "${CMAKE_CURRENT_SOURCE_DIR}/..")
set(FLUTTER_ASSETS_DIR "${FLUTTER_APP_DIR}/flutter_assets")
set(BINARY_NAME "rs_ffi_barrage_example")

add_library(flutter INTERFACE)

target_include_directories(flutter INTERFACE
    "${FLUTTER_ROOT}/include"
)

target_link_libraries(flutter INTERFACE
    "${FLUTTER_ROOT}/lib/flutter_windows.dll.lib"
)

add_custom_target(flutter_assemblies
    COMMAND "${FLUTTER_ROOT}/bin/flutter assemble"
    --output="${EPHEMERAL_DIR}"
    --DartDefines="${DART_DEFINES}"
    "${FLUTTER_APP_DIR}/pubspec.yaml"
    WORKING_DIRECTORY "${FLUTTER_APP_DIR}"
)