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

# C language unit tests for the gkit-media-c FFI library
# Uses Unity test framework (GKitWrapUnity::WrapUnity)

set(_test_dir "${CMAKE_CURRENT_SOURCE_DIR}")

gkit_find_package(WrapUnity PROVIDED_TARGETS GKitWrapUnity::WrapUnity)

# test_basic — PeerConnection create/destroy lifecycle
add_executable(gkit-media-c-test-basic "${_test_dir}/test_basic.c")
target_link_libraries(gkit-media-c-test-basic gkit-media-c GKitWrapUnity::WrapUnity)
set_target_properties(gkit-media-c-test-basic PROPERTIES FOLDER "gkit-media-ffi/tests")
add_test(NAME gkit-media-c-test-basic COMMAND gkit-media-c-test-basic)

# test_sdp — SDP offer/answer exchange + ICE
add_executable(gkit-media-c-test-sdp "${_test_dir}/test_sdp.c")
target_link_libraries(gkit-media-c-test-sdp gkit-media-c GKitWrapUnity::WrapUnity)
set_target_properties(gkit-media-c-test-sdp PROPERTIES FOLDER "gkit-media-ffi/tests")
add_test(NAME gkit-media-c-test-sdp COMMAND gkit-media-c-test-sdp)

# test_data_channel — DataChannel label/send/close/state
add_executable(gkit-media-c-test-dc "${_test_dir}/test_data_channel.c")
target_link_libraries(gkit-media-c-test-dc gkit-media-c GKitWrapUnity::WrapUnity)
set_target_properties(gkit-media-c-test-dc PROPERTIES FOLDER "gkit-media-ffi/tests")
add_test(NAME gkit-media-c-test-dc COMMAND gkit-media-c-test-dc)

# test_errors — null handles, closed peer error codes
add_executable(gkit-media-c-test-errors "${_test_dir}/test_errors.c")
target_link_libraries(gkit-media-c-test-errors gkit-media-c GKitWrapUnity::WrapUnity)
set_target_properties(gkit-media-c-test-errors PROPERTIES FOLDER "gkit-media-ffi/tests")
add_test(NAME gkit-media-c-test-errors COMMAND gkit-media-c-test-errors)

# test_video_frame — VideoFrame create/destroy, scale/crop/rotate
add_executable(gkit-media-c-test-video-frame "${_test_dir}/test_video_frame.c")
target_link_libraries(gkit-media-c-test-video-frame gkit-media-c GKitWrapUnity::WrapUnity)
set_target_properties(gkit-media-c-test-video-frame PROPERTIES FOLDER "gkit-media-ffi/tests")
add_test(NAME gkit-media-c-test-video-frame COMMAND gkit-media-c-test-video-frame)

# test_rtc_factory — Factory create/destroy, backend names, registered backends
add_executable(gkit-media-c-test-rtc-factory "${_test_dir}/test_rtc_factory.c")
target_link_libraries(gkit-media-c-test-rtc-factory gkit-media-c GKitWrapUnity::WrapUnity)
set_target_properties(gkit-media-c-test-rtc-factory PROPERTIES FOLDER "gkit-media-ffi/tests")
add_test(NAME gkit-media-c-test-rtc-factory COMMAND gkit-media-c-test-rtc-factory)
