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
# Ported and adapted from Qt's QtFindPackageHelpers.cmake
# Original: https://code.qt.io/cgit/qt/qtbase.git/tree/cmake/QtFindPackageHelpers.cmake
# Copyright (C) 2022 The Qt Company Ltd.
# SPDX-License-Identifier: BSD-3-Clause

#-----------------------------------------------------------------------------------------------------------------------
# gkit_find_package — Enhanced find_package with target validation and promotion.
#
# Usage:
#   gkit_find_package(<PackageName> [version]
#       [REQUIRED] [QUIET] [CONFIG] [NO_MODULE] [MODULE]
#       [PROVIDED_TARGETS <target1> [<target2> ...]]
#       [MARK_OPTIONAL]
#       [COMPONENTS <comp1> [<comp2> ...]]
#       [OPTIONAL_COMPONENTS <comp1> [<comp2> ...]]
#   )
#
# Features over stock find_package:
#   1. PROVIDED_TARGETS  — validates that expected targets were actually found.
#      If the config-mode find created the package but didn't create the targets,
#      it unsets the _FOUND var and re-tries in module mode.
#   2. Target promotion   — promotes found targets to global scope so they are
#      accessible in all subdirectories.
#   3. Package tracking   — records which package provides each target in target
#      properties, for later install/config generation.
#   4. MARK_OPTIONAL      — marks the package dependency as optional.
#
# Examples:
#   gkit_find_package(WrapGTest PROVIDED_TARGETS GTest::GTest GTest::Main)
#   gkit_find_package(WrapBenchmark PROVIDED_TARGETS benchmark::benchmark)
#   gkit_find_package(Threads REQUIRED PROVIDED_TARGETS Threads::Threads)
#-----------------------------------------------------------------------------------------------------------------------
macro(gkit_find_package)
    set(_find_package_options CONFIG NO_MODULE MODULE REQUIRED QUIET)
    set(options ${_find_package_options} MARK_OPTIONAL)
    set(oneValueArgs)
    set(multiValueArgs PROVIDED_TARGETS COMPONENTS OPTIONAL_COMPONENTS)
    cmake_parse_arguments(_arg "${options}" "${oneValueArgs}" "${multiValueArgs}" ${ARGN})

    if(_arg_COMPONENTS)
        list(APPEND _arg_UNPARSED_ARGUMENTS "COMPONENTS;${_arg_COMPONENTS}")
    endif()
    if(_arg_OPTIONAL_COMPONENTS)
        list(APPEND _arg_UNPARSED_ARGUMENTS "OPTIONAL_COMPONENTS;${_arg_OPTIONAL_COMPONENTS}")
    endif()

    if(NOT (_arg_CONFIG OR _arg_NO_MODULE OR _arg_MODULE))
        # Try config mode first quietly, then module mode if needed
        set(_config_args ${_arg_UNPARSED_ARGUMENTS})
        list(APPEND _config_args "CONFIG;QUIET")
        find_package(${_config_args})

        set(_should_retry FALSE)
        if(${ARGV0}_FOUND AND _arg_PROVIDED_TARGETS)
            set(_any_target_found FALSE)
            foreach(_tgt ${_arg_PROVIDED_TARGETS})
                if(TARGET ${_tgt})
                    set(_any_target_found TRUE)
                    break()
                endif()
            endforeach()
            if(NOT _any_target_found)
                set(_should_retry TRUE)
            endif()
        endif()

        if(NOT ${ARGV0}_FOUND OR _should_retry)
            unset(${ARGV0}_FOUND)
            unset(${ARGV0}_FOUND CACHE)
            if(DEFINED CACHE{${ARGV0}_DIR} AND NOT ${ARGV0}_DIR)
                unset(${ARGV0}_DIR CACHE)
            endif()
        endif()
    endif()

    # Restore options into unparsed args
    foreach(_opt IN LISTS _find_package_options)
        if(_arg_${_opt})
            list(APPEND _arg_UNPARSED_ARGUMENTS ${_opt})
        endif()
    endforeach()

    if(NOT ${ARGV0}_FOUND)
        find_package(${_arg_UNPARSED_ARGUMENTS})
    endif()

    if(${ARGV0}_FOUND AND _arg_PROVIDED_TARGETS)
        foreach(_tgt ${_arg_PROVIDED_TARGETS})
            if(TARGET ${_tgt})
                # Resolve alias to actual target
                get_target_property(_aliased ${_tgt} ALIASED_TARGET)
                if(_aliased)
                    set(_actual_tgt ${_aliased})
                else()
                    set(_actual_tgt ${_tgt})
                endif()

                # Record package metadata on target
                set_target_properties(${_actual_tgt} PROPERTIES
                    GKIT_PACKAGE_NAME "${ARGV0}"
                    GKIT_PACKAGE_IS_OPTIONAL "${_arg_MARK_OPTIONAL}")

                if(${ARGC} GREATER_EQUAL 2 AND ARGV1 MATCHES "^[0-9\.]+$")
                    set_target_properties(${_actual_tgt} PROPERTIES
                        GKIT_PACKAGE_VERSION "${ARGV1}")
                endif()

                # Promote target and its dependencies to global
                _gkit_promote_target_to_global(${_actual_tgt})
            endif()
        endforeach()
    endif()
endmacro()


#-----------------------------------------------------------------------------------------------------------------------
# Promote a target and its transitive dependencies to global scope.
# This ensures targets are visible across CMake subdirectory boundaries.
#-----------------------------------------------------------------------------------------------------------------------
function(_gkit_promote_target_to_global target)
    if(TARGET ${target})
        get_target_property(_type ${target} TYPE)
        if(_type STREQUAL "INTERFACE_LIBRARY" OR _type STREQUAL "STATIC_LIBRARY"
           OR _type STREQUAL "SHARED_LIBRARY" OR _type STREQUAL "UNKNOWN_LIBRARY")
            get_target_property(_already_global ${target} _gkit_promoted_to_global)
            if(NOT _already_global)
                set_target_properties(${target} PROPERTIES
                    IMPORTED_GLOBAL TRUE
                    _gkit_promoted_to_global TRUE)
            endif()

            # Recurse into dependencies
            get_target_property(_link_libs ${target} INTERFACE_LINK_LIBRARIES)
            if(_link_libs)
                foreach(_dep ${_link_libs})
                    if(TARGET ${_dep})
                        _gkit_promote_target_to_global(${_dep})
                    endif()
                endforeach()
            endif()
        endif()
    endif()
endfunction()


#-----------------------------------------------------------------------------------------------------------------------
# Record an extra third-party dependency for a target.
# Used to ensure find_dependency() calls are generated in the install config.
#
# Usage:
#   gkit_record_extra_package_dependency(<main_target> <dep_package_name> [version])
#-----------------------------------------------------------------------------------------------------------------------
function(gkit_record_extra_package_dependency main_target dep_package_name)
    if(TARGET ${main_target})
        set(version "")
        if(${ARGC} GREATER_EQUAL 3)
            set(version "${ARGV2}")
        endif()

        get_target_property(_deps ${main_target} GKIT_EXTRA_PACKAGE_DEPENDENCIES)
        if(NOT _deps)
            set(_deps "")
        endif()
        list(APPEND _deps "${dep_package_name}\\;${version}")
        set_target_properties(${main_target} PROPERTIES
            GKIT_EXTRA_PACKAGE_DEPENDENCIES "${_deps}")
    endif()
endfunction()
