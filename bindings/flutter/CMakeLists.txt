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

# require Dart SDK
find_program(GKIT_DART_EXECUTABLE dart)
if(NOT GKIT_DART_EXECUTABLE)
	message(WARNING "Dart SDK not found — Flutter bindings will not be generated. Install with: brew install dart")
	return()
endif()
message(STATUS "Found dart: ${GKIT_DART_EXECUTABLE}")

# auto-install flutter_rust_bridge_codegen if not found
find_program(GKIT_FRB_CODEGEN flutter_rust_bridge_codegen)
if(NOT GKIT_FRB_CODEGEN)
	find_program(GKIT_CARGO_EXECUTABLE cargo REQUIRED)
	message(STATUS "flutter_rust_bridge_codegen not found — installing via cargo install flutter_rust_bridge_codegen...")
	execute_process(
		COMMAND "${GKIT_CARGO_EXECUTABLE}" install flutter_rust_bridge_codegen
		RESULT_VARIABLE _frb_install_result)
	if(NOT _frb_install_result EQUAL 0)
		message(WARNING "Failed to install flutter_rust_bridge_codegen — Flutter bindings will not be built")
		return()
	endif()
	find_program(GKIT_FRB_CODEGEN flutter_rust_bridge_codegen)
	if(NOT GKIT_FRB_CODEGEN)
		message(WARNING "flutter_rust_bridge_codegen not found after install — check your PATH")
		return()
	endif()
endif()
message(STATUS "Found flutter_rust_bridge_codegen: ${GKIT_FRB_CODEGEN}")

# platform RID for lib organization
if(APPLE)
	if(CMAKE_SYSTEM_PROCESSOR MATCHES "arm64|aarch64")
		set(GKIT_FLUTTER_RID "osx-arm64")
	else()
		set(GKIT_FLUTTER_RID "osx-x64")
	endif()
elseif(WIN32)
	if(CMAKE_SIZEOF_VOID_P EQUAL 8)
		set(GKIT_FLUTTER_RID "win-x64")
	else()
		set(GKIT_FLUTTER_RID "win-x86")
	endif()
else()
	if(CMAKE_SIZEOF_VOID_P EQUAL 8)
		set(GKIT_FLUTTER_RID "linux-x64")
	else()
		set(GKIT_FLUTTER_RID "linux-x86")
	endif()
endif()

set(GKIT_FLUTTER_LIB_DIR "${GKIT_BUILD_DIR}/flutter/lib/${GKIT_FLUTTER_RID}")

add_subdirectory(gkit-core)
add_subdirectory(gkit-media)
