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
add_executable(gkit_media_c_test_basic "${_test_dir}/test_basic.c")
target_link_libraries(gkit_media_c_test_basic gkit_media_c GKitWrapUnity::WrapUnity)
set_target_properties(gkit_media_c_test_basic PROPERTIES FOLDER "gkit_media/ffi/c/tests")
add_test(NAME gkit_media_c_test_basic COMMAND gkit_media_c_test_basic)

# test_sdp — SDP offer/answer exchange + ICE
add_executable(gkit_media_c_test_sdp "${_test_dir}/test_sdp.c")
target_link_libraries(gkit_media_c_test_sdp gkit_media_c GKitWrapUnity::WrapUnity)
set_target_properties(gkit_media_c_test_sdp PROPERTIES FOLDER "gkit_media/ffi/c/tests")
add_test(NAME gkit_media_c_test_sdp COMMAND gkit_media_c_test_sdp)

# test_data_channel — DataChannel label/send/close/state
add_executable(gkit_media_c_test_dc "${_test_dir}/test_data_channel.c")
target_link_libraries(gkit_media_c_test_dc gkit_media_c GKitWrapUnity::WrapUnity)
set_target_properties(gkit_media_c_test_dc PROPERTIES FOLDER "gkit_media/ffi/c/tests")
add_test(NAME gkit_media_c_test_dc COMMAND gkit_media_c_test_dc)

# test_errors — null handles, closed peer error codes
add_executable(gkit_media_c_test_errors "${_test_dir}/test_errors.c")
target_link_libraries(gkit_media_c_test_errors gkit_media_c GKitWrapUnity::WrapUnity)
set_target_properties(gkit_media_c_test_errors PROPERTIES FOLDER "gkit_media/ffi/c/tests")
add_test(NAME gkit_media_c_test_errors COMMAND gkit_media_c_test_errors)

# test_video_frame — VideoFrame create/destroy, scale/crop/rotate
add_executable(gkit_media_c_test_video_frame "${_test_dir}/test_video_frame.c")
target_link_libraries(gkit_media_c_test_video_frame gkit_media_c GKitWrapUnity::WrapUnity)
set_target_properties(gkit_media_c_test_video_frame PROPERTIES FOLDER "gkit_media/ffi/c/tests")
add_test(NAME gkit_media_c_test_video_frame COMMAND gkit_media_c_test_video_frame)

# test_rtc_factory — Factory create/destroy, backend names, registered backends
add_executable(gkit_media_c_test_rtc_factory "${_test_dir}/test_rtc_factory.c")
target_link_libraries(gkit_media_c_test_rtc_factory gkit_media_c GKitWrapUnity::WrapUnity)
set_target_properties(gkit_media_c_test_rtc_factory PROPERTIES FOLDER "gkit_media/ffi/c/tests")
add_test(NAME gkit_media_c_test_rtc_factory COMMAND gkit_media_c_test_rtc_factory)
