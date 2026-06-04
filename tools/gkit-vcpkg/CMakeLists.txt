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

set(_tool_name gkit-vcpkg)

add_custom_target(${_tool_name}_build
	COMMAND "${GKIT_CARGO_EXECUTABLE}" build -p ${_tool_name} --release
	WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}"
	COMMENT "Building Rust tool: ${_tool_name}"
	USES_TERMINAL
	VERBATIM)

add_custom_target(${_tool_name}_run
	COMMAND "${GKIT_CARGO_EXECUTABLE}" run -p ${_tool_name} --release -- --help
	WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}"
	COMMENT "Running Rust tool: ${_tool_name}"
	USES_TERMINAL
	VERBATIM)

add_custom_target(${_tool_name}
	COMMENT "Rust tool: ${_tool_name}"
	DEPENDS ${_tool_name}_build
	VERBATIM)

set_target_properties(${_tool_name} ${_tool_name}_build ${_tool_name}_run
	PROPERTIES FOLDER "gkit/tools")
