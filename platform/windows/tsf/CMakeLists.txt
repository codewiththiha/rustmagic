# RustMagic's Windows text service.
#
# Builds RustMagicTSF.dll: the TSF input method that makes RustMagic selectable
# in Windows Settings and heard in every application. The rules do not live
# here; they come from the engine through the C ABI in crates/km-ffi, linked in
# as a static library so the DLL has nothing to load at run time but ole32,
# oleaut32 and the TSF manager every Windows already has.
#
#   cmake -B build -G "Visual Studio 17 2022" -A x64 .
#   cmake --build build --config Release
#
# On a Linux host with mingw-w64 the same file cross-compiles, which is how the
# code is syntax-checked here without a Windows machine:
#
#   cmake -B build -DCMAKE_TOOLCHAIN_FILE=cross/mingw.cmake .
#
# Options:
#   RUSTMAGIC_FFI_LIB      prebuilt libkm_ffi.a (skips the cargo step)
#   RUSTMAGIC_FFI_INCLUDE  directory holding rustmagic.h
#   RUSTMAGIC_ICON         a prebuilt rustmagic.ico (skips icons.sh)
cmake_minimum_required(VERSION 3.20)
project(RustMagicTSF LANGUAGES CXX RC)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

if(NOT CMAKE_BUILD_TYPE AND NOT CMAKE_CONFIGURATION_TYPES)
    set(CMAKE_BUILD_TYPE Release CACHE STRING "" FORCE)
endif()

set(REPO_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/../../..")

# ----------------------------------------------------------------- the engine

set(RUSTMAGIC_FFI_INCLUDE "${REPO_ROOT}/crates/km-ffi/include" CACHE PATH
    "Directory holding rustmagic.h")
if(NOT EXISTS "${RUSTMAGIC_FFI_INCLUDE}/rustmagic.h")
    message(FATAL_ERROR "rustmagic.h not found in ${RUSTMAGIC_FFI_INCLUDE}")
endif()

# A static library, so ask the linker for the whole archive only if a symbol is
# ever dropped; the engine is reached from several translation units.
set(RUSTMAGIC_FFI_LIB "" CACHE FILEPATH "Prebuilt libkm_ffi.a")
if(NOT RUSTMAGIC_FFI_LIB)
    find_program(CARGO_EXECUTABLE cargo)
    if(NOT CARGO_EXECUTABLE)
        message(FATAL_ERROR
            "cargo not found and RUSTMAGIC_FFI_LIB not set: the text service needs "
            "libkm_ffi.a, built with `cargo build -p km-ffi --release --target "
            "<x86_64|aarch64>-pc-windows-gnu`")
    endif()
    if(CMAKE_SIZEOF_VOID_P EQUAL 8)
        if(MSVC)
            set(FFI_TARGET x86_64-pc-windows-msvc)
        else()
            set(FFI_TARGET x86_64-pc-windows-gnu)
        endif()
    else()
        set(FFI_TARGET i686-pc-windows-gnu)
    endif()
    # cargo names the archive after the toolchain's conventions
    if(MSVC)
        set(FFI_ARCHIVE km_ffi.lib)
    else()
        set(FFI_ARCHIVE libkm_ffi.a)
    endif()
    set(FFI_OUT "${CMAKE_CURRENT_BINARY_DIR}/ffi")
    add_custom_command(
        OUTPUT "${FFI_OUT}/${FFI_ARCHIVE}"
        COMMAND ${CMAKE_COMMAND} -E env ${CARGO_EXECUTABLE} build -p km-ffi --release
                --target ${FFI_TARGET} --target-dir "${FFI_OUT}"
        COMMAND ${CMAKE_COMMAND} -E copy_if_different
                "${FFI_OUT}/${FFI_TARGET}/release/${FFI_ARCHIVE}" "${FFI_OUT}/${FFI_ARCHIVE}"
        WORKING_DIRECTORY "${REPO_ROOT}"
        COMMENT "Building the engine (cargo build -p km-ffi --target ${FFI_TARGET})"
        VERBATIM)
    add_custom_target(km_ffi_static DEPENDS "${FFI_OUT}/${FFI_ARCHIVE}")
    set(RUSTMAGIC_FFI_BUILT "${FFI_OUT}/${FFI_ARCHIVE}")
else()
    add_custom_target(km_ffi_static)
    set(RUSTMAGIC_FFI_BUILT "${RUSTMAGIC_FFI_LIB}")
endif()

# ------------------------------------------------------------------- the icon

# Language settings draws the icon a profile points at. It comes from the one
# brand SVG, never from a committed bitmap; when no renderer is available the
# DLL is built without one rather than failing.
set(RUSTMAGIC_ICON "" CACHE FILEPATH "Prebuilt rustmagic.ico")
set(RUSTMAGIC_ICON_FILE "")
if(RUSTMAGIC_ICON)
    set(RUSTMAGIC_ICON_FILE "${RUSTMAGIC_ICON}")
else()
    find_program(RSVG_CONVERT rsvg-convert)
    find_program(IMAGEMAGICK_CONVERT NAMES convert magick)
    if(RSVG_CONVERT AND IMAGEMAGICK_CONVERT)
        set(ICON_OUT "${CMAKE_CURRENT_BINARY_DIR}/icons")
        add_custom_command(
            OUTPUT "${ICON_OUT}/rustmagic.ico"
            COMMAND sh "${REPO_ROOT}/tools/package/icons.sh" "${ICON_OUT}"
            WORKING_DIRECTORY "${REPO_ROOT}"
            COMMENT "Rendering the brand SVG into rustmagic.ico"
            VERBATIM)
        add_custom_target(rustmagic_icon DEPENDS "${ICON_OUT}/rustmagic.ico")
        set(RUSTMAGIC_ICON_FILE "${ICON_OUT}/rustmagic.ico")
    else()
        message(WARNING
            "rsvg-convert and ImageMagick are needed for the profile icon; "
            "building the text service without one")
    endif()
endif()

# -------------------------------------------------------------------- the DLL

add_library(RustMagicTSF SHARED
    class_factory.cpp
    display_attribute.cpp
    dllmain.cpp
    engine.cpp
    text_service.cpp
    tsf_guids.cpp
    tsf.def)

target_include_directories(RustMagicTSF PRIVATE "${RUSTMAGIC_FFI_INCLUDE}")
add_dependencies(RustMagicTSF km_ffi_static)

# The engine is a Rust static library, and a static library does not carry the
# system libraries the Rust standard library calls: the futex parking, sockets
# and the crypto it links against are the host's, so they are named here. A
# missing one of these is an undefined reference to WakeByAddressSingle or
# WSAGetLastError at the very last step of the build.
target_link_libraries(RustMagicTSF PRIVATE
    "${RUSTMAGIC_FFI_BUILT}"
    ole32 oleaut32 uuid advapi32 shlwapi user32 kernel32
    ws2_32 userenv bcrypt ntdll synchronization)

if(RUSTMAGIC_ICON_FILE)
    get_filename_component(ICON_DIR "${RUSTMAGIC_ICON_FILE}" DIRECTORY)
    get_filename_component(ICON_NAME "${RUSTMAGIC_ICON_FILE}" NAME)
    configure_file(resource.rc.in "${CMAKE_CURRENT_BINARY_DIR}/resource.rc" @ONLY)
    target_sources(RustMagicTSF PRIVATE "${CMAKE_CURRENT_BINARY_DIR}/resource.rc")
    target_include_directories(RustMagicTSF PRIVATE "${ICON_DIR}")
    if(RSVG_CONVERT AND NOT RUSTMAGIC_ICON)
        add_dependencies(RustMagicTSF rustmagic_icon)
    endif()
endif()

# The exports live in the .def; do not let a compiler decide the names.
if(MSVC)
    target_compile_options(RustMagicTSF PRIVATE /W4 /permissive- /EHsc /utf-8)
    target_link_options(RustMagicTSF PRIVATE /DEF:"${CMAKE_CURRENT_SOURCE_DIR}/tsf.def")
else()
    target_compile_options(RustMagicTSF PRIVATE -Wall -Wextra -Wpedantic -finput-charset=UTF-8)
    # A text service is loaded into other people's processes, so it may not
    # depend on the compiler's own runtime DLLs: they exist on the machine that
    # built it and on no one else's. Static libgcc and libstdc++, stripped in a
    # release build.
    target_link_options(RustMagicTSF PRIVATE
        -static-libgcc -static-libstdc++
        $<$<CONFIG:Release>:-s>
        "-Wl,--kill-at"
        "${CMAKE_CURRENT_SOURCE_DIR}/tsf.def")
endif()

set_target_properties(RustMagicTSF PROPERTIES
    PREFIX ""
    OUTPUT_NAME "RustMagicTSF"
    SUFFIX ".dll")
