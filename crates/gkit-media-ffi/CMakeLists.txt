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

gkit_cargo_setup_ffi_target(gkit_media_c
	MANIFEST "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml"
	HEADER "gkit_media.h"
	CRATE_NAME "gkit-media-c"
	FOLDER "gkit_media/ffi")

# API visibility macro (committed alongside the Rust crate)
target_include_directories(gkit_media_c INTERFACE "${CMAKE_CURRENT_SOURCE_DIR}")

gkit_install(FILES "${CMAKE_CURRENT_SOURCE_DIR}/gkit_media_api.h"
	DESTINATION "${GKIT_INSTALL_INCLUDEDIR}"
	COMPONENT Devel)

gkit_cargo_install_config(gkit_media_c
	DESCRIPTION "GenericKit Media C FFI bindings"
	LIB_NAME_STATIC "gkit_media_c"
	LIB_NAME_SHARED "gkit_media_c")

if(GKIT_BUILD_TESTS)
	add_subdirectory(tests)
endif()

# TODO: re-enable after FindWrapCImGui backend compilation fixed
# if(GKIT_BUILD_EXAMPLES)
# 	add_subdirectory(examples)
# endif()
