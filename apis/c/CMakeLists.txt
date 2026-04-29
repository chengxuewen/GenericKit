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

gkit_cargo_setup_ffi_target(gkit_core_c
	MANIFEST "${CMAKE_CURRENT_SOURCE_DIR}/gkit-core/Cargo.toml"
	HEADER "gkit_core.h"
	CRATE_NAME "gkit-core-c"
	FOLDER "gkit_core/c")

gkit_cargo_setup_ffi_target(gkit_media_c
	MANIFEST "${CMAKE_CURRENT_SOURCE_DIR}/gkit-media/Cargo.toml"
	HEADER "gkit_media.h"
	CRATE_NAME "gkit-media-c"
	FOLDER "gkit_media/c")

gkit_cargo_install_config(gkit_core_c
	DESCRIPTION "GenericKit Core C FFI bindings"
	LIB_NAME_STATIC "gkit_core_c"
	LIB_NAME_SHARED "gkit_core_c")

gkit_cargo_install_config(gkit_media_c
	DESCRIPTION "GenericKit Media C FFI bindings"
	LIB_NAME_STATIC "gkit_media_c"
	LIB_NAME_SHARED "gkit_media_c")
