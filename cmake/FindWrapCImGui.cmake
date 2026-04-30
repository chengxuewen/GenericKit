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
# cimgui + imgui (docking) — C bindings for Dear ImGui
# Builds a single static library: cimgui core + SDL3 + OpenGL3 backends.

if(TARGET GKitWrapCImGui::WrapCImGui)
    set(GKitWrapCImGui_FOUND ON)
    return()
endif()

set(GKitWrapCImGui_NAME "cimgui-1.92.7dock")
set(GKitWrapCImGui_PKG_NAME "${GKitWrapCImGui_NAME}.tar.gz")
set(GKitWrapCImGui_DIR_NAME "${GKitWrapCImGui_NAME}-${GKIT_LOWER_BUILD_TYPE}")
set(GKitWrapCImGui_URL_PATH "${PROJECT_SOURCE_DIR}/3rdparty/${GKitWrapCImGui_PKG_NAME}")
set(GKitWrapCImGui_ROOT_DIR "${PROJECT_BINARY_DIR}/3rdparty/${GKitWrapCImGui_DIR_NAME}")
set(GKitWrapCImGui_BUILD_DIR "${GKitWrapCImGui_ROOT_DIR}/build" CACHE INTERNAL "" FORCE)
set(GKitWrapCImGui_SOURCE_DIR "${GKitWrapCImGui_ROOT_DIR}/source" CACHE INTERNAL "" FORCE)
set(GKitWrapCImGui_INSTALL_DIR "${GKitWrapCImGui_ROOT_DIR}/install" CACHE INTERNAL "" FORCE)
set(GKitWrapCImGui_BE_DIR "${GKitWrapCImGui_ROOT_DIR}/be_build" CACHE INTERNAL "" FORCE)
gkit_stamp_file_info(GKitWrapCImGui OUTPUT_DIR "${GKitWrapCImGui_ROOT_DIR}")

gkit_fetch_3rdparty(GKitWrapCImGui URL "${GKitWrapCImGui_URL_PATH}" OUTPUT_NAME "${GKitWrapCImGui_DIR_NAME}")

if(NOT EXISTS "${GKitWrapCImGui_STAMP_FILE_PATH}")
    if(NOT EXISTS ${GKitWrapCImGui_SOURCE_DIR})
        message(FATAL_ERROR "${GKitWrapCImGui_NAME} FetchContent failed.")
    endif()

    # Extract imgui source
    set(_imgui_dir "${GKitWrapCImGui_SOURCE_DIR}/imgui")
    file(GLOB _imgui_files "${_imgui_dir}/*")
    if(NOT _imgui_files)
        file(MAKE_DIRECTORY "${_imgui_dir}")
        set(_imgui_pkg "${PROJECT_SOURCE_DIR}/3rdparty/imgui-1.92.7-docking.tar.gz")
        message(STATUS "Extracting imgui to ${_imgui_dir} ...")
        execute_process(
            COMMAND tar xzf "${_imgui_pkg}" --strip-components=1
            WORKING_DIRECTORY "${_imgui_dir}"
            RESULT_VARIABLE _extract_rc)
        if(NOT _extract_rc EQUAL 0)
            message(FATAL_ERROR "Failed to extract imgui")
        endif()
    endif()

    gkit_reset_dir(${GKitWrapCImGui_BUILD_DIR})

    # Step 1: Build cimgui core (without backends)
    message(STATUS "Build ${GKitWrapCImGui_NAME} core ...")
    execute_process(
        COMMAND ${CMAKE_COMMAND}
        -G ${CMAKE_GENERATOR}
        -DIMGUI_STATIC=ON
        -DCIMGUI_USE_SDL3=ON
        -DCMAKE_BUILD_TYPE=Release
        -DCMAKE_POSITION_INDEPENDENT_CODE=ON
        -DCMAKE_INSTALL_PREFIX=${GKitWrapCImGui_INSTALL_DIR}
        ${GKitWrapCImGui_SOURCE_DIR}
        WORKING_DIRECTORY "${GKitWrapCImGui_BUILD_DIR}"
        RESULT_VARIABLE _cfg_rc)
    if(NOT _cfg_rc EQUAL 0)
        message(FATAL_ERROR "${GKitWrapCImGui_NAME} configure failed.")
    endif()

    execute_process(
        COMMAND ${CMAKE_COMMAND} --build ./ --parallel ${GKIT_NUMBER_OF_ASYNC_JOBS} --config Release --target install
        WORKING_DIRECTORY "${GKitWrapCImGui_BUILD_DIR}"
        RESULT_VARIABLE _build_rc)
    if(NOT _build_rc EQUAL 0)
        message(FATAL_ERROR "${GKitWrapCImGui_NAME} build failed.")
    endif()

    # Step 2: Compile backend sources and merge into cimgui.a
    message(STATUS "Build ${GKitWrapCImGui_NAME} backends (SDL3+OpenGL3) ...")
    set(_be_sources
        "${GKitWrapCImGui_SOURCE_DIR}/cimgui_impl.cpp"
        "${GKitWrapCImGui_SOURCE_DIR}/imgui/backends/imgui_impl_sdl3.cpp"
        "${GKitWrapCImGui_SOURCE_DIR}/imgui/backends/imgui_impl_opengl3.cpp")
    set(_be_includes
        "${GKitWrapCImGui_SOURCE_DIR}"
        "${GKitWrapCImGui_SOURCE_DIR}/imgui"
        "${GKitWrapCImGui_SOURCE_DIR}/imgui/backends")

    set(_be_objs "")
    foreach(_src ${_be_sources})
        get_filename_component(_name "${_src}" NAME_WE)
        set(_obj "${GKitWrapCImGui_BE_DIR}/${_name}.o")
        file(MAKE_DIRECTORY "${GKitWrapCImGui_BE_DIR}")
        set(_inc_flags "")
        foreach(_d ${_be_includes})
            list(APPEND _inc_flags -I"${_d}")
        endforeach()
        execute_process(
            COMMAND ${CMAKE_CXX_COMPILER} -c -O2 -fPIC -DCIMGUI_USE_SDL3
                ${_inc_flags} -std=c++17
                -o "${_obj}" "${_src}"
            WORKING_DIRECTORY "${GKitWrapCImGui_BE_DIR}"
            RESULT_VARIABLE _cc_rc)
        if(NOT _cc_rc EQUAL 0)
            message(FATAL_ERROR "Failed to compile ${_src}")
        endif()
        list(APPEND _be_objs "${_obj}")
    endforeach()

    # Merge backend objects into cimgui.a
    execute_process(
        COMMAND ${CMAKE_AR} qs "${GKitWrapCImGui_INSTALL_DIR}/cimgui.a" ${_be_objs}
        WORKING_DIRECTORY "${GKitWrapCImGui_BE_DIR}"
        RESULT_VARIABLE _ar_rc)
    if(NOT _ar_rc EQUAL 0)
        message(FATAL_ERROR "Failed to merge backend objects into cimgui.a")
    endif()

    gkit_make_stamp_file("${GKitWrapCImGui_STAMP_FILE_PATH}")
endif()

# Wrap target
add_library(GKitWrapCImGui::WrapCImGui INTERFACE IMPORTED)
target_include_directories(GKitWrapCImGui::WrapCImGui INTERFACE
    "${GKitWrapCImGui_SOURCE_DIR}"
    "${GKitWrapCImGui_SOURCE_DIR}/imgui"
    "${GKitWrapCImGui_SOURCE_DIR}/imgui/backends")
target_link_libraries(GKitWrapCImGui::WrapCImGui INTERFACE
    "${GKitWrapCImGui_INSTALL_DIR}/cimgui.a")
set(GKitWrapCImGui_FOUND ON)
