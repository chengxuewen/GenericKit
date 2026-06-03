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

# auto-install wasm-pack if not found
find_program(GKIT_WASM_PACK_EXECUTABLE wasm-pack)
if(NOT GKIT_WASM_PACK_EXECUTABLE)
	find_program(GKIT_CARGO_EXECUTABLE cargo REQUIRED)
	message(STATUS "wasm-pack not found — installing via cargo install wasm-pack...")
	execute_process(
		COMMAND "${GKIT_CARGO_EXECUTABLE}" install wasm-pack
		RESULT_VARIABLE _wasm_pack_install_result
		OUTPUT_QUIET)
	if(NOT _wasm_pack_install_result EQUAL 0)
		message(WARNING "Failed to install wasm-pack — WebAssembly bindings will not be built")
		return()
	endif()
	find_program(GKIT_WASM_PACK_EXECUTABLE wasm-pack)
	if(NOT GKIT_WASM_PACK_EXECUTABLE)
		message(WARNING "wasm-pack not found after install — check your PATH")
		return()
	endif()
endif()
message(STATUS "Found wasm-pack: ${GKIT_WASM_PACK_EXECUTABLE}")

# verify wasm32 target is available
find_program(GKIT_RUSTUP_EXECUTABLE rustup)
if(GKIT_RUSTUP_EXECUTABLE)
	execute_process(
		COMMAND "${GKIT_RUSTUP_EXECUTABLE}" target list --installed
		OUTPUT_VARIABLE _rustup_installed_targets
		OUTPUT_STRIP_TRAILING_WHITESPACE)
	if(NOT _rustup_installed_targets MATCHES "wasm32-unknown-unknown")
		message(WARNING "wasm32-unknown-unknown target not installed. Install with: rustup target add wasm32-unknown-unknown")
	endif()
endif()

add_subdirectory(gkit-core)
add_subdirectory(gkit-media)
