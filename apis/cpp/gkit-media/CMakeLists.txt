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

gkit_cargo_setup_ffi_cpp_target(gkit_media_cpp
	HEADER_DIR "${CMAKE_CURRENT_SOURCE_DIR}"
	FFI_BASE_NAME "gkit_media_c"
	INSTALL_HEADERS "gkit_media.hpp"
	FOLDER "gkit_media/cpp")

gkit_cargo_install_config(gkit_media_cpp
	DESCRIPTION "GenericKit Media C++ bindings"
	FFI_BASE_NAME "gkit_media_c")
