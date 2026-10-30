cmake_minimum_required(VERSION 3.20)

project(vifastkey_tsf LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

set(VFK_RUST_PROFILE "release" CACHE STRING "Rust profile đã build vfk-ffi")
set(VFK_FFI_LIB_DIR
    "${CMAKE_CURRENT_SOURCE_DIR}/../../rust/target/${VFK_RUST_PROFILE}"
    CACHE PATH "Thư mục chứa vifastkey.dll và import library")
set(VFK_FFI_INCLUDE_DIR
    "${CMAKE_CURRENT_SOURCE_DIR}/../../rust/crates/vfk-ffi/include"
    CACHE PATH "Thư mục chứa vifastkey.h")

if(NOT EXISTS "${VFK_FFI_INCLUDE_DIR}/vifastkey.h")
  message(FATAL_ERROR "Không thấy vifastkey.h trong ${VFK_FFI_INCLUDE_DIR}")
endif()

set(VFK_FFI_IMPORT_LIB "${VFK_FFI_LIB_DIR}/vifastkey.dll.lib")
if(NOT EXISTS "${VFK_FFI_IMPORT_LIB}")
  set(VFK_FFI_IMPORT_LIB "${VFK_FFI_LIB_DIR}/vifastkey.lib")
endif()
if(NOT EXISTS "${VFK_FFI_IMPORT_LIB}")
  message(FATAL_ERROR
    "Không thấy import library của vfk-ffi trong ${VFK_FFI_LIB_DIR}. "
    "Chạy: cargo build --release -p vfk-ffi (đúng target/arch).")
endif()

add_library(vifastkey_tsf SHARED
  src/class_factory.cpp
  src/config.cpp
  src/display_attribute.cpp
  src/dllmain.cpp
  src/engine.cpp
  src/globals.h
  src/guids.cpp
  src/keymap.cpp
  src/registration.cpp
  src/routing.cpp
  src/text_service.cpp
  src/text_service.h
  src/utf.cpp
)

target_include_directories(vifastkey_tsf PRIVATE
  "${CMAKE_CURRENT_SOURCE_DIR}/src"
  "${VFK_FFI_INCLUDE_DIR}"
)

target_compile_definitions(vifastkey_tsf PRIVATE
  UNICODE
  _UNICODE
  WIN32_LEAN_AND_MEAN
  NOMINMAX
  WINVER=0x0A00
  _WIN32_WINNT=0x0A00
)

target_link_libraries(vifastkey_tsf PRIVATE
  "${VFK_FFI_IMPORT_LIB}"
  advapi32
  ole32
  oleaut32
  shell32
  user32
  uuid
)

set_target_properties(vifastkey_tsf PROPERTIES
  OUTPUT_NAME "vifastkey_tsf"
  PREFIX ""
)

if(CMAKE_SIZEOF_VOID_P EQUAL 8)
  target_sources(vifastkey_tsf PRIVATE src/vifastkey_tsf_x64.def)
else()
  target_sources(vifastkey_tsf PRIVATE src/vifastkey_tsf_x86.def)
endif()

if(MSVC)
  target_compile_options(vifastkey_tsf PRIVATE /W4 /EHsc /permissive- /utf-8)
endif()

add_custom_command(TARGET vifastkey_tsf POST_BUILD
  COMMAND ${CMAKE_COMMAND} -E copy_if_different
          "${VFK_FFI_LIB_DIR}/vifastkey.dll"
          "$<TARGET_FILE_DIR:vifastkey_tsf>/vifastkey.dll"
  COMMENT "Copy vifastkey.dll cạnh TSF DLL"
)
