cmake_minimum_required(VERSION 3.16)
project(vifastkey-fcitx5 VERSION 0.1.1 LANGUAGES CXX)

include(GNUInstallDirs)
include(CTest)
set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

find_package(Fcitx5Core REQUIRED)
find_package(Fcitx5Utils REQUIRED)

if(NOT VIFASTKEY_LIB)
    set(VIFASTKEY_LIB "${CMAKE_CURRENT_SOURCE_DIR}/../../rust/target/release/libvifastkey.a")
endif()

add_library(vifastkey MODULE
    src/vifastkey.cpp
    src/vifastkey_state.cpp
    src/config_store.cpp)
target_compile_features(vifastkey PRIVATE cxx_std_20)
target_compile_options(vifastkey PRIVATE -Wall -Wextra -Wpedantic)
target_include_directories(vifastkey PRIVATE
    "${CMAKE_CURRENT_SOURCE_DIR}/src"
    "${CMAKE_CURRENT_SOURCE_DIR}/../../rust/crates/vfk-ffi/include")
target_link_libraries(vifastkey PRIVATE
    Fcitx5::Core
    Fcitx5::Utils
    "${VIFASTKEY_LIB}")
if(UNIX AND NOT APPLE)
    target_link_libraries(vifastkey PRIVATE dl pthread m)
endif()

set(FCITX_ADDON_LIBDIR "${CMAKE_INSTALL_LIBDIR}")
if(UNIX AND NOT APPLE)
    find_program(DPKG_ARCHITECTURE dpkg-architecture)
    if(DPKG_ARCHITECTURE)
        execute_process(COMMAND dpkg-architecture -qDEB_HOST_MULTIARCH
            OUTPUT_VARIABLE DEB_MULTIARCH OUTPUT_STRIP_TRAILING_WHITESPACE)
        # GNUInstallDirs trên Debian đã trả về lib/x86_64-linux-gnu. Chỉ thêm
        # multiarch khi libdir vẫn là "lib" để tránh lặp đường dẫn.
        if(DEB_MULTIARCH AND CMAKE_INSTALL_LIBDIR STREQUAL "lib")
            set(FCITX_ADDON_LIBDIR "${CMAKE_INSTALL_LIBDIR}/${DEB_MULTIARCH}")
        endif()
    endif()
endif()
install(TARGETS vifastkey LIBRARY DESTINATION "${FCITX_ADDON_LIBDIR}/fcitx5")
install(FILES vifastkey-addon.conf DESTINATION "${CMAKE_INSTALL_DATADIR}/fcitx5/addon" RENAME vifastkey.conf)
install(FILES vifastkey.conf DESTINATION "${CMAKE_INSTALL_DATADIR}/fcitx5/inputmethod")

# Icon for the input method selector and the status action. Fcitx5 resolves
# these names through the XDG icon theme, same convention as fcitx5-bamboo.
install(FILES
    "${CMAKE_CURRENT_SOURCE_DIR}/../../resources/brand/vifastkey-icon.svg"
    DESTINATION "${CMAKE_INSTALL_DATADIR}/icons/hicolor/scalable/apps"
    RENAME vifastkey.svg)
install(FILES
    "${CMAKE_CURRENT_SOURCE_DIR}/../../resources/brand/vifastkey-icon.svg"
    DESTINATION "${CMAKE_INSTALL_DATADIR}/icons/hicolor/scalable/apps"
    RENAME org.fcitx.Fcitx5.vifastkey.svg)
install(FILES
    "${CMAKE_CURRENT_SOURCE_DIR}/../../resources/brand/vifastkey-icon-192.png"
    DESTINATION "${CMAKE_INSTALL_DATADIR}/icons/hicolor/192x192/apps"
    RENAME vifastkey.png)
install(FILES
    "${CMAKE_CURRENT_SOURCE_DIR}/../../resources/brand/vifastkey-icon-512.png"
    DESTINATION "${CMAKE_INSTALL_DATADIR}/icons/hicolor/512x512/apps"
    RENAME vifastkey.png)

if(BUILD_TESTING)
    add_executable(vifastkey-fcitx5-test
        tests/test_state.cpp
        src/vifastkey_state.cpp
        src/config_store.cpp)
    target_compile_features(vifastkey-fcitx5-test PRIVATE cxx_std_20)
    target_compile_options(vifastkey-fcitx5-test PRIVATE
        -Wall -Wextra -Wpedantic -UNDEBUG)
    target_include_directories(vifastkey-fcitx5-test PRIVATE
        "${CMAKE_CURRENT_SOURCE_DIR}/src"
        "${CMAKE_CURRENT_SOURCE_DIR}/../../rust/crates/vfk-ffi/include")
    target_link_libraries(vifastkey-fcitx5-test PRIVATE
        Fcitx5::Core
        Fcitx5::Utils
        "${VIFASTKEY_LIB}")
    if(UNIX AND NOT APPLE)
        target_link_libraries(vifastkey-fcitx5-test PRIVATE dl pthread m)
    endif()
    add_test(NAME vifastkey-fcitx5 COMMAND vifastkey-fcitx5-test)
endif()
