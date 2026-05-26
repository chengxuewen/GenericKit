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

if(DEFINED _GKIT_CARGO_EXAMPLE_LIST)
	return()
endif()
set(_GKIT_CARGO_EXAMPLE_LIST)

#[[.rst:
GenericKit Cargo Example Helpers
--------------------------------

.. command:: gkit_cargo_add_example

Create standard build / run / aggregate CMake targets for a Rust example::

    gkit_cargo_add_example(
        NAME <name>            # example name (also cargo --example value)
        CRATE <crate>          # Rust crate name (e.g., gkit-media)
        [ASSETS <dir>]         # optional: source/assets directory to copy
        [FOLDER <folder>]      # IDE FOLDER property
        [RELEASE]              # build with --release
        [FEATURES <features>]  # optional: cargo --features flag
        [REQUIRED_CARGO]       # find_program cargo REQUIRED (default ON)
    )

Creates targets:
  - ``<name>_build``: ``cargo build -p <crate> --example <name> [--release]``
  - ``<name>_run`` : depends _build + assets, executes binary
  - ``<name>`` (ALL): aggregate, depends _build + assets
  - ``gkit_examples_assets`` (if ASSETS supplied): copies assets to build dir

Post-build copies binary from cargo target dir to ``${GKIT_BUILD_DIR}/examples/``.

Assets target ID is derived from NAME: ``gkit_examples_<name>_assets`` for per-example
assets, or ``gkit_examples_assets`` when no NAME to share between examples.

Example::

    gkit_cargo_add_example(
        NAME gkit-media-square-gen
        CRATE gkit-media
        FOLDER gkit_media/examples
    )

    gkit_cargo_add_example(
        NAME gkit-media-viewer
        CRATE gkit-media
        ASSETS "${CMAKE_SOURCE_DIR}/assets"
        FOLDER gkit_media/examples
    )
#]]

function(_gkit_cargo_example_platform_bin name build_type out_cargo out_output)
	if(${build_type} STREQUAL "release")
		set(_dir "release")
	else()
		set(_dir "debug")
	endif()
	if(WIN32)
		set(${out_cargo} "${CMAKE_SOURCE_DIR}/target/${_dir}/examples/${name}.exe" PARENT_SCOPE)
		set(${out_output} "${GKIT_BUILD_DIR}/examples/${name}.exe" PARENT_SCOPE)
	else()
		set(${out_cargo} "${CMAKE_SOURCE_DIR}/target/${_dir}/examples/${name}" PARENT_SCOPE)
		set(${out_output} "${GKIT_BUILD_DIR}/examples/${name}" PARENT_SCOPE)
	endif()
endfunction()

function(gkit_cargo_add_example)
	set(_options RELEASE REQUIRED_CARGO)
	set(_onevalue NAME CRATE ASSETS FOLDER FEATURES)
	set(_multi)
	cmake_parse_arguments(_arg "${_options}" "${_onevalue}" "${_multi}" ${ARGN})

	if(NOT _arg_NAME)
		message(FATAL_ERROR "gkit_cargo_add_example: NAME is required")
	endif()
	if(NOT _arg_CRATE)
		message(FATAL_ERROR "gkit_cargo_add_example: CRATE is required")
	endif()

	set(_example_name ${_arg_NAME})

	# --- Cargo executable ---
	if(NOT DEFINED GKIT_CARGO_EXECUTABLE)
		set(_cargo_required REQUIRED)
		if(DEFINED _arg_REQUIRED_CARGO AND NOT _arg_REQUIRED_CARGO)
			set(_cargo_required)
		endif()
		find_program(GKIT_CARGO_EXECUTABLE cargo ${_cargo_required})
		set(GKIT_CARGO_EXECUTABLE "${GKIT_CARGO_EXECUTABLE}" CACHE FILEPATH "Cargo executable")
	endif()

	# --- Build flags ---
	if(_arg_FEATURES)
		set(_cargo_features "--features" "${_arg_FEATURES}")
	else()
		set(_cargo_features)
	endif()
	if(_arg_RELEASE)
		set(_cargo_flags --release)
		set(_build_type release)
	else()
		set(_build_type debug)
	endif()

	# --- Assets (optional) ---
	if(_arg_ASSETS)
		set(_assets_target gkit_examples_${_example_name}_assets)
		set(_assets_dir "${GKIT_BUILD_DIR}/assets/${_example_name}")
		if(NOT TARGET ${_assets_target})
			add_custom_target(${_assets_target}
				COMMAND ${CMAKE_COMMAND} -E copy_directory
					"${_arg_ASSETS}" "${_assets_dir}"
				COMMENT "Copying assets for ${_example_name} → ${_assets_dir}"
				VERBATIM)
		endif()
		set(_assets_dep ${_assets_target})
	endif()

	# --- Platform binary paths ---
	_gkit_cargo_example_platform_bin(${_example_name} ${_build_type} _cargo_bin _output_bin)

	# --- Build target ---
	add_custom_target(${_example_name}_build
		COMMAND "${GKIT_CARGO_EXECUTABLE}" build -p ${_arg_CRATE}
			--example ${_example_name} ${_cargo_features} ${_cargo_flags}
		WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}"
		COMMENT "Building Rust example: ${_example_name} (${_build_type})"
		USES_TERMINAL
		VERBATIM)

	# --- Post-build copy ---
	add_custom_command(TARGET ${_example_name}_build POST_BUILD
		COMMAND ${CMAKE_COMMAND} -E make_directory "${GKIT_BUILD_DIR}/examples"
		COMMAND ${CMAKE_COMMAND} -E copy_if_different "${_cargo_bin}" "${_output_bin}"
		COMMENT "Copying ${_example_name} → ${GKIT_BUILD_DIR}/examples/"
		VERBATIM)

	# --- Run target ---
	add_custom_target(${_example_name}_run
		DEPENDS ${_assets_dep} ${_example_name}_build
		COMMAND "${_output_bin}"
		WORKING_DIRECTORY "${GKIT_BUILD_DIR}/examples"
		COMMENT "Running ${_example_name}"
		USES_TERMINAL
		VERBATIM)

	# --- Aggregate ALL target ---
	add_custom_target(${_example_name} ALL
		COMMENT "Rust example: ${_example_name}"
		DEPENDS ${_assets_dep} ${_example_name}_build
		VERBATIM)

	# --- FOLDER ---
	set(_folder ${_arg_FOLDER})
	if(NOT _folder)
		set(_folder "gkit_examples")
	endif()

	set(_targets ${_example_name} ${_example_name}_build ${_example_name}_run)
	if(_assets_dep)
		list(APPEND _targets ${_assets_dep})
	endif()

	set_target_properties(${_targets} PROPERTIES FOLDER "${_folder}")
endfunction()
