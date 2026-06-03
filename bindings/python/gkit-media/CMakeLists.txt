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
set(_whl_out_dir "${GKIT_BUILD_DIR}/dist")

add_custom_target(gkit_media_py_build ALL
	COMMAND "${GKIT_MATURIN_EXECUTABLE}" build --release --out "${_whl_out_dir}"
	WORKING_DIRECTORY "${_crate_dir}"
	BYPRODUCTS "${_whl_out_dir}/.gkit_media_py_build.stamp"
	COMMENT "Building python wheel for gkit_media → ${_whl_out_dir}"
	USES_TERMINAL
	VERBATIM)

add_custom_target(gkit_media_py_develop
	COMMAND "${GKIT_MATURIN_EXECUTABLE}" develop
	WORKING_DIRECTORY "${_crate_dir}"
	COMMENT "Installing gkit_media to Python environment (develop mode)"
	USES_TERMINAL
	VERBATIM)

set_target_properties(gkit_media_py_build gkit_media_py_develop
	PROPERTIES FOLDER "gkit_media/python")
