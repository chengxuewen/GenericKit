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

if(TARGET GKitWrapSDL3::WrapSDL3)
    set(GKitWrapSDL3_FOUND ON)
    return()
endif()

include(InstallVcpkg)
gkit_vcpkg_install_package(sdl3
    NOT_IMPORT
    TARGET 
    GKitWrapSDL3::WrapSDL3
    PREFIX 
    GKitWrapSDL3)
set(CMAKE_PREFIX_PATH_BACKUP ${CMAKE_PREFIX_PATH})
set(CMAKE_PREFIX_PATH ${GKitWrapSDL3_INSTALL_DIR})
find_package(SDL3 PATHS ${GKitWrapSDL3_INSTALL_DIR} NO_DEFAULT_PATH REQUIRED)
target_link_libraries(GKitWrapSDL3::WrapSDL3 INTERFACE SDL3::SDL3)
set(CMAKE_PREFIX_PATH ${CMAKE_PREFIX_PATH_BACKUP})
set(GKitWrapSDL3_FOUND ON)
