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

set(_crate_dir "${CMAKE_CURRENT_SOURCE_DIR}")
set(_wasm_base "${GKIT_BUILD_DIR}/wasm")

# --- bundler target ---
add_custom_target(gkit_core_wasm_build_bundler ALL
	COMMAND "${GKIT_WASM_PACK_EXECUTABLE}" build --release --target bundler --out-dir "${_wasm_base}/bundler"
	WORKING_DIRECTORY "${_crate_dir}"
	BYPRODUCTS "${_wasm_base}/bundler/.gkit_core_wasm_build_bundler.stamp"
	COMMENT "Building wasm package (bundler) for gkit_core → ${_wasm_base}/bundler"
	USES_TERMINAL
	VERBATIM)

# --- web target ---
add_custom_target(gkit_core_wasm_build_web ALL
	COMMAND "${GKIT_WASM_PACK_EXECUTABLE}" build --release --target web --out-dir "${_wasm_base}/web"
	WORKING_DIRECTORY "${_crate_dir}"
	BYPRODUCTS "${_wasm_base}/web/.gkit_core_wasm_build_web.stamp"
	COMMENT "Building wasm package (web) for gkit_core → ${_wasm_base}/web"
	USES_TERMINAL
	VERBATIM)

# --- nodejs target ---
add_custom_target(gkit_core_wasm_build_nodejs ALL
	COMMAND "${GKIT_WASM_PACK_EXECUTABLE}" build --release --target nodejs --out-dir "${_wasm_base}/nodejs"
	WORKING_DIRECTORY "${_crate_dir}"
	BYPRODUCTS "${_wasm_base}/nodejs/.gkit_core_wasm_build_nodejs.stamp"
	COMMENT "Building wasm package (nodejs) for gkit_core → ${_wasm_base}/nodejs"
	USES_TERMINAL
	VERBATIM)

set_target_properties(gkit_core_wasm_build_bundler gkit_core_wasm_build_web gkit_core_wasm_build_nodejs
	PROPERTIES FOLDER "gkit_core/wasm")
