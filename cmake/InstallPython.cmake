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
if(TARGET GKitPython::Python)
    set(GKitPython_FOUND ON)
    return()
endif()

if(WIN32)
    set(GKitPython_NAME "python-3.8.10-embed-win32")
    set(GKitPython_PKG_NAME "${GKitPython_NAME}.zip")
    set(GKitPython_URL_PATH "${PROJECT_SOURCE_DIR}/3rdparty/${GKitPython_PKG_NAME}")
    set(GKitPython_ROOT_DIR "${PROJECT_BINARY_DIR}/3rdparty/${GKitPython_NAME}")
    set(GKitPython_SOURCE_DIR "${GKitPython_ROOT_DIR}/source")
    set(GKitPython_INSTALL_DIR "${GKitPython_ROOT_DIR}/install")
    octk_stamp_file_info(GKitPython OUTPUT_DIR "${GKitPython_ROOT_DIR}")
    octk_fetch_3rdparty(GKitPython URL "${GKitPython_URL_PATH}" OUTPUT_NAME "${GKitPython_NAME}")
    set(GKitPython_EXECUTABLE "${GKitPython_SOURCE_DIR}/python.exe" CACHE INTERNAL "python executable path." FORCE)
else()
    message(STATUS "Find Python interpreter...")
    find_package(Python 3.8 REQUIRED COMPONENTS Interpreter)
    set(GKitPython_EXECUTABLE "${Python_EXECUTABLE}" CACHE INTERNAL "python executable path." FORCE)
endif()
set(OCTKMeson_FILE "${PROJECT_SOURCE_DIR}/3rdparty/meson.pyz" CACHE INTERNAL "meson pyz file path." FORCE)
set(GKitPython_FOUND ON)