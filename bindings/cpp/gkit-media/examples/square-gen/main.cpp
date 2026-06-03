/***********************************************************************************************************************
** gkit-media-cpp SquareGenerator Demo (ImGui + SDL3 + OpenGL3)
**
** Uses gkit::VideoSource (C++ RAII) wrapping a VideoFrameGenerator.
** Pipeline: SquarePattern → I420 → RGBA → GL texture → ImGui display
**
** Usage: cmake --build build-auto --target gkit_media_cpp_example_square_gen
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
#include <vector>
#include <mutex>

static constexpr int kWidth  = 640;
static constexpr int kHeight = 480;
static constexpr int kFps    = 30;

/* ============================================================================
 * Frame storage (shared between callback thread and render thread)
 * ============================================================================ */

static std::vector<uint8_t> g_rgba;
static std::mutex g_mtx;
static bool g_has_frame = false;

extern "C" void frame_callback(void* frame_handle, void* /*user_data*/) {
    if (!frame_handle) return;
    int w = (int)gkit_media_video_frame_get_width(frame_handle);
    int h = (int)gkit_media_video_frame_get_height(frame_handle);

    std::vector<uint8_t> rgba(w * h * 4);
    // format 3 = RGBA (0=ARGB, 1=BGRA, 2=ABGR, 3=RGBA)
    int ok = gkit_media_video_frame_i420_to_argb(frame_handle, rgba.data(), (uint32_t)(w * 4), 3);
    if (ok == 0) {
        std::lock_guard<std::mutex> lk(g_mtx);
        g_rgba = std::move(rgba);
        g_has_frame = true;
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

    SDL_Window* win = SDL_CreateWindow("gkit-media C++ SquareGenerator (ImGui + SDL3)",
        1100, 700, SDL_WINDOW_OPENGL | SDL_WINDOW_RESIZABLE);
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
    GLuint tex = 0;
    bool tex_valid = false;

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
            ImGui::TextUnformatted("gkit-media C++ SquareGenerator");
            ImGui::SameLine(0, 20);
            ImGui::TextColored(ImVec4(0.3f, 0.9f, 0.3f, 1.0f), "Running");
            ImGui::SameLine(0, 10);
            ImGui::Text("%dx%d  %dfps", kWidth, kHeight, kFps);
            ImGui::EndMainMenuBar();
        }

        /* ── Main content ── */
        ImGui::SetNextWindowPos(ImVec2(0, ImGui::GetFrameHeight()));
        ImGui::SetNextWindowSize(ImVec2((float)ww, (float)(wh - ImGui::GetFrameHeight())));
        ImGui::Begin("##main", nullptr,
            ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoMove);

        if (tex_valid) {
            float avail_w = ImGui::GetContentRegionAvail().x;
            float avail_h = ImGui::GetContentRegionAvail().y;
            float scale = std::min(avail_w / (float)kWidth, avail_h / (float)kHeight);
            ImGui::Image((ImTextureID)(intptr_t)tex,
                ImVec2((float)kWidth * scale, (float)kHeight * scale));
        } else {
            ImGui::TextUnformatted("Waiting for frames...");
        }

        ImGui::End();

        /* ── Upload latest frame ── */
        {
            std::lock_guard<std::mutex> lk(g_mtx);
            if (g_has_frame) {
                if (tex_valid) free_texture(tex);
                tex = rgba_to_texture(g_rgba.data(), kWidth, kHeight);
                tex_valid = true;
                g_has_frame = false;
            }
        }

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
    if (tex_valid) free_texture(tex);

    ImGui_ImplOpenGL3_Shutdown();
    ImGui_ImplSDL3_Shutdown();
    ImGui::DestroyContext();
    SDL_GL_DestroyContext(gl_ctx);
    SDL_DestroyWindow(win);
    SDL_Quit();
    return 0;
}
