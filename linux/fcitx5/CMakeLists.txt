cmake_minimum_required(VERSION 3.16)
project(vifastkey-fcitx5 VERSION 0.4.0 LANGUAGES CXX)

include(GNUInstallDirs)
set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

find_package(Fcitx5Core REQUIRED)

if(NOT VIFASTKEY_LIB)
    set(VIFASTKEY_LIB "${CMAKE_CURRENT_SOURCE_DIR}/../../rust/target/release/libvifastkey.a")
endif()

add_library(vifastkey MODULE src/vifastkey.cpp)
target_include_directories(vifastkey PRIVATE
    "${CMAKE_CURRENT_SOURCE_DIR}/../../rust/crates/vfk-ffi/include")
target_link_libraries(vifastkey PRIVATE Fcitx5::Core "${VIFASTKEY_LIB}")
if(UNIX AND NOT APPLE)
    target_link_libraries(vifastkey PRIVATE dl pthread)
endif()

install(TARGETS vifastkey LIBRARY DESTINATION "${CMAKE_INSTALL_LIBDIR}/fcitx5")
install(FILES vifastkey-addon.conf DESTINATION "${CMAKE_INSTALL_DATADIR}/fcitx5/addon")
install(FILES vifastkey.conf DESTINATION "${CMAKE_INSTALL_DATADIR}/fcitx5/inputmethod")
