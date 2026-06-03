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

set(_node_out_dir "${GKIT_BUILD_DIR}/node")

add_custom_target(gkit_core_node_build ALL
	COMMAND ${CMAKE_COMMAND} -E make_directory "${_node_out_dir}"
	COMMAND ${CMAKE_COMMAND} -E copy_if_different
		"$<TARGET_FILE:gkit_core_node-shared>"
		"${_node_out_dir}/gkit_core_node.node"
	DEPENDS cargo-build_gkit_core_node
	BYPRODUCTS "${_node_out_dir}/.gkit_core_node_build.stamp"
	COMMENT "Packaging node binding gkit_core → ${_node_out_dir}/gkit_core_node.node"
	VERBATIM)

set_target_properties(gkit_core_node_build
	PROPERTIES FOLDER "gkit_core/node")
