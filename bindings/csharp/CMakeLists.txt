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

# platform RID for NuGet/lib organization
if(APPLE)
	if(CMAKE_SYSTEM_PROCESSOR MATCHES "arm64|aarch64")
		set(GKIT_CSHARP_RID "osx-arm64")
	else()
		set(GKIT_CSHARP_RID "osx-x64")
	endif()
elseif(WIN32)
	if(CMAKE_SIZEOF_VOID_P EQUAL 8)
		set(GKIT_CSHARP_RID "win-x64")
	else()
		set(GKIT_CSHARP_RID "win-x86")
	endif()
else()
	if(CMAKE_SIZEOF_VOID_P EQUAL 8)
		set(GKIT_CSHARP_RID "linux-x64")
	else()
		set(GKIT_CSHARP_RID "linux-x86")
	endif()
endif()

set(GKIT_CSHARP_SRC_DIR  "${GKIT_BUILD_DIR}/csharp/src")
set(GKIT_CSHARP_LIB_DIR  "${GKIT_BUILD_DIR}/csharp/lib/${GKIT_CSHARP_RID}")

add_subdirectory(gkit-core)
add_subdirectory(gkit-media)
