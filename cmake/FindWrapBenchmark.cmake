########################################################################################################################
#
# Library: GenericKit
#
# Copyright (C) 2026~Present ChengXueWen.
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
# http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
#
########################################################################################################################

# We can't create the same interface imported target multiple times, CMake will complain if we do
# that. This can happen if the find_package call is done in multiple different subdirectories.
if(TARGET GKitWrapBenchmark::WrapBenchmark)
    set(GKitWrapBenchmark_FOUND ON)
    return()
endif()

set(GKitWrapBenchmark_NAME "benchmark-1.8.4")
set(GKitWrapBenchmark_PKG_NAME "${GKitWrapBenchmark_NAME}.tar.gz")
set(GKitWrapBenchmark_DIR_NAME "${GKitWrapBenchmark_NAME}-${GKIT_LOWER_BUILD_TYPE}")
set(GKitWrapBenchmark_URL_PATH "${PROJECT_SOURCE_DIR}/3rdparty/${GKitWrapBenchmark_PKG_NAME}")
set(GKitWrapBenchmark_ROOT_DIR "${PROJECT_BINARY_DIR}/3rdparty/${GKitWrapBenchmark_DIR_NAME}")
set(GKitWrapBenchmark_BUILD_DIR "${GKitWrapBenchmark_ROOT_DIR}/build" CACHE INTERNAL "" FORCE)
set(GKitWrapBenchmark_SOURCE_DIR "${GKitWrapBenchmark_ROOT_DIR}/source" CACHE INTERNAL "" FORCE)
set(GKitWrapBenchmark_INSTALL_DIR "${GKitWrapBenchmark_ROOT_DIR}/install" CACHE INTERNAL "" FORCE)
gkit_stamp_file_info(GKitWrapBenchmark OUTPUT_DIR "${GKitWrapBenchmark_ROOT_DIR}")
gkit_fetch_3rdparty(GKitWrapBenchmark URL "${GKitWrapBenchmark_URL_PATH}" OUTPUT_NAME "${GKitWrapBenchmark_DIR_NAME}")
if(NOT EXISTS "${GKitWrapBenchmark_STAMP_FILE_PATH}")
    if(NOT EXISTS ${GKitWrapBenchmark_SOURCE_DIR})
        message(FATAL_ERROR "${GKitWrapBenchmark_DIR_NAME} FetchContent failed.")
    endif()
    gkit_reset_dir(${GKitWrapBenchmark_BUILD_DIR})

    message(STATUS "Configure ${GKitWrapBenchmark_DIR_NAME} lib...")
    execute_process(
        COMMAND ${CMAKE_COMMAND}
        -G ${CMAKE_GENERATOR}
        -DBUILD_SHARED_LIBS=OFF
        -DBENCHMARK_ENABLE_TESTING=OFF
        -DCMAKE_POSITION_INDEPENDENT_CODE=ON
        -DCMAKE_BUILD_TYPE=${CMAKE_BUILD_TYPE}
        -DCMAKE_CONFIGURATION_TYPES=${CMAKE_BUILD_TYPE}
        -DCMAKE_INSTALL_PREFIX=${GKitWrapBenchmark_INSTALL_DIR}
        ${GKitWrapBenchmark_SOURCE_DIR}
        WORKING_DIRECTORY "${GKitWrapBenchmark_BUILD_DIR}"
        RESULT_VARIABLE CONFIGURE_RESULT)
    if(NOT CONFIGURE_RESULT EQUAL 0)
        message(FATAL_ERROR "${GKitWrapBenchmark_DIR_NAME} configure failed.")
    endif()
    message(STATUS "${GKitWrapBenchmark_DIR_NAME} configure success")

    message(STATUS "Build ${GKitWrapBenchmark_DIR_NAME} lib...")
    execute_process(
        COMMAND ${CMAKE_COMMAND} --build ./ --parallel ${GKIT_NUMBER_OF_ASYNC_JOBS} --config ${CMAKE_BUILD_TYPE} --target install
        WORKING_DIRECTORY "${GKitWrapBenchmark_BUILD_DIR}"
        RESULT_VARIABLE BUILD_RESULT)
    if(NOT BUILD_RESULT EQUAL 0)
        message(FATAL_ERROR "${GKitWrapBenchmark_DIR_NAME} build failed.")
    endif()
    message(STATUS "${GKitWrapBenchmark_DIR_NAME} build success")

    execute_process(
        COMMAND ${CMAKE_COMMAND} --install ./
        WORKING_DIRECTORY "${GKitWrapBenchmark_BUILD_DIR}"
        RESULT_VARIABLE INSTALL_RESULT)
    if(NOT INSTALL_RESULT EQUAL 0)
        message(FATAL_ERROR "${GKitWrapBenchmark_DIR_NAME} install failed.")
    endif()
    message(STATUS "${GKitWrapBenchmark_DIR_NAME} install success")
    gkit_make_stamp_file("${GKitWrapBenchmark_STAMP_FILE_PATH}")
endif()

add_library(GKitWrapBenchmark::WrapBenchmark INTERFACE IMPORTED)
gkit_find_package(benchmark PATHS ${GKitWrapBenchmark_INSTALL_DIR} NO_DEFAULT_PATH REQUIRED
    PROVIDED_TARGETS benchmark::benchmark benchmark::benchmark_main)
target_link_libraries(GKitWrapBenchmark::WrapBenchmark INTERFACE benchmark::benchmark benchmark::benchmark_main)
set(GKitWrapBenchmark_FOUND ON)
