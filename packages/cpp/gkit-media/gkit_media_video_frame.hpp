/***********************************************************************************************************************
**
** Library: GenericKit
**
** Copyright (C) 2026~Present ChengXueWen.
**
** Licensed under the Apache License, Version 2.0 (the "License");
** you may not use this file except in compliance with the License.
** You may obtain a copy of the License at
**
** http://www.apache.org/licenses/LICENSE-2.0
**
** Unless required by applicable law or agreed to in writing, software
** distributed under the License is distributed on an "AS IS" BASIS,
** WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
** See the License for the specific language governing permissions and
** limitations under the License.
**
***********************************************************************************************************************/

#pragma once

extern "C" {
#include "gkit_media.h"
}

#include <cstdint>
#include <vector>
#include <stdexcept>

namespace gkit {

/// RAII wrapper for VideoFrame opaque handle.
class GKIT_MEDIA_API VideoFrame {
public:
    VideoFrame() : handle_(nullptr) {}

    /// Creates an I420 video frame.
    explicit VideoFrame(uint32_t width, uint32_t height);
    ~VideoFrame();

    VideoFrame(VideoFrame&& other) noexcept;
    VideoFrame& operator=(VideoFrame&& other) noexcept;

    VideoFrame(const VideoFrame&) = delete;
    VideoFrame& operator=(const VideoFrame&) = delete;

    /// Creates an NV12 video frame.
    static VideoFrame create_nv12(uint32_t width, uint32_t height);

    uint32_t width() const;
    uint32_t height() const;

    int rotation() const;
    void set_rotation(int r);

    int64_t timestamp_us() const;
    void set_timestamp_us(int64_t ts);

    int buffer_type() const;

    bool get_i420_planes(std::vector<uint8_t>& out_y, uint32_t& stride_y,
                         std::vector<uint8_t>& out_u, uint32_t& stride_u,
                         std::vector<uint8_t>& out_v, uint32_t& stride_v) const;

    VideoFrame scale(uint32_t w, uint32_t h) const;
    VideoFrame crop(uint32_t x, uint32_t y, uint32_t w, uint32_t h) const;
    VideoFrame rotate(uint32_t degrees) const;

    bool valid() const { return handle_ != nullptr; }

private:
    void release() { if (handle_) { gkit_media_video_frame_destroy(handle_); handle_ = nullptr; } }
    void* handle_;
};

// ============================================================================
// Inline implementations
// ============================================================================

inline VideoFrame::VideoFrame(uint32_t width, uint32_t height)
    : handle_(gkit_media_video_frame_create(width, height))
{
    if (!handle_) throw std::runtime_error("gkit_media_video_frame_create failed");
}

inline VideoFrame::~VideoFrame() { release(); }

inline VideoFrame::VideoFrame(VideoFrame&& other) noexcept : handle_(other.handle_)
{
    other.handle_ = nullptr;
}

inline VideoFrame& VideoFrame::operator=(VideoFrame&& other) noexcept
{
    if (this != &other) { release(); handle_ = other.handle_; other.handle_ = nullptr; }
    return *this;
}

inline VideoFrame VideoFrame::create_nv12(uint32_t width, uint32_t height)
{
    VideoFrame vf;
    vf.handle_ = gkit_media_video_frame_create_nv12(width, height);
    if (!vf.handle_) throw std::runtime_error("gkit_media_video_frame_create_nv12 failed");
    return vf;
}

inline uint32_t VideoFrame::width() const { return gkit_media_video_frame_get_width(handle_); }
inline uint32_t VideoFrame::height() const { return gkit_media_video_frame_get_height(handle_); }

inline int VideoFrame::rotation() const { return gkit_media_video_frame_get_rotation(handle_); }
inline void VideoFrame::set_rotation(int r) { gkit_media_video_frame_set_rotation(handle_, r); }

inline int64_t VideoFrame::timestamp_us() const { return gkit_media_video_frame_get_timestamp(handle_); }
inline void VideoFrame::set_timestamp_us(int64_t ts) { gkit_media_video_frame_set_timestamp(handle_, ts); }

inline int VideoFrame::buffer_type() const { return gkit_media_video_frame_get_buffer_type(handle_); }

inline bool VideoFrame::get_i420_planes(
    std::vector<uint8_t>& out_y, uint32_t& stride_y,
    std::vector<uint8_t>& out_u, uint32_t& stride_u,
    std::vector<uint8_t>& out_v, uint32_t& stride_v) const
{
    uint32_t sy, su, sv;
    out_y.resize(width() * height());
    out_u.resize(((width() + 1) / 2) * ((height() + 1) / 2));
    out_v.resize(((width() + 1) / 2) * ((height() + 1) / 2));
    int rc = gkit_media_video_frame_get_i420_planes(
        handle_, out_y.data(), &sy, out_u.data(), &su, out_v.data(), &sv);
    stride_y = sy; stride_u = su; stride_v = sv;
    return rc == 0;
}

inline VideoFrame VideoFrame::scale(uint32_t w, uint32_t h) const
{
    VideoFrame result;
    result.handle_ = gkit_media_video_frame_scale(handle_, w, h);
    if (!result.handle_) throw std::runtime_error("gkit_media_video_frame_scale failed");
    return result;
}

inline VideoFrame VideoFrame::crop(uint32_t x, uint32_t y, uint32_t w, uint32_t h) const
{
    VideoFrame result;
    result.handle_ = gkit_media_video_frame_crop(handle_, x, y, w, h);
    if (!result.handle_) throw std::runtime_error("gkit_media_video_frame_crop failed");
    return result;
}

inline VideoFrame VideoFrame::rotate(uint32_t degrees) const
{
    VideoFrame result;
    result.handle_ = gkit_media_video_frame_rotate(handle_, degrees);
    if (!result.handle_) throw std::runtime_error("gkit_media_video_frame_rotate failed");
    return result;
}

} // namespace gkit
