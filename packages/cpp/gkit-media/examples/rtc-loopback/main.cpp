/***********************************************************************************************************************
** gkit-media-cpp WebRTC Loopback Demo (ImGui + SDL3 + OpenGL3)
**
** Simulated two-PeerConnection pipeline:
**   PC1 (sender): VideoFrameGenerator → I420 → RGBA → display
**   PC2 (receiver): frame copy → RGBA → display
** ImGui side-by-side comparison view.
**
** In production, the simulated queue is replaced by real SDP + ICE exchange.
**
** Usage: cmake --build build-auto --target gkit_media_cpp_example_rtc_loopback
***********************************************************************************************************************/

#include <SDL3/SDL.h>
#ifdef __APPLE__
#define GL_SILENCE_DEPRECATION
#include <OpenGL/gl3.h>
#else
#include <SDL3/SDL_opengl.h>
#endif

#include "imgui.h"
#include "imgui_impl_sdl3.h"
#include "imgui_impl_opengl3.h"

#include "gkit_media_video_frame.hpp"
#include "gkit_media_source.hpp"

#include <cstdio>
#include <cstdlib>
#include <mutex>
#include <vector>
#include <string>

static constexpr int kWidth  = 1280;
static constexpr int kHeight = 720;
static constexpr int kFps    = 30;

/* ============================================================================
 * Frame storage (shared between callback thread and render thread)
 * ============================================================================ */

struct Pipeline {
    std::mutex mtx;
    std::vector<uint8_t> generatedRgba;
    std::vector<uint8_t> receivedRgba;
    uint64_t genCount = 0;
    uint64_t recvCount = 0;
    bool hasGen = false;
    bool hasRecv = false;
};

static Pipeline g_pipeline;

extern "C" void frame_callback(void* frame_handle, void* user_data) {
    if (!frame_handle) return;
    int w = (int)gkit_media_video_frame_get_width(frame_handle);
    int h = (int)gkit_media_video_frame_get_height(frame_handle);

    std::vector<uint8_t> rgba(w * h * 4);
    int ok = gkit_media_video_frame_i420_to_argb(frame_handle, rgba.data(), (uint32_t)(w * 4), 3);
    if (ok == 0) {
        std::lock_guard<std::mutex> lk(g_pipeline.mtx);
        g_pipeline.generatedRgba = std::move(rgba);
        g_pipeline.receivedRgba = g_pipeline.generatedRgba; // simulated loopback
        g_pipeline.hasGen = true;
        g_pipeline.hasRecv = true;
        g_pipeline.genCount++;
        g_pipeline.recvCount++;
    }
    gkit_media_video_frame_destroy(frame_handle);
}

/* ============================================================================
 * GL texture helper
 * ============================================================================ */

static GLuint rgba_to_texture(const uint8_t* rgba, int w, int h) {
    GLuint tex = 0; glGenTextures(1, &tex);
    glBindTexture(GL_TEXTURE_2D, tex);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
    glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA, w, h, 0, GL_RGBA, GL_UNSIGNED_BYTE, rgba);
    return tex;
}

static void free_texture(GLuint tex) { glDeleteTextures(1, &tex); }

/* ============================================================================
 * Main application
 * ============================================================================ */

int main() {
    // --- SDL3 + OpenGL init ---
    SDL_Init(SDL_INIT_VIDEO);
    SDL_GL_SetAttribute(SDL_GL_CONTEXT_MAJOR_VERSION, 3);
    SDL_GL_SetAttribute(SDL_GL_CONTEXT_MINOR_VERSION, 2);
    SDL_GL_SetAttribute(SDL_GL_CONTEXT_PROFILE_MASK, SDL_GL_CONTEXT_PROFILE_CORE);
    SDL_GL_SetAttribute(SDL_GL_DOUBLEBUFFER, 1);

    SDL_Window* win = SDL_CreateWindow("gkit-media C++ WebRTC Loopback (ImGui + SDL3)",
        1600, 600, SDL_WINDOW_OPENGL | SDL_WINDOW_RESIZABLE);
    if (!win) { fprintf(stderr, "Window: %s\n", SDL_GetError()); SDL_Quit(); return 1; }
    SDL_GLContext gl_ctx = SDL_GL_CreateContext(win);
    SDL_GL_SetSwapInterval(1);

    IMGUI_CHECKVERSION();
    ImGui::CreateContext();
    ImGui::StyleColorsDark();
    ImGui_ImplSDL3_InitForOpenGL(win, gl_ctx);
    ImGui_ImplOpenGL3_Init("#version 150");

    // --- Create VideoSource generator ---
    void* src = gkit_media_video_source_create_generator(kWidth, kHeight, kFps);
    gkit_media_video_source_set_frame_callback(src, frame_callback, nullptr);
    gkit_media_video_source_start(src);

    bool running = true;
    GLuint genTex = 0, recvTex = 0;
    bool genTexValid = false, recvTexValid = false;

    while (running) {
        SDL_Event evt;
        while (SDL_PollEvent(&evt)) {
            ImGui_ImplSDL3_ProcessEvent(&evt);
            if (evt.type == SDL_EVENT_QUIT) running = false;
        }
        ImGui_ImplOpenGL3_NewFrame();
        ImGui_ImplSDL3_NewFrame();
        ImGui::NewFrame();

        int ww, wh; SDL_GetWindowSize(win, &ww, &wh);

        /* ── Menu bar ── */
        if (ImGui::BeginMainMenuBar()) {
            ImGui::TextUnformatted("gkit-media C++ WebRTC Loopback (simulated)");
            ImGui::SameLine(0, 20);
            ImGui::TextColored(ImVec4(0.3f, 0.9f, 0.3f, 1.0f), "Running");
            ImGui::SameLine(0, 10);
            ImGui::Text("%dx%d  %dfps", kWidth, kHeight, kFps);
            ImGui::EndMainMenuBar();
        }

        /* ── Side-by-side comparison ── */
        ImGui::SetNextWindowPos(ImVec2(0, ImGui::GetFrameHeight()));
        ImGui::SetNextWindowSize(ImVec2((float)ww, (float)(wh - ImGui::GetFrameHeight())));
        ImGui::Begin("##main", nullptr,
            ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoMove);

        float panelW = ImGui::GetContentRegionAvail().x / 2.0f - 4.0f;

        // Upload frames from pipeline
        {
            std::lock_guard<std::mutex> lk(g_pipeline.mtx);
            if (g_pipeline.hasGen) {
                if (genTexValid) free_texture(genTex);
                genTex = rgba_to_texture(g_pipeline.generatedRgba.data(), kWidth, kHeight);
                genTexValid = true;
                g_pipeline.hasGen = false;
            }
            if (g_pipeline.hasRecv) {
                if (recvTexValid) free_texture(recvTex);
                recvTex = rgba_to_texture(g_pipeline.receivedRgba.data(), kWidth, kHeight);
                recvTexValid = true;
                g_pipeline.hasRecv = false;
            }
        }

        // Left: PC1 Sender (Generated)
        ImGui::BeginChild("sender", ImVec2(panelW, 0), ImGuiChildFlags_Borders);
        ImGui::Text("PC1 Sender (Generated)  Frames: %llu", g_pipeline.genCount);
        if (genTexValid) {
            float s = std::min(panelW / (float)kWidth, (ImGui::GetContentRegionAvail().y - 20) / (float)kHeight);
            ImGui::Image((ImTextureID)(intptr_t)genTex, ImVec2(kWidth * s, kHeight * s));
        } else {
            ImGui::TextUnformatted("Waiting for frames...");
        }
        ImGui::EndChild();
        ImGui::SameLine();

        // Right: PC2 Receiver (Received)
        ImGui::BeginChild("receiver", ImVec2(panelW, 0), ImGuiChildFlags_Borders);
        ImGui::Text("PC2 Receiver (Received)   Frames: %llu", g_pipeline.recvCount);
        if (recvTexValid) {
            float s = std::min(panelW / (float)kWidth, (ImGui::GetContentRegionAvail().y - 20) / (float)kHeight);
            ImGui::Image((ImTextureID)(intptr_t)recvTex, ImVec2(kWidth * s, kHeight * s));
        } else {
            ImGui::TextUnformatted("Waiting for frames...");
        }
        ImGui::EndChild();

        ImGui::End();

        /* ── Render ── */
        ImGui::Render();
        glViewport(0, 0, ww, wh);
        glClearColor(0.1f, 0.1f, 0.1f, 1.0f);
        glClear(GL_COLOR_BUFFER_BIT);
        ImGui_ImplOpenGL3_RenderDrawData(ImGui::GetDrawData());
        SDL_GL_SwapWindow(win);
    }

    // Cleanup
    gkit_media_video_source_stop(src);
    gkit_media_video_source_destroy(src);
    if (genTexValid) free_texture(genTex);
    if (recvTexValid) free_texture(recvTex);

    ImGui_ImplOpenGL3_Shutdown();
    ImGui_ImplSDL3_Shutdown();
    ImGui::DestroyContext();
    SDL_GL_DestroyContext(gl_ctx);
    SDL_DestroyWindow(win);
    SDL_Quit();
    return 0;
}
