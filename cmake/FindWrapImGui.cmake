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

if(TARGET GKitWrapImGui::WrapImGui)
    set(GKitWrapImGui_FOUND ON)
    return()
endif()

include(InstallVcpkg)
gkit_vcpkg_install_package(imgui
    NOT_IMPORT
    TARGET 
    GKitWrapImGui::WrapImGui
    PREFIX 
    GKitWrapImGui
    COMPONENTS 
    sdl3-binding
    opengl3-binding
    docking-experimental)
set(CMAKE_PREFIX_PATH_BACKUP ${CMAKE_PREFIX_PATH})
set(CMAKE_PREFIX_PATH ${GKitWrapImGui_INSTALL_DIR})
find_package(imgui PATHS ${GKitWrapImGui_INSTALL_DIR} NO_DEFAULT_PATH REQUIRED)
target_link_libraries(GKitWrapImGui::WrapImGui INTERFACE imgui::imgui)
set(CMAKE_PREFIX_PATH ${CMAKE_PREFIX_PATH_BACKUP})
set(GKitWrapImGui_FOUND ON)
