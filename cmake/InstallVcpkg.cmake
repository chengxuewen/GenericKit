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
# Ported from OpenCTK cmake/InstallVcpkg.cmake
#
# Provides:
#   gkit_vcpkg_install()            — clone + bootstrap vcpkg (two copies: regular + tools)
#   gkit_vcpkg_install_package()    — install + export + pack + import a vcpkg package
#-----------------------------------------------------------------------------------------------------------------------

function(gkit_vcpkg_install)
    if(EXISTS "${GKIT_VCPKG_EXECUTABLE}" AND EXISTS "${GKIT_VCPKG_TOOLS_EXECUTABLE}")
        set(GKIT_VCPKG_FOUND ON)
        return()
    endif()

    set(GKIT_VCPKG_NAME "vcpkg")
    set(GKIT_VCPKG_ROOT_DIR "${CMAKE_SOURCE_DIR}/vcpkg" CACHE INTERNAL "" FORCE)
    set(GKIT_VCPKG_TOOLS_ROOT_DIR "${CMAKE_SOURCE_DIR}/vcpkg-tools" CACHE INTERNAL "" FORCE)
    find_package(Git REQUIRED)

    if(WIN32)
        set(_exe_name "vcpkg.exe")
        set(_bootstrap "bootstrap-vcpkg.bat")
    else()
        set(_exe_name "vcpkg")
        set(_bootstrap "./bootstrap-vcpkg.sh")
    endif()

    # Clone vcpkg if not exists
    if(NOT EXISTS "${GKIT_VCPKG_ROOT_DIR}/${_bootstrap}")
        if(EXISTS "${GKIT_VCPKG_ROOT_DIR}")
            execute_process(
                COMMAND ${CMAKE_COMMAND} -E remove_directory "${GKIT_VCPKG_ROOT_DIR}"
                WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}"
                RESULT_VARIABLE _rmdir_rc)
            if(NOT _rmdir_rc EQUAL 0)
                message(FATAL_ERROR "Failed to remove stale vcpkg directory")
            endif()
        endif()
        message(STATUS "Cloning vcpkg into ${CMAKE_SOURCE_DIR} ...")
        execute_process(
            COMMAND "${GIT_EXECUTABLE}" clone https://github.com/microsoft/vcpkg.git --depth 1
            WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}"
            RESULT_VARIABLE _clone_rc)
        if(NOT _clone_rc EQUAL 0)
            message(FATAL_ERROR "vcpkg clone failed")
        endif()
    endif()

    set(GKIT_VCPKG_EXECUTABLE "${GKIT_VCPKG_ROOT_DIR}/${_exe_name}" CACHE INTERNAL "" FORCE)

    # Bootstrap if needed
    if(NOT EXISTS "${GKIT_VCPKG_ROOT_DIR}/${_exe_name}")
        message(STATUS "Bootstrapping vcpkg...")
        if(WIN32)
            execute_process(COMMAND "${_bootstrap}" WORKING_DIRECTORY "${GKIT_VCPKG_ROOT_DIR}" RESULT_VARIABLE _init_rc)
        else()
            execute_process(COMMAND sh "${_bootstrap}" WORKING_DIRECTORY "${GKIT_VCPKG_ROOT_DIR}" RESULT_VARIABLE _init_rc)
        endif()
        if(NOT _init_rc EQUAL 0)
            message(FATAL_ERROR "vcpkg bootstrap failed")
        endif()
    endif()

    # Create tools copy for packages that need a clean vcpkg
    if(NOT EXISTS "${GKIT_VCPKG_TOOLS_ROOT_DIR}/.git")
        execute_process(
            COMMAND ${CMAKE_COMMAND} -E copy_directory "${GKIT_VCPKG_ROOT_DIR}" "${GKIT_VCPKG_TOOLS_ROOT_DIR}"
            WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}"
            RESULT_VARIABLE _copy_rc)
        if(NOT _copy_rc EQUAL 0)
            message(FATAL_ERROR "Failed to create vcpkg-tools copy")
        endif()
    endif()

    set(GKIT_VCPKG_TOOLS_EXECUTABLE "${GKIT_VCPKG_TOOLS_ROOT_DIR}/${_exe_name}" CACHE INTERNAL "" FORCE)
    set(GKIT_VCPKG_FOUND ON)
endfunction()


gkit_set_input_variable(GKIT_3RDPARTY_PACKAGES_DIR)

function(gkit_vcpkg_install_package NAME)
    gkit_parse_all_arguments(arg "gkit_vcpkg_install_package"
        "NOT_IMPORT;TOOLS;DYNAMIC;QUIET"
        "TARGET;PREFIX;OUTPUT_DIR;PACK_NAME;VERSION"
        "COMPONENTS;IMPORTED_TARGETS" ${ARGN})

    if(NOT arg_TARGET)
        set(arg_TARGET ${NAME})
    endif()
    if(NOT arg_PREFIX)
        string(REGEX REPLACE "[^a-zA-Z0-9]" "" arg_PREFIX "${arg_TARGET}")
    endif()
    if(NOT arg_OUTPUT_DIR)
        set(arg_OUTPUT_DIR "${PROJECT_BINARY_DIR}/3rdparty/vcpkg")
    endif()
    if(NOT arg_PACK_NAME)
        set(arg_PACK_NAME ${NAME})
    endif()

    if(TARGET ${arg_TARGET})
        set(${arg_PREFIX}_FOUND ON PARENT_SCOPE)
        return()
    endif()

    # Determine triplet
    if(WIN32)
        if(arg_DYNAMIC)
            set(_triplet "x64-windows")
        else()
            set(_triplet "x64-windows-static-md")
        endif()
    elseif(APPLE)
        if(CMAKE_SYSTEM_PROCESSOR MATCHES "arm64")
            if(arg_DYNAMIC)
                set(_triplet "arm64-osx-dynamic")
            else()
                set(_triplet "arm64-osx")
            endif()
        else()
            if(arg_DYNAMIC)
                set(_triplet "x64-osx-dynamic")
            else()
                set(_triplet "x64-osx")
            endif()
        endif()
    else()
        if(arg_DYNAMIC)
            set(_triplet "x64-linux-dynamic")
        else()
            set(_triplet "x64-linux")
        endif()
    endif()

    # Cached metadata
    set(${arg_PREFIX}_INSTALLED OFF CACHE INTERNAL "" FORCE)
    set(${arg_PREFIX}_NAME "${arg_PACK_NAME}" CACHE INTERNAL "" FORCE)
    set(${arg_PREFIX}_ROOT_DIR "${arg_OUTPUT_DIR}/${arg_PACK_NAME}" CACHE INTERNAL "" FORCE)
    set(${arg_PREFIX}_INSTALL_DIR "${${arg_PREFIX}_ROOT_DIR}/installed/${_triplet}" CACHE INTERNAL "" FORCE)

    # Flow: use cached export if available, otherwise install + export via vcpkg
    if(NOT EXISTS "${${arg_PREFIX}_INSTALL_DIR}/include")
        gkit_vcpkg_install()

        if(arg_TOOLS)
            set(_vcpkg_exe "${GKIT_VCPKG_TOOLS_EXECUTABLE}")
            set(_vcpkg_root "${GKIT_VCPKG_TOOLS_ROOT_DIR}")
        else()
            set(_vcpkg_exe "${GKIT_VCPKG_EXECUTABLE}")
            set(_vcpkg_root "${GKIT_VCPKG_ROOT_DIR}")
        endif()

        # Build component/feature spec
        unset(_comp_config)
        if(arg_COMPONENTS)
            set(_comp_config "[")
            foreach(_c IN LISTS arg_COMPONENTS)
                if(NOT "${_comp_config}" STREQUAL "[")
                    set(_comp_config "${_comp_config},")
                endif()
                set(_comp_config "${_comp_config}${_c}")
            endforeach()
            set(_comp_config "${_comp_config}]")
        endif()

        set(_vcpkg_name "${NAME}${_comp_config}:${_triplet}")
        set(_vcpkg_configs "${NAME}${_comp_config}:${_triplet}")

        # Check if already installed
        execute_process(
            COMMAND "${_vcpkg_exe}" list "${_vcpkg_name}"
            WORKING_DIRECTORY "${_vcpkg_root}"
            OUTPUT_VARIABLE _list_out RESULT_VARIABLE _list_rc)
        string(FIND "${_list_out}" "${_vcpkg_name} " _found_pos)

        if(_found_pos EQUAL -1)
            message(STATUS "vcpkg install ${_vcpkg_configs} ...")
            execute_process(
                COMMAND "${_vcpkg_exe}" install "${_vcpkg_configs}" --recurse
                WORKING_DIRECTORY "${_vcpkg_root}"
                RESULT_VARIABLE _install_rc)
            if(NOT _install_rc EQUAL 0)
                if(arg_QUIET)
                    message(WARNING "${_vcpkg_configs} install failed")
                    return()
                else()
                    message(FATAL_ERROR "${_vcpkg_configs} install failed")
                endif()
            endif()
        endif()

        # Export to build directory for offline reuse
        message(STATUS "Exporting ${NAME}:${_triplet} → ${arg_OUTPUT_DIR}")
        execute_process(
            COMMAND "${_vcpkg_exe}" export "${NAME}:${_triplet}"
                --raw --output=${arg_PACK_NAME} --output-dir=${arg_OUTPUT_DIR}
            WORKING_DIRECTORY "${_vcpkg_root}"
            RESULT_VARIABLE _export_rc)
        if(NOT _export_rc EQUAL 0)
            message(FATAL_ERROR "${NAME} export failed")
        endif()
    endif()

    # Create wrapper target
    add_library(${arg_TARGET} INTERFACE IMPORTED GLOBAL)

    if(NOT arg_NOT_IMPORT)
        message(STATUS "Importing ${arg_PACK_NAME} ...")
        set(_share "${${arg_PREFIX}_INSTALL_DIR}/share/${NAME}")
        if(EXISTS "${_share}/${NAME}Config.cmake")
            set(CMAKE_MODULE_PATH_BAK "${CMAKE_MODULE_PATH}")
            list(APPEND CMAKE_MODULE_PATH "${_share}")
            set(${NAME}_DIR "${_share}")
            find_package(${NAME} REQUIRED)
            if(TARGET "${NAME}::${NAME}")
                target_link_libraries(${arg_TARGET} INTERFACE "${NAME}::${NAME}")
            endif()
            set(CMAKE_MODULE_PATH "${CMAKE_MODULE_PATH_BAK}")
        endif()
        target_link_libraries(${arg_TARGET} INTERFACE ${arg_IMPORTED_TARGETS})
        target_include_directories(${arg_TARGET} INTERFACE "${${arg_PREFIX}_INSTALL_DIR}/include")
    endif()

    set(${arg_PREFIX}_INSTALLED ON CACHE INTERNAL "" FORCE)
    set(${arg_PREFIX}_FOUND ON PARENT_SCOPE)
endfunction()
