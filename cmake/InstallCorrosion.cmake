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
# distributed under the License is distributed on an “AS IS” BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
#
########################################################################################################################

# We can't create the same interface imported target multiple times, CMake will complain if we do
# that. This can happen if the find_package call is done in multiple different subdirectories.
if(TARGET GKitWrapCorrosion::WrapCorrosion)
	set(GKitWrapCorrosion_FOUND ON)
	return()
endif()

set(GKitWrapCorrosion_NAME "corrosion-0.6.1")
set(GKitWrapCorrosion_DIR_NAME "${GKitWrapCorrosion_NAME}")
set(GKitWrapCorrosion_PKG_NAME "${GKitWrapCorrosion_NAME}.tar.gz")
set(GKitWrapCorrosion_URL_PATH "${PROJECT_SOURCE_DIR}/3rdparty/${GKitWrapCorrosion_PKG_NAME}")
set(GKitWrapCorrosion_ROOT_DIR "${PROJECT_BINARY_DIR}/3rdparty/${GKitWrapCorrosion_DIR_NAME}")
set(GKitWrapCorrosion_BUILD_DIR "${GKitWrapCorrosion_ROOT_DIR}/build" CACHE INTERNAL "" FORCE)
set(GKitWrapCorrosion_SOURCE_DIR "${GKitWrapCorrosion_ROOT_DIR}/source" CACHE INTERNAL "" FORCE)
set(GKitWrapCorrosion_INSTALL_DIR "${GKitWrapCorrosion_ROOT_DIR}/install" CACHE INTERNAL "" FORCE)
gkit_stamp_file_info(GKitWrapCorrosion OUTPUT_DIR "${GKitWrapCorrosion_ROOT_DIR}")
gkit_fetch_3rdparty(GKitWrapCorrosion URL "${GKitWrapCorrosion_URL_PATH}" OUTPUT_NAME "${GKitWrapCorrosion_DIR_NAME}")
if(NOT EXISTS "${GKitWrapCorrosion_STAMP_FILE_PATH}")
	if(NOT EXISTS ${GKitWrapCorrosion_SOURCE_DIR})
		message(FATAL_ERROR "${GKitWrapCorrosion_NAME} FetchContent failed.")
	endif()
	gkit_reset_dir(${GKitWrapCorrosion_BUILD_DIR})

	message(STATUS "Configure ${GKitWrapCorrosion_NAME} lib...")
	execute_process(
		COMMAND ${CMAKE_COMMAND}
		-G ${CMAKE_GENERATOR}
		-DCMAKE_INSTALL_PREFIX=${GKitWrapCorrosion_INSTALL_DIR}
		${GKitWrapCorrosion_SOURCE_DIR}
		WORKING_DIRECTORY "${GKitWrapCorrosion_BUILD_DIR}"
		RESULT_VARIABLE CONFIGURE_RESULT)
	if(NOT CONFIGURE_RESULT MATCHES 0)
		message(FATAL_ERROR "${GKitWrapCorrosion_NAME} configure failed.")
	endif()
	message(STATUS "${GKitWrapCorrosion_NAME} configure success")

	execute_process(
		COMMAND ${CMAKE_COMMAND} --build ./ --parallel ${OCTK_NUMBER_OF_ASYNC_JOBS} --config Release --target install
		WORKING_DIRECTORY "${GKitWrapCorrosion_BUILD_DIR}"
		RESULT_VARIABLE BUILD_RESULT)
	if(NOT BUILD_RESULT MATCHES 0)
		message(FATAL_ERROR "${GKitWrapCorrosion_NAME} build failed.")
	endif()
	message(STATUS "${GKitWrapCorrosion_NAME} build success")

	execute_process(
		COMMAND ${CMAKE_COMMAND} --install ./
		WORKING_DIRECTORY "${GKitWrapCorrosion_BUILD_DIR}"
		RESULT_VARIABLE INSTALL_RESULT)
	if(NOT INSTALL_RESULT MATCHES 0)
		message(FATAL_ERROR "${GKitWrapCorrosion_NAME} install failed.")
	endif()
	message(STATUS "${GKitWrapCorrosion_NAME} install success")
	gkit_make_stamp_file("${GKitWrapCorrosion_STAMP_FILE_PATH}")
endif()
# wrap lib
find_package(Corrosion PATHS ${GKitWrapCorrosion_INSTALL_DIR} NO_DEFAULT_PATH REQUIRED)
set(GKitWrapCorrosion_FOUND ON)