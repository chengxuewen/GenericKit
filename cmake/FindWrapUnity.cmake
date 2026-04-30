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
if(TARGET GKitWrapUnity::WrapUnity)
    set(GKitWrapUnity_FOUND ON)
    return()
endif()

set(GKitWrapUnity_NAME "Unity-2.6.1")
set(GKitWrapUnity_PKG_NAME "${GKitWrapUnity_NAME}.tar.gz")
set(GKitWrapUnity_DIR_NAME "${GKitWrapUnity_NAME}-${GKIT_LOWER_BUILD_TYPE}")
set(GKitWrapUnity_URL_PATH "${PROJECT_SOURCE_DIR}/3rdparty/${GKitWrapUnity_PKG_NAME}")
set(GKitWrapUnity_ROOT_DIR "${PROJECT_BINARY_DIR}/3rdparty/${GKitWrapUnity_DIR_NAME}")
set(GKitWrapUnity_BUILD_DIR "${GKitWrapUnity_ROOT_DIR}/build" CACHE INTERNAL "" FORCE)
set(GKitWrapUnity_SOURCE_DIR "${GKitWrapUnity_ROOT_DIR}/source" CACHE INTERNAL "" FORCE)
set(GKitWrapUnity_INSTALL_DIR "${GKitWrapUnity_ROOT_DIR}/install" CACHE INTERNAL "" FORCE)
gkit_stamp_file_info(GKitWrapUnity OUTPUT_DIR "${GKitWrapUnity_ROOT_DIR}")
gkit_fetch_3rdparty(GKitWrapUnity URL "${GKitWrapUnity_URL_PATH}" OUTPUT_NAME "${GKitWrapUnity_DIR_NAME}")
if(NOT EXISTS "${GKitWrapUnity_STAMP_FILE_PATH}")
    if(NOT EXISTS ${GKitWrapUnity_SOURCE_DIR})
        message(FATAL_ERROR "${GKitWrapUnity_NAME} FetchContent failed.")
    endif()
    gkit_reset_dir(${GKitWrapUnity_BUILD_DIR})

    message(STATUS "Configure ${GKitWrapUnity_NAME} lib...")
    execute_process(
        COMMAND ${CMAKE_COMMAND}
        -G ${CMAKE_GENERATOR}
        -DCMAKE_BUILD_TYPE=Release
        -DCMAKE_POSITION_INDEPENDENT_CODE=ON
        -DCMAKE_INSTALL_PREFIX=${GKitWrapUnity_INSTALL_DIR}
        ${GKitWrapUnity_SOURCE_DIR}
        WORKING_DIRECTORY "${GKitWrapUnity_BUILD_DIR}"
        RESULT_VARIABLE CONFIGURE_RESULT)
    if(NOT CONFIGURE_RESULT MATCHES 0)
        message(FATAL_ERROR "${GKitWrapUnity_NAME} configure failed.")
    endif()
    message(STATUS "${GKitWrapUnity_NAME} configure success")
    
    execute_process(
        COMMAND ${CMAKE_COMMAND} --build ./ --parallel ${GKIT_NUMBER_OF_ASYNC_JOBS} --config Release --target install
        WORKING_DIRECTORY "${GKitWrapUnity_BUILD_DIR}"
        RESULT_VARIABLE BUILD_RESULT)
    if(NOT BUILD_RESULT MATCHES 0)
        message(FATAL_ERROR "${GKitWrapUnity_NAME} build failed.")
    endif()       
    message(STATUS "${GKitWrapUnity_NAME} build success")
            
    execute_process(
        COMMAND ${CMAKE_COMMAND} --install ./
        WORKING_DIRECTORY "${GKitWrapUnity_BUILD_DIR}"
        RESULT_VARIABLE INSTALL_RESULT)
    if(NOT INSTALL_RESULT MATCHES 0)
        message(FATAL_ERROR "${GKitWrapUnity_NAME} install failed.")
    endif()
    message(STATUS "${GKitWrapUnity_NAME} install success")
    gkit_make_stamp_file("${GKitWrapUnity_STAMP_FILE_PATH}")
endif()
# wrap lib
add_library(GKitWrapUnity::WrapUnity INTERFACE IMPORTED)
find_package(unity PATHS ${GKitWrapUnity_INSTALL_DIR} NO_DEFAULT_PATH REQUIRED)
target_link_libraries(GKitWrapUnity::WrapUnity INTERFACE unity::framework)
set(GKitWrapUnity_FOUND ON)
