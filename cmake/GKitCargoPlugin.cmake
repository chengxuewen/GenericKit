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

if(NOT DEFINED _GKIT_CARGO_PLUGIN_LIST)
    set(_GKIT_CARGO_PLUGIN_LIST CACHE INTERNAL "List of plugin crate names")
endif()

#[[.rst:
GenericKit Cargo Plugin Helpers
--------------------------------

.. command:: gkit_cargo_add_plugin

Register a cdylib plugin crate for Corrosion build::

    gkit_cargo_add_plugin(
        NAME <crate-name>         # Cargo package name
        [CATEGORY <category>]     # output subdir: build/plugins/{CATEGORY}/
        [PLATFORMS <platforms>]   # semicolon-separated: macos;linux;windows
    )

Call BEFORE ``corrosion_import_crate`` to collect plugin names.
Then call ``gkit_cargo_setup_plugins()`` AFTER ``corrosion_import_crate``
to set up post-build copy and IDE folder properties.

Example::

    gkit_cargo_add_plugin(
        NAME gkit-plugin-webrtc-libwebrtc
        CATEGORY webrtc
        PLATFORMS macos;linux;windows
    )
#]]

function(gkit_cargo_add_plugin)
    set(_options)
    set(_onevalue NAME CATEGORY)
    set(_multi PLATFORMS)
    cmake_parse_arguments(_arg "${_options}" "${_onevalue}" "${_multi}" ${ARGN})

    if(NOT _arg_NAME)
        message(FATAL_ERROR "gkit_cargo_add_plugin: NAME is required")
    endif()

    if(GKIT_FEATURE_MEDIA_WEBRTC_BACKEND STREQUAL "wasm")
        return()
    endif()

    if(_arg_PLATFORMS)
        list(FIND _arg_PLATFORMS "all" _all_idx)
        if(_all_idx EQUAL -1)
            set(_matched FALSE)
            if(APPLE AND "macos" IN_LIST _arg_PLATFORMS)
                set(_matched TRUE)
            elseif(UNIX AND NOT APPLE AND "linux" IN_LIST _arg_PLATFORMS)
                set(_matched TRUE)
            elseif(WIN32 AND "windows" IN_LIST _arg_PLATFORMS)
                set(_matched TRUE)
            endif()
            if(NOT _matched)
                return()
            endif()
        endif()
    endif()

    if(NOT _arg_CATEGORY)
        set(_arg_CATEGORY "misc")
    endif()

    list(APPEND _GKIT_CARGO_PLUGIN_LIST ${_arg_NAME})
    set(_GKIT_CARGO_PLUGIN_LIST "${_GKIT_CARGO_PLUGIN_LIST}" CACHE INTERNAL "")
    set(_GKIT_CARGO_PLUGIN_LIST "${_GKIT_CARGO_PLUGIN_LIST}" CACHE INTERNAL "Plugin crate names")

    set(_prop_name "GKIT_PLUGIN_${_arg_NAME}_CATEGORY")
    set(${_prop_name} "${_arg_CATEGORY}" CACHE INTERNAL "Plugin category for ${_arg_NAME}")
endfunction()

#[[.rst:
.. command:: gkit_cargo_setup_plugins

Call AFTER ``corrosion_import_crate``. Iterates over all registered plugins
and sets up post-build dylib copy to ``${GKIT_BUILD_DIR}/plugins/<category>/``.
Also sets IDE FOLDER property.
#]]
function(gkit_cargo_setup_plugins)
    foreach(_plugin ${_GKIT_CARGO_PLUGIN_LIST})
        set(_prop_name "GKIT_PLUGIN_${_plugin}_CATEGORY")
        set(_category "${${_prop_name}}")
        if(NOT _category)
            set(_category "misc")
        endif()

        string(REPLACE "-" "_" _target ${_plugin})

        if(NOT TARGET ${_target})
            continue()
        endif()

        set_target_properties(${_target} PROPERTIES
            FOLDER "gkit_media/plugins/${_category}")

        foreach(_prefix cargo-build_ _cargo-build_ cargo-clean_ _cargo-clean_
                      cargo-check_ _cargo-check_ cargo-test_ _cargo-test_
                      cargo-clippy_ _cargo-clippy_ cargo-prebuild_ _cargo-prebuild_)
            set(_util_target ${_prefix}${_target})
            if(TARGET ${_util_target})
                set_target_properties(${_util_target} PROPERTIES
                    FOLDER "gkit_media/plugins/${_category}")
            endif()
        endforeach()

        set(_build_target cargo-build_${_target})
        if(NOT TARGET ${_build_target})
            set(_build_target _cargo-build_${_target})
        endif()
        if(NOT TARGET ${_build_target})
            continue()
        endif()

        if(NOT TARGET copy-plugin-${_plugin})
            add_custom_target(copy-plugin-${_plugin} ALL
                COMMAND ${CMAKE_COMMAND} -E make_directory
                    "${GKIT_BUILD_DIR}/plugins/${_category}"
                COMMAND ${CMAKE_COMMAND} -E copy_if_different
                    "$<TARGET_FILE:${_target}-shared>"
                    "${GKIT_BUILD_DIR}/plugins/${_category}/"
                DEPENDS ${_build_target}
                COMMENT "Plugin ${_target} → plugins/${_category}/"
                VERBATIM)
        endif()

        set_target_properties(copy-plugin-${_plugin} PROPERTIES
            FOLDER "gkit_media/plugins/${_category}")

        install(FILES "$<TARGET_FILE:${_target}-shared>"
            DESTINATION "${CMAKE_INSTALL_LIBDIR}/plugins/${_category}")
    endforeach()
endfunction()
