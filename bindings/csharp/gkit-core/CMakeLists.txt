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

# generate C# source files via codegen crate (configure time)
set(ENV{CSHARP_OUT_DIR} "${GKIT_CSHARP_SRC_DIR}")
execute_process(
	COMMAND "${GKIT_CARGO_EXECUTABLE}" build -p gkit-core-csharp-codegen
	WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}"
	RESULT_VARIABLE _csbuild_result
	ERROR_VARIABLE _csbuild_err)
unset(ENV{CSHARP_OUT_DIR})
if(NOT _csbuild_result EQUAL 0)
	message(WARNING "Failed to build C# codegen for gkit-core:\n${_csbuild_err}")
endif()

# copy native lib to csharp/lib/<rid>/ (build time)
add_custom_target(gkit_core_csharp ALL
	COMMAND ${CMAKE_COMMAND} -E make_directory "${GKIT_CSHARP_LIB_DIR}"
	COMMAND ${CMAKE_COMMAND} -E copy_if_different
		"$<TARGET_FILE:gkit_core_c-shared>"
		"${GKIT_CSHARP_LIB_DIR}/$<TARGET_FILE_NAME:gkit_core_c-shared>"
	DEPENDS cargo-build_gkit_core_c
	COMMENT "Packaging C# bindings for gkit_core → ${GKIT_CSHARP_SRC_DIR}"
	VERBATIM)

set_target_properties(gkit_core_csharp
	PROPERTIES FOLDER "gkit_core/csharp")
