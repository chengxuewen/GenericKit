#pragma once

// gkit_media.h must be included before this header
extern "C" {
    void* gkit_media_video_source_create_generator(uint32_t w, uint32_t h, uint32_t fps);
    void  gkit_media_video_source_destroy(void* handle);
    int   gkit_media_video_source_start(void* handle);
    void  gkit_media_video_source_stop(void* handle);
    int   gkit_media_video_source_is_running(void* handle);
}

#include <cstdint>
#include <functional>
#include <memory>

namespace gkit {

class VideoSource {
public:
    static VideoSource createGenerator(uint32_t w, uint32_t h, uint32_t fps) {
        void* raw = gkit_media_video_source_create_generator(w, h, fps);
        return VideoSource(raw);
    }

    ~VideoSource() {
        if (handle_) gkit_media_video_source_destroy(handle_);
    }

    VideoSource(VideoSource&& other) noexcept : handle_(other.handle_) {
        other.handle_ = nullptr;
    }

    VideoSource& operator=(VideoSource&& other) noexcept {
        if (this != &other) {
            if (handle_) gkit_media_video_source_destroy(handle_);
            handle_ = other.handle_;
            other.handle_ = nullptr;
        }
        return *this;
    }

    VideoSource(const VideoSource&) = delete;
    VideoSource& operator=(const VideoSource&) = delete;

    int start() { return gkit_media_video_source_start(handle_); }
    void stop() { gkit_media_video_source_stop(handle_); }
    bool isRunning() const { return gkit_media_video_source_is_running(handle_) != 0; }
    bool valid() const { return handle_ != nullptr; }

private:
    explicit VideoSource(void* h) : handle_(h) {}
    void* handle_ = nullptr;
};

} // namespace gkit
