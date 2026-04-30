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

gkit_find_package(WrapImGui PROVIDED_TARGETS GKitWrapImGui::WrapImGui)
gkit_find_package(WrapSDL3 PROVIDED_TARGETS GKitWrapSDL3::WrapSDL3)
find_package(OpenGL REQUIRED)

set(_example_name gkit_media_cpp_example_viewer)
set(_source_dir "${CMAKE_CURRENT_SOURCE_DIR}")

add_executable(${_example_name}
	"${_source_dir}/viewer.cpp"
)

target_link_libraries(${_example_name}
	gkit_media_cpp
	gkit_media_c
	GKitWrapImGui::WrapImGui
	GKitWrapSDL3::WrapSDL3
	OpenGL::GL
)

target_include_directories(${_example_name} PRIVATE
	"${_source_dir}"
)

set_target_properties(${_example_name} PROPERTIES
	FOLDER "gkit_media/apis/cpp/examples"
	RUNTIME_OUTPUT_DIRECTORY "${GKIT_BUILD_DIR}/${GKIT_INSTALL_BINDIR}"
	CXX_STANDARD 17
)
