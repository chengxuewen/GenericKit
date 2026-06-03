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
set(_dart_out "${GKIT_BUILD_DIR}/flutter/gkit-media/dart")

# create minimal Dart package descriptor for FRB codegen
file(WRITE "${GKIT_BUILD_DIR}/flutter/gkit-media/pubspec.yaml"
	"name: gkit_media_flutter\nversion: 0.1.0\npublish_to: none\n")

# generate Dart glue code at configure time
message(STATUS "Generating Flutter bindings for gkit-media...")
execute_process(
	COMMAND "${GKIT_FRB_CODEGEN}" generate
		--rust-input "bindings/flutter/gkit-media/src/api"
		--rust-root "bindings/flutter/gkit-media"
		--dart-output "${_dart_out}"
		--rust-output "bindings/flutter/gkit-media/src/frb_generated.rs"
	WORKING_DIRECTORY "${CMAKE_SOURCE_DIR}"
	RESULT_VARIABLE _frb_result
	ERROR_VARIABLE _frb_err
	OUTPUT_VARIABLE _frb_out)
if(NOT _frb_result EQUAL 0)
	message(WARNING "flutter_rust_bridge_codegen failed for gkit-media:\n  ${_frb_err}\n  ${_frb_out}")
else()
	message(STATUS "Flutter bindings for gkit-media → ${_dart_out}")
endif()

# copy native lib to flutter/lib/<rid>/ at build time
add_custom_target(gkit_media_flutter_bindings ALL
	COMMAND ${CMAKE_COMMAND} -E make_directory "${GKIT_FLUTTER_LIB_DIR}"
	COMMAND ${CMAKE_COMMAND} -E copy_if_different
		"$<TARGET_FILE:gkit_media_flutter-shared>"
		"${GKIT_FLUTTER_LIB_DIR}/$<TARGET_FILE_NAME:gkit_media_flutter-shared>"
	DEPENDS cargo-build_gkit_media_flutter
	BYPRODUCTS "${_dart_out}"
	COMMENT "Packaging Flutter bindings for gkit_media → ${GKIT_FLUTTER_LIB_DIR}"
	VERBATIM)

set_target_properties(gkit_media_flutter_bindings
	PROPERTIES FOLDER "gkit_media/flutter")
