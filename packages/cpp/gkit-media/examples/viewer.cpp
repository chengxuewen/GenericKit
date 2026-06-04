/***********************************************************************************************************************
** gkit-media-viewer-cpp — C++ VideoFrame transform demo (ImGui + SDL3 + OpenGL3)
**
** Uses gkit::VideoFrame C++ RAII class.
** Pipeline: BMP → I420 → [Scale|Crop|Rotate|NV12] → RGBA → Display
**
** Usage: cmake --build build-auto --target gkit_media_cpp_example_viewer
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

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <vector>
#include <string>

/* ============================================================================
 * Texture helpers
 * ============================================================================ */

static GLuint rgba_to_texture(const uint8_t* rgba, int w, int h) {
    GLuint tex = 0; glGenTextures(1, &tex);
    glBindTexture(GL_TEXTURE_2D, tex);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
    glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
    glTexImage2D(GL_TEXTURE_2D, 0, GL_RGBA, w, h, 0, GL_RGBA, GL_UNSIGNED_BYTE, rgba);
    return tex;
}

static void free_texture(GLuint tex) { glDeleteTextures(1, &tex); }

/* ============================================================================
 * Frame variant
 * ============================================================================ */

struct FrameVariant {
    std::string label;
    int w = 0, h = 0;
    std::vector<uint8_t> rgba;
    GLuint tex = 0;
    bool tex_valid = false;
    ~FrameVariant() { if (tex_valid) free_texture(tex); }
    FrameVariant() = default;
    FrameVariant(FrameVariant&& o) noexcept
        : label(std::move(o.label)), w(o.w), h(o.h), rgba(std::move(o.rgba)),
          tex(o.tex), tex_valid(o.tex_valid) { o.tex_valid = false; }
    FrameVariant& operator=(FrameVariant&& o) noexcept {
        if (this != &o) {
            if (tex_valid) free_texture(tex);
            label = std::move(o.label); w = o.w; h = o.h;
            rgba = std::move(o.rgba); tex = o.tex;
            tex_valid = o.tex_valid; o.tex_valid = false;
        } return *this;
    }
    FrameVariant(const FrameVariant&) = delete;
    FrameVariant& operator=(const FrameVariant&) = delete;
    void ensure_texture() {
        if (!tex_valid) { tex = rgba_to_texture(rgba.data(), w, h); tex_valid = true; }
    }
    ImTextureID imgui_id() const { return (ImTextureID)(intptr_t)tex; }
};

/* ============================================================================
 * Application
 * ============================================================================ */

struct App {
    std::vector<FrameVariant> variants;
    int thumb_w = 240;
    char status[256] = {};
    bool loaded = false;
    int sel_idx = 0;
    void* orig_i420 = nullptr;
    int page = 0;  // 0=grid, 1=single
};

/* ============================================================================
 * Asset loading
 * ============================================================================ */

static std::string asset_path(const char* name) {
    std::string p = std::string("../assets/") + name;
    FILE* f = fopen(p.c_str(), "rb"); if (f) { fclose(f); return p; }
    return std::string("../../assets/") + name;
}

static bool load_bmp_rgba(const char* path, std::vector<uint8_t>& out, int* w, int* h) {
    SDL_IOStream* io = SDL_IOFromFile(path, "rb"); if (!io) return false;
    SDL_Surface* surf = SDL_LoadBMP_IO(io, 1); if (!surf) return false;
    *w = surf->w; *h = surf->h;
    out.resize(surf->w * surf->h * 4);
    uint8_t* dst = out.data(), *src = (uint8_t*)surf->pixels;
    int bpp = (int)SDL_BYTESPERPIXEL(surf->format);
    for (int y = 0; y < surf->h; y++)
        for (int x = 0; x < surf->w; x++) {
            int si = y * surf->pitch + x * bpp;
            int di = (y * surf->w + x) * 4;
            dst[di+0] = src[si+2]; dst[di+1] = src[si+1];
            dst[di+2] = src[si+0]; dst[di+3] = (bpp >= 4) ? src[si+3] : 255;
        }
    SDL_DestroySurface(surf);
    return true;
}

/* ============================================================================
 * Build variants
 * ============================================================================ */

static void add_rgba_variant(App& app, const char* label, void* handle) {
    if (!handle) return;
    FrameVariant v;
    v.label = label;
    v.w = (int)gkit_media_video_frame_get_width(handle);
    v.h = (int)gkit_media_video_frame_get_height(handle);
    int stride = v.w * 4;
    v.rgba.resize(stride * v.h);
    gkit_media_video_frame_i420_to_argb(handle, v.rgba.data(), (uint32_t)stride, 3);
    app.variants.push_back(std::move(v));
}

static void add_transform_variant(App& app, const char* label, void* handle) {
    add_rgba_variant(app, label, handle);
    app.sel_idx = (int)app.variants.size() - 1;
    if (handle) gkit_media_video_frame_destroy(handle);
}

static void build_variants(App& app) {
    app.variants.clear(); app.loaded = false;
    std::vector<uint8_t> rgba; int w = 0, h = 0;
    std::string bmp = asset_path("images/color_card_1920x1080.bmp");
    if (!load_bmp_rgba(bmp.c_str(), rgba, &w, &h)) {
        snprintf(app.status, sizeof(app.status), "Failed: %s", bmp.c_str()); return;
    }
    void* orig = gkit_media_video_frame_argb_to_i420(
        rgba.data(), (uint32_t)w, (uint32_t)h, (uint32_t)(w * 4));
    if (!orig) { snprintf(app.status, sizeof(app.status), "argb_to_i420 failed"); return; }
    app.orig_i420 = orig;

    add_rgba_variant(app, "Original 1920x1080 I420", orig);

    void* h50 = gkit_media_video_frame_scale(orig, (uint32_t)(w/2), (uint32_t)(h/2));
    add_rgba_variant(app, "Scale 50% -> 960x540", h50);
    void* h25 = gkit_media_video_frame_scale(orig, (uint32_t)(w/4), (uint32_t)(h/4));
    add_rgba_variant(app, "Scale 25% -> 480x270", h25);

    { int cx = ((w-960)/2)&~1, cy = ((h-540)/2)&~1;
      void* cr = gkit_media_video_frame_crop(orig, (uint32_t)cx, (uint32_t)cy, 960, 540);
      add_rgba_variant(app, "Crop Center 960x540", cr);
      gkit_media_video_frame_destroy(cr); }

    int degs[] = {90, 180, 270}; char rl[32];
    for (int d = 0; d < 3; d++) {
        void* rot = gkit_media_video_frame_rotate(orig, (uint32_t)degs[d]);
        snprintf(rl, sizeof(rl), "Rotate %d deg", degs[d]);
        add_rgba_variant(app, rl, rot); gkit_media_video_frame_destroy(rot);
    }

    { void* hc = gkit_media_video_frame_scale(orig, 960, 540);
      int cx = ((960-480)/2)&~1, cy = ((540-270)/2)&~1;
      void* c2 = gkit_media_video_frame_crop(hc, (uint32_t)cx, (uint32_t)cy, 480, 270);
      add_rgba_variant(app, "Scale->Crop 480x270", c2);
      gkit_media_video_frame_destroy(c2); gkit_media_video_frame_destroy(hc); }

    { void* r90 = gkit_media_video_frame_rotate(orig, 90);
      int rw = (int)gkit_media_video_frame_get_width(r90);
      int rh = (int)gkit_media_video_frame_get_height(r90);
      void* rs = gkit_media_video_frame_scale(r90, (uint32_t)(rw/2), (uint32_t)(rh/2));
      add_rgba_variant(app, "Rot90->Scale 50%", rs);
      gkit_media_video_frame_destroy(rs); gkit_media_video_frame_destroy(r90); }

    // format conversions
    { void* nv12 = gkit_media_video_frame_i420_to_nv12(orig);
      void* nv12_i420 = gkit_media_video_frame_nv12_to_i420(nv12);
      add_rgba_variant(app, "NV12 round-trip", nv12_i420);
      gkit_media_video_frame_destroy(nv12_i420); gkit_media_video_frame_destroy(nv12); }
    { void* nv21 = gkit_media_video_frame_i420_to_nv21(orig);
      void* nv21_i420 = gkit_media_video_frame_nv21_to_i420(nv21);
      add_rgba_variant(app, "NV21 round-trip", nv21_i420);
      gkit_media_video_frame_destroy(nv21_i420); gkit_media_video_frame_destroy(nv21); }

    gkit_media_video_frame_destroy(h50); gkit_media_video_frame_destroy(h25);
    app.sel_idx = 0; app.loaded = true;
    snprintf(app.status, sizeof(app.status), "%zu variants", app.variants.size());
}

/* ============================================================================
 * GUI
 * ============================================================================ */

static void show_grid_page(App& app) {
    int tw = app.thumb_w;
    ImGui::TextUnformatted("Thumb:"); ImGui::SameLine();
    int sizes[] = {120, 180, 240, 320};
    for (int i = 0; i < 4; i++) {
        char b[8]; snprintf(b, sizeof(b), "%d", sizes[i]);
        if (i > 0) ImGui::SameLine();
        if (ImGui::SmallButton(b)) app.thumb_w = sizes[i];
    }
    ImGui::Separator();

    float avail = ImGui::GetContentRegionAvail().x;
    int cols = (int)(avail / (float)(tw + 24));
    if (cols < 1) cols = 1;

    for (int i = 0; i < (int)app.variants.size(); i++) {
        FrameVariant& v = app.variants[i];
        v.ensure_texture();
        float scale = (float)tw / (float)(v.w > v.h ? v.w : v.h);
        ImGui::BeginGroup();
        ImGui::Image(v.imgui_id(), ImVec2((float)v.w * scale, (float)v.h * scale));
        ImGui::TextUnformatted(v.label.c_str());
        ImGui::TextDisabled("%dx%d", v.w, v.h);
        ImGui::EndGroup();
        if ((i + 1) % cols != 0 && i + 1 < (int)app.variants.size())
            ImGui::SameLine();
    }
}

static void show_single_page(App& app) {
    if (app.sel_idx < 0 || app.sel_idx >= (int)app.variants.size()) return;
    FrameVariant& v = app.variants[app.sel_idx];
    v.ensure_texture();

    ImGui::Columns(2, "single_layout", false);
    ImGui::SetColumnWidth(0, 220);

    int ow = (int)gkit_media_video_frame_get_width(app.orig_i420);
    int oh = (int)gkit_media_video_frame_get_height(app.orig_i420);

    ImGui::TextUnformatted("Transform");
    ImGui::Separator();
    if (ImGui::Button("Original", ImVec2(-1, 0))) app.sel_idx = 0;

    ImGui::Spacing(); ImGui::TextUnformatted("Scale");
    if (ImGui::Button("50%", ImVec2(-1, 0))) {
        void* sc = gkit_media_video_frame_scale(app.orig_i420, (uint32_t)(ow/2), (uint32_t)(oh/2));
        add_transform_variant(app, "Scale 50%", sc);
    }
    if (ImGui::Button("25%", ImVec2(-1, 0))) {
        void* sc = gkit_media_video_frame_scale(app.orig_i420, (uint32_t)(ow/4), (uint32_t)(oh/4));
        add_transform_variant(app, "Scale 25%", sc);
    }

    ImGui::Spacing(); ImGui::TextUnformatted("Crop");
    if (ImGui::Button("Center 960x540", ImVec2(-1, 0))) {
        int cx = ((ow-960)/2)&~1, cy = ((oh-540)/2)&~1;
        void* cr = gkit_media_video_frame_crop(app.orig_i420, (uint32_t)cx, (uint32_t)cy, 960, 540);
        add_transform_variant(app, "Crop 960x540", cr);
    }

    ImGui::Spacing(); ImGui::TextUnformatted("Rotate");
    int degs[] = {90, 180, 270}; char btn[16];
    for (int d = 0; d < 3; d++) {
        snprintf(btn, sizeof(btn), "%d deg", degs[d]);
        if (ImGui::Button(btn, ImVec2(-1, 0))) {
            void* rot = gkit_media_video_frame_rotate(app.orig_i420, (uint32_t)degs[d]);
            snprintf(btn, sizeof(btn), "Rotate %d deg", degs[d]);
            add_transform_variant(app, btn, rot);
        }
    }

    ImGui::Spacing(); ImGui::TextUnformatted("Format Convert");
    ImGui::Separator();
    if (ImGui::Button("I420 -> NV12 -> I420", ImVec2(-1, 0))) {
        void* nv12 = gkit_media_video_frame_i420_to_nv12(app.orig_i420);
        if (nv12) {
            void* back = gkit_media_video_frame_nv12_to_i420(nv12);
            add_transform_variant(app, "NV12 round-trip", back);
            gkit_media_video_frame_destroy(nv12);
        }
    }
    if (ImGui::Button("I420 -> NV21 -> I420", ImVec2(-1, 0))) {
        void* nv21 = gkit_media_video_frame_i420_to_nv21(app.orig_i420);
        if (nv21) {
            void* back = gkit_media_video_frame_nv21_to_i420(nv21);
            add_transform_variant(app, "NV21 round-trip", back);
            gkit_media_video_frame_destroy(nv21);
        }
    }

    ImGui::Spacing(); ImGui::Separator();
    ImGui::TextUnformatted("Variant List");
    for (int i = 0; i < (int)app.variants.size(); i++)
        if (ImGui::Selectable(app.variants[i].label.c_str(), i == app.sel_idx))
            app.sel_idx = i;

    ImGui::NextColumn();

    ImGui::Text("%s | %dx%d", v.label.c_str(), v.w, v.h);
    float avail_w = ImGui::GetContentRegionAvail().x;
    float avail_h = ImGui::GetContentRegionAvail().y;
    float scale = avail_w / (float)v.w;
    if (avail_h / (float)v.h < scale) scale = avail_h / (float)v.h;
    if (scale > 1.0f) scale = 1.0f;
    ImGui::Image(v.imgui_id(), ImVec2((float)v.w * scale, (float)v.h * scale));
    ImGui::Columns(1);
}

/* ============================================================================
 * Main
 * ============================================================================ */

int main(int, char**) {
    if (!SDL_Init(SDL_INIT_VIDEO)) { fprintf(stderr, "SDL_Init: %s\n", SDL_GetError()); return 1; }
    SDL_GL_SetAttribute(SDL_GL_CONTEXT_MAJOR_VERSION, 3);
    SDL_GL_SetAttribute(SDL_GL_CONTEXT_MINOR_VERSION, 2);
    SDL_GL_SetAttribute(SDL_GL_CONTEXT_PROFILE_MASK, SDL_GL_CONTEXT_PROFILE_CORE);
    SDL_GL_SetAttribute(SDL_GL_DOUBLEBUFFER, 1);

    SDL_Window* win = SDL_CreateWindow("gkit-media C++ Viewer (ImGui + SDL3)",
        1200, 800, SDL_WINDOW_OPENGL | SDL_WINDOW_RESIZABLE);
    if (!win) { fprintf(stderr, "Window: %s\n", SDL_GetError()); SDL_Quit(); return 1; }
    SDL_GLContext gl_ctx = SDL_GL_CreateContext(win);
    SDL_GL_SetSwapInterval(1);

    IMGUI_CHECKVERSION();
    ImGui::CreateContext();
    ImGui::StyleColorsDark();
    ImGui_ImplSDL3_InitForOpenGL(win, gl_ctx);
    ImGui_ImplOpenGL3_Init("#version 150");

    App app; build_variants(app);
    bool running = true;

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

        /* ── Main menu bar ── */
        if (ImGui::BeginMainMenuBar()) {
            ImGui::TextUnformatted("gkit-media C++ Viewer");
            ImGui::SameLine(0, 20);
            if (ImGui::RadioButton("Grid View", app.page == 0)) app.page = 0;
            ImGui::SameLine(0, 4);
            if (ImGui::RadioButton("Single View", app.page == 1)) app.page = 1;
            ImGui::SameLine(0, 20);
            if (ImGui::Button("Reload")) {
                if (app.orig_i420) { gkit_media_video_frame_destroy(app.orig_i420); app.orig_i420 = nullptr; }
                app.variants.clear(); build_variants(app);
            }
            ImGui::SameLine(0, 10);
            ImGui::TextUnformatted(app.status);
            ImGui::EndMainMenuBar();
        }

        /* ── Main content ── */
        ImGui::SetNextWindowPos(ImVec2(0, ImGui::GetFrameHeight()));
        ImGui::SetNextWindowSize(ImVec2((float)ww, (float)(wh - ImGui::GetFrameHeight())));
        ImGui::Begin("##main", nullptr,
            ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
            ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar);
        if (!app.loaded) ImGui::TextUnformatted("Loading...");
        else if (app.page == 0) show_grid_page(app);
        else show_single_page(app);
        ImGui::End();

        ImGui::Render();
        glViewport(0, 0, ww, wh);
        glClearColor(0.15f, 0.15f, 0.15f, 1.0f);
        glClear(GL_COLOR_BUFFER_BIT);
        ImGui_ImplOpenGL3_RenderDrawData(ImGui::GetDrawData());
        SDL_GL_SwapWindow(win);
    }

    if (app.orig_i420) gkit_media_video_frame_destroy(app.orig_i420);
    ImGui_ImplOpenGL3_Shutdown(); ImGui_ImplSDL3_Shutdown();
    ImGui::DestroyContext();
    SDL_GL_DestroyContext(gl_ctx); SDL_DestroyWindow(win); SDL_Quit();
    return 0;
}