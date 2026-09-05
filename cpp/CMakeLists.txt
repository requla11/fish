cmake_minimum_required(VERSION 3.20)
project(fish_cpp_shims LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

set(CMAKE_RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bin)
set(CMAKE_LIBRARY_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bin)

if(MSVC)
    add_compile_options(/W4 /WX /permissive- /Zc:__cplusplus)
else()
    add_compile_options(-Wall -Wextra -Werror -pedantic)
endif()

add_library(fish_shim SHARED
    src/log_writer.cpp
    src/shim_main.cpp
    src/interceptor_posix.cpp
    src/interceptor_win.cpp
)

target_include_directories(fish_shim PUBLIC
    include
)

target_compile_definitions(fish_shim PRIVATE FISH_SHIM_EXPORTS)

if(UNIX)
    target_link_libraries(fish_shim PRIVATE ${CMAKE_DL_LIBS})
endif()

enable_testing()
add_subdirectory(tests)
