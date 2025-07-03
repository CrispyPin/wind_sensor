set(PROJECT wind_station)
cmake_minimum_required(VERSION 3.12)
set(PICO_BOARD pico_w)
include(pico_sdk_import.cmake)
project(${PROJECT} C CXX)
pico_sdk_init()
add_executable(${PROJECT} src/main.c)
target_include_directories(${PROJECT} PRIVATE ${CMAKE_CURRENT_LIST_DIR}/src)
target_link_libraries(${PROJECT}
    pico_stdlib
    pico_cyw43_arch_none
    pico_cyw43_arch_lwip_threadsafe_background
    hardware_adc
)
pico_add_extra_outputs(${PROJECT})
