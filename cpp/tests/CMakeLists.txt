add_executable(test_shim
    test_shim.cpp
)

target_include_directories(test_shim PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/../include
)

target_link_libraries(test_shim PRIVATE
    fish_shim
)

add_test(NAME test_shim COMMAND test_shim)
