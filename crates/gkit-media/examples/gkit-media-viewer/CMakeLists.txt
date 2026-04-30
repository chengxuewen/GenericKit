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

find_program(GKIT_CARGO_EXECUTABLE cargo REQUIRED)

set(_example_name gkit-media-viewer)
set(_example_bin_dir "${GKIT_BUILD_DIR}/examples")
set(_assets_dir "${GKIT_BUILD_DIR}/assets")

# --- Copy assets to build directory ---
add_custom_target(gkit_examples_assets
	COMMAND ${CMAKE_COMMAND} -E copy_directory
		"${CMAKE_SOURCE_DIR}/assets"
		"${_assets_dir}"
	COMMENT "Copying assets to ${_assets_dir}"
	VERBATIM)

# --- Build example ---
add_custom_target(${_example_name}_build
	COMMAND "${GKIT_CARGO_EXECUTABLE}" build -p gkit-media --example ${_example_name} --release
	WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}"
	COMMENT "Building Rust example: ${_example_name}"
	USES_TERMINAL
	VERBATIM)

# --- Post-build: copy binary to build/examples/ ---
# Cargo outputs to target/release/examples/<name>
if(WIN32)
	set(_cargo_bin "${CMAKE_SOURCE_DIR}/target/release/examples/${_example_name}.exe")
	set(_output_bin "${_example_bin_dir}/${_example_name}.exe")
else()
	set(_cargo_bin "${CMAKE_SOURCE_DIR}/target/release/examples/${_example_name}")
	set(_output_bin "${_example_bin_dir}/${_example_name}")
endif()

add_custom_command(TARGET ${_example_name}_build POST_BUILD
	COMMAND ${CMAKE_COMMAND} -E make_directory "${_example_bin_dir}"
	COMMAND ${CMAKE_COMMAND} -E copy_if_different "${_cargo_bin}" "${_output_bin}"
	COMMENT "Copying ${_example_name} → ${_example_bin_dir}"
	VERBATIM)

# --- Run example (from build/examples/ with ../assets/ relative path) ---
add_custom_target(${_example_name}_run
	DEPENDS gkit_examples_assets ${_example_name}_build
	COMMAND "${_output_bin}"
	WORKING_DIRECTORY "${_example_bin_dir}"
	COMMENT "Running ${_example_name}"
	USES_TERMINAL
	VERBATIM)

# --- Aggregate target ---
add_custom_target(${_example_name} ALL
	COMMENT "Rust example: ${_example_name}"
	DEPENDS gkit_examples_assets ${_example_name}_build
	VERBATIM)

set_target_properties(${_example_name} ${_example_name}_build ${_example_name}_run gkit_examples_assets
	PROPERTIES FOLDER "gkit_media/examples")
