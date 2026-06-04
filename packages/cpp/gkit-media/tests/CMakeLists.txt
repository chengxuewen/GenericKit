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

gkit_find_package(WrapGTest PROVIDED_TARGETS GKitWrapGTest::WrapGTest)

set(_test_dir "${CMAKE_CURRENT_SOURCE_DIR}")

add_executable(gkit-media-cpp-test-video-frame "${_test_dir}/test_video_frame.cpp")
target_link_libraries(gkit-media-cpp-test-video-frame gkit-media-cpp gkit-media-c GKitWrapGTest::WrapGTest)
set_target_properties(gkit-media-cpp-test-video-frame PROPERTIES FOLDER "gkit-media-ffi/packages/cpp/tests")
add_test(NAME gkit-media-cpp-test-video-frame COMMAND gkit-media-cpp-test-video-frame)
