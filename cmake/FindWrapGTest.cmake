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
if(TARGET GKitWrapGTest::WrapGTest)
    set(GKitWrapGTest_FOUND ON)
    return()
endif()

set(GKitWrapGTest_NAME "googletest-release-1.12.1")
set(GKitWrapGTest_PKG_NAME "${GKitWrapGTest_NAME}.tar.gz")
set(GKitWrapGTest_DIR_NAME "${GKitWrapGTest_NAME}-${GKIT_LOWER_BUILD_TYPE}")
set(GKitWrapGTest_URL_PATH "${PROJECT_SOURCE_DIR}/3rdparty/${GKitWrapGTest_PKG_NAME}")
set(GKitWrapGTest_ROOT_DIR "${PROJECT_BINARY_DIR}/3rdparty/${GKitWrapGTest_DIR_NAME}")
set(GKitWrapGTest_BUILD_DIR "${GKitWrapGTest_ROOT_DIR}/build" CACHE INTERNAL "" FORCE)
set(GKitWrapGTest_SOURCE_DIR "${GKitWrapGTest_ROOT_DIR}/source" CACHE INTERNAL "" FORCE)
set(GKitWrapGTest_INSTALL_DIR "${GKitWrapGTest_ROOT_DIR}/install" CACHE INTERNAL "" FORCE)
gkit_stamp_file_info(GKitWrapGTest OUTPUT_DIR "${GKitWrapGTest_ROOT_DIR}")
gkit_fetch_3rdparty(GKitWrapGTest URL "${GKitWrapGTest_URL_PATH}" OUTPUT_NAME "${GKitWrapGTest_DIR_NAME}")
if(NOT EXISTS "${GKitWrapGTest_STAMP_FILE_PATH}")
    if(NOT EXISTS ${GKitWrapGTest_SOURCE_DIR})
        message(FATAL_ERROR "${GKitWrapGTest_DIR_NAME} FetchContent failed.")
    endif()
    gkit_reset_dir(${GKitWrapGTest_BUILD_DIR})

    message(STATUS "Configure ${GKitWrapGTest_DIR_NAME} lib...")
    execute_process(
        COMMAND ${CMAKE_COMMAND}
        -G ${CMAKE_GENERATOR}
        -DBUILD_SHARED_LIBS=OFF
        -Dgtest_force_shared_crt=ON
        -DCMAKE_POSITION_INDEPENDENT_CODE=ON
        -DCMAKE_BUILD_TYPE=${CMAKE_BUILD_TYPE}
        -DCMAKE_CONFIGURATION_TYPES=${CMAKE_BUILD_TYPE}
        -DCMAKE_INSTALL_PREFIX=${GKitWrapGTest_INSTALL_DIR}
        ${GKitWrapGTest_SOURCE_DIR}
        WORKING_DIRECTORY "${GKitWrapGTest_BUILD_DIR}"
        RESULT_VARIABLE CONFIGURE_RESULT)
    if(NOT CONFIGURE_RESULT EQUAL 0)
        message(FATAL_ERROR "${GKitWrapGTest_DIR_NAME} configure failed.")
    endif()
    message(STATUS "${GKitWrapGTest_DIR_NAME} configure success")

    message(STATUS "Build ${GKitWrapGTest_DIR_NAME} lib...")
    execute_process(
        COMMAND ${CMAKE_COMMAND} --build ./ --parallel ${GKIT_NUMBER_OF_ASYNC_JOBS} --config ${CMAKE_BUILD_TYPE} --target install
        WORKING_DIRECTORY "${GKitWrapGTest_BUILD_DIR}"
        RESULT_VARIABLE BUILD_RESULT)
    if(NOT BUILD_RESULT EQUAL 0)
        message(FATAL_ERROR "${GKitWrapGTest_DIR_NAME} build failed.")
    endif()
    message(STATUS "${GKitWrapGTest_DIR_NAME} build success")

    execute_process(
        COMMAND ${CMAKE_COMMAND} --install ./
        WORKING_DIRECTORY "${GKitWrapGTest_BUILD_DIR}"
        RESULT_VARIABLE INSTALL_RESULT)
    if(NOT INSTALL_RESULT EQUAL 0)
        message(FATAL_ERROR "${GKitWrapGTest_DIR_NAME} install failed.")
    endif()
    message(STATUS "${GKitWrapGTest_DIR_NAME} install success")
    gkit_make_stamp_file("${GKitWrapGTest_STAMP_FILE_PATH}")
endif()

add_library(GKitWrapGTest::WrapGTest INTERFACE IMPORTED)
gkit_find_package(GTest PATHS ${GKitWrapGTest_INSTALL_DIR} NO_DEFAULT_PATH REQUIRED
    PROVIDED_TARGETS GTest::gtest GTest::gtest_main GTest::gmock GTest::gmock_main)
target_link_libraries(GKitWrapGTest::WrapGTest INTERFACE GTest::gtest GTest::gtest_main GTest::gmock GTest::gmock_main)
set(GKitWrapGTest_FOUND ON)
