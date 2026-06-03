/***********************************************************************************************************************
** gkit-media-viewer-c — C VideoFrame transform demo (cimgui + SDL3 + OpenGL3)
**
** Uses gkit-media C FFI and cimgui (C bindings for Dear ImGui).
** Pipeline: BMP → I420 → [Scale|Crop|Rotate|NV12] → RGBA → Display
**
** Usage: cmake --build build-auto --target gkit_media_c_example_viewer
***********************************************************************************************************************/

#include <SDL3/SDL.h>
#ifdef __APPLE__
#define GL_SILENCE_DEPRECATION
#include <OpenGL/gl3.h>
#else
#include <SDL3/SDL_opengl.h>
#endif

#define CIMGUI_DEFINE_ENUMS_AND_STRUCTS
#define CIMGUI_USE_SDL3
#include "cimgui.h"
#include "cimgui_impl.h"

/* OpenGL3 backend functions are inside CIMGUI_USE_GLFW guard in cimgui_impl.h;
   declare them manually since we use SDL3+OpenGL3.
   NOTE: no CIMGUI_API — must use C++ linkage to match imgui_impl_opengl3.cpp */
bool   ImGui_ImplOpenGL3_Init(const char* glsl_version);
void   ImGui_ImplOpenGL3_NewFrame(void);
void   ImGui_ImplOpenGL3_RenderDrawData(ImDrawData* draw_data);
void   ImGui_ImplOpenGL3_Shutdown(void);

extern "C" {
#include "gkit_media.h"
}

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>

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

#define MAX_VARIANTS 32

typedef struct {
    char label[128];
    int w, h;
    uint8_t* rgba;
    GLuint tex;
    int tex_valid;
} FrameVariant;

typedef struct {
    int page;          /* 0=grid, 1=single */
    FrameVariant variants[MAX_VARIANTS];
    int variant_count;
    int thumb_w;
    char status[256];
    int loaded;
    int sel_idx;
    void* orig_i420;
} App;

/* ── Helpers ──────────────────────────────────────────────────────────── */

static void variant_free(FrameVariant* v) {
    if (v->tex_valid) free_texture(v->tex);
    free(v->rgba);
}

static void variant_ensure_texture(FrameVariant* v) {
    if (!v->tex_valid) {
        v->tex = rgba_to_texture(v->rgba, v->w, v->h);
        v->tex_valid = 1;
    }
}

static void variant_from_i420(FrameVariant* v, const char* label, void* handle) {
    snprintf(v->label, sizeof(v->label), "%s", label);
    v->w = (int)gkit_media_video_frame_get_width(handle);
    v->h = (int)gkit_media_video_frame_get_height(handle);
    int stride = v->w * 4, sz = stride * v->h;
    v->rgba = (uint8_t*)malloc(sz);
    memset(v->rgba, 0, (size_t)sz);
    gkit_media_video_frame_i420_to_argb(handle, v->rgba, (uint32_t)stride, 3);
    v->tex = 0; v->tex_valid = 0;
}

static void add_variant(App* app, const char* label, void* handle) {
    if (app->variant_count >= MAX_VARIANTS) return;
    variant_from_i420(&app->variants[app->variant_count], label, handle);
    app->variant_count++;
}

static void add_transform_variant(App* app, const char* label, void* handle) {
    add_variant(app, label, handle);
    app->sel_idx = app->variant_count - 1;
    if (handle) gkit_media_video_frame_destroy(handle);
}

/* ── Asset loading ────────────────────────────────────────────────────── */

static char* asset_path(const char* name) {
    static char buf[1024];
    snprintf(buf, sizeof(buf), "../assets/%s", name);
    FILE* f = fopen(buf, "rb"); if (f) { fclose(f); return buf; }
    snprintf(buf, sizeof(buf), "../../assets/%s", name);
    return buf;
}

static int load_bmp_rgba(const char* path, uint8_t** out, int* w, int* h) {
    SDL_IOStream* io = SDL_IOFromFile(path, "rb"); if (!io) return -1;
    SDL_Surface* surf = SDL_LoadBMP_IO(io, 1); if (!surf) return -1;
    *w = surf->w; *h = surf->h;
    int sz = surf->w * surf->h * 4;
    *out = (uint8_t*)malloc(sz);
    uint8_t* dst = *out, *src = (uint8_t*)surf->pixels;
    int bpp = (int)SDL_BYTESPERPIXEL(surf->format);
    for (int y = 0; y < surf->h; y++)
        for (int x = 0; x < surf->w; x++) {
            int si = y * surf->pitch + x * bpp;
            int di = (y * surf->w + x) * 4;
            dst[di+0] = src[si+2]; dst[di+1] = src[si+1];
            dst[di+2] = src[si+0]; dst[di+3] = (bpp >= 4) ? src[si+3] : 255;
        }
    SDL_DestroySurface(surf);
    return 0;
}

/* ── Build variants ───────────────────────────────────────────────────── */

static void build_variants(App* app) {
    app->variant_count = 0; app->loaded = 0;
    uint8_t* rgba = NULL; int w = 0, h = 0;
    const char* bmp = asset_path("images/color_card_1920x1080.bmp");
    if (load_bmp_rgba(bmp, &rgba, &w, &h) != 0) {
        snprintf(app->status, sizeof(app->status), "Failed: %s", bmp); return;
    }
    void* orig = gkit_media_video_frame_argb_to_i420(rgba, (uint32_t)w, (uint32_t)h, (uint32_t)(w * 4));
    free(rgba);
    if (!orig) { snprintf(app->status, sizeof(app->status), "argb_to_i420 failed"); return; }
    app->orig_i420 = orig;

    add_variant(app, "Original 1920x1080 I420", orig);

    void* h50 = gkit_media_video_frame_scale(orig, (uint32_t)(w/2), (uint32_t)(h/2));
    add_variant(app, "Scale 50% -> 960x540", h50);
    void* h25 = gkit_media_video_frame_scale(orig, (uint32_t)(w/4), (uint32_t)(h/4));
    add_variant(app, "Scale 25% -> 480x270", h25);

    { int cx = ((w-960)/2)&~1, cy = ((h-540)/2)&~1;
      void* cr = gkit_media_video_frame_crop(orig, (uint32_t)cx, (uint32_t)cy, 960, 540);
      add_variant(app, "Crop Center 960x540", cr); gkit_media_video_frame_destroy(cr); }

    int degs[] = {90, 180, 270}; char rl[32];
    for (int d = 0; d < 3; d++) {
        void* rot = gkit_media_video_frame_rotate(orig, (uint32_t)degs[d]);
        snprintf(rl, sizeof(rl), "Rotate %d deg", degs[d]);
        add_variant(app, rl, rot); gkit_media_video_frame_destroy(rot);
    }

    { void* hc = gkit_media_video_frame_scale(orig, 960, 540);
      int cx = ((960-480)/2)&~1, cy = ((540-270)/2)&~1;
      void* c2 = gkit_media_video_frame_crop(hc, (uint32_t)cx, (uint32_t)cy, 480, 270);
      add_variant(app, "Scale->Crop 480x270", c2);
      gkit_media_video_frame_destroy(c2); gkit_media_video_frame_destroy(hc); }

    { void* r90 = gkit_media_video_frame_rotate(orig, 90);
      int rw = (int)gkit_media_video_frame_get_width(r90), rh = (int)gkit_media_video_frame_get_height(r90);
      void* rs = gkit_media_video_frame_scale(r90, (uint32_t)(rw/2), (uint32_t)(rh/2));
      add_variant(app, "Rot90->Scale 50%", rs);
      gkit_media_video_frame_destroy(rs); gkit_media_video_frame_destroy(r90); }

    gkit_media_video_frame_destroy(h50); gkit_media_video_frame_destroy(h25);
    app->sel_idx = 0; app->loaded = 1;
    snprintf(app->status, sizeof(app->status), "%d variants", app->variant_count);
}

/* ── GUI: Grid page ───────────────────────────────────────────────────── */

static void show_grid_page(App* app) {
    int tw = app->thumb_w;
    igText("Thumb:"); igSameLine(0, 4);
    int sizes[] = {120, 180, 240, 320}; char buf[8];
    for (int i = 0; i < 4; i++) {
        snprintf(buf, sizeof(buf), "%d", sizes[i]);
        if (i > 0) igSameLine(0, 2);
        if (igSmallButton(buf)) app->thumb_w = sizes[i];
    }
    igSeparator();

    float avail = igGetContentRegionAvail().x;
    int cols = (int)(avail / (float)(tw + 24));
    if (cols < 1) cols = 1;

    for (int i = 0; i < app->variant_count; i++) {
        FrameVariant* v = &app->variants[i];
        variant_ensure_texture(v);
        float scale = (float)tw / (float)(v->w > v->h ? v->w : v->h);
        igBeginGroup();
        ImTextureRef_c tr = { NULL, (ImTextureID)(intptr_t)v->tex };
        igImage(tr, (ImVec2_c){ (float)v->w * scale, (float)v->h * scale },
                (ImVec2_c){0,0}, (ImVec2_c){1,1});
        igTextUnformatted(v->label, NULL);
        char dim[32]; snprintf(dim, sizeof(dim), "%dx%d", v->w, v->h);
        igTextDisabled(dim);
        igEndGroup();
        if ((i + 1) % cols != 0 && i + 1 < app->variant_count)
            igSameLine(0, 4);
    }
}

/* ── GUI: Single page ─────────────────────────────────────────────────── */

static void show_single_page(App* app) {
    if (app->sel_idx < 0 || app->sel_idx >= app->variant_count) return;
    FrameVariant* v = &app->variants[app->sel_idx];
    variant_ensure_texture(v);

    igColumns(2, "single_layout", true);
    igSetColumnWidth(0, 220);

    int ow = (int)gkit_media_video_frame_get_width(app->orig_i420);
    int oh = (int)gkit_media_video_frame_get_height(app->orig_i420);

    igTextUnformatted("Transform", NULL);
    igSeparator();
    if (igButton("Original", (ImVec2){-1, 0})) app->sel_idx = 0;

    igSpacing(); igTextUnformatted("Scale", NULL);
    if (igButton("50%", (ImVec2){-1, 0})) {
        void* sc = gkit_media_video_frame_scale(app->orig_i420, (uint32_t)(ow/2), (uint32_t)(oh/2));
        add_transform_variant(app, "Scale 50%", sc);
    }
    if (igButton("25%", (ImVec2){-1, 0})) {
        void* sc = gkit_media_video_frame_scale(app->orig_i420, (uint32_t)(ow/4), (uint32_t)(oh/4));
        add_transform_variant(app, "Scale 25%", sc);
    }

    igSpacing(); igTextUnformatted("Crop", NULL);
    if (igButton("Center 960x540", (ImVec2){-1, 0})) {
        int cx = ((ow-960)/2)&~1, cy = ((oh-540)/2)&~1;
        void* cr = gkit_media_video_frame_crop(app->orig_i420, (uint32_t)cx, (uint32_t)cy, 960, 540);
        add_transform_variant(app, "Crop 960x540", cr);
    }

    igSpacing(); igTextUnformatted("Rotate", NULL);
    int degs[] = {90, 180, 270}; char btn[16];
    for (int d = 0; d < 3; d++) {
        snprintf(btn, sizeof(btn), "%d deg", degs[d]);
        if (igButton(btn, (ImVec2){-1, 0})) {
            void* rot = gkit_media_video_frame_rotate(app->orig_i420, (uint32_t)degs[d]);
            snprintf(btn, sizeof(btn), "Rotate %d deg", degs[d]);
            add_transform_variant(app, btn, rot);
        }
    }

    igSpacing(); igTextUnformatted("Format Convert", NULL);
    igSeparator();
    if (igButton("I420->NV12->I420", (ImVec2){-1, 0})) {
        void* nv12 = gkit_media_video_frame_i420_to_nv12(app->orig_i420);
        if (nv12) {
            void* back = gkit_media_video_frame_nv12_to_i420(nv12);
            add_transform_variant(app, "NV12 round-trip", back);
            gkit_media_video_frame_destroy(nv12);
        }
    }

    igSpacing(); igSeparator();
    igTextUnformatted("Variant List", NULL);
    for (int i = 0; i < app->variant_count; i++)
        if (igSelectable_Bool(app->variants[i].label, i == app->sel_idx, 0, (ImVec2){0,0}))
            app->sel_idx = i;

    igNextColumn();

    char info[256];
    snprintf(info, sizeof(info), "%s | %dx%d", v->label, v->w, v->h);
    igTextUnformatted(info, NULL);
    float avail_w = igGetContentRegionAvail().x;
    float avail_h = igGetContentRegionAvail().y;
    float scale = avail_w / (float)v->w;
    if (avail_h / (float)v->h < scale) scale = avail_h / (float)v->h;
    if (scale > 1.0f) scale = 1.0f;
    ImTextureRef_c tr2 = { NULL, (ImTextureID)(intptr_t)v->tex };
    igImage(tr2, (ImVec2_c){ (float)v->w * scale, (float)v->h * scale },
            (ImVec2_c){0,0}, (ImVec2_c){1,1});
    igColumns(1, NULL, false);
}

/* ============================================================================
 * Main
 * ============================================================================ */

int main(int argc, char** argv) {
    (void)argc; (void)argv;
    if (!SDL_Init(SDL_INIT_VIDEO)) {
        fprintf(stderr, "SDL_Init: %s\n", SDL_GetError()); return 1;
    }

    SDL_GL_SetAttribute(SDL_GL_CONTEXT_MAJOR_VERSION, 3);
    SDL_GL_SetAttribute(SDL_GL_CONTEXT_MINOR_VERSION, 2);
    SDL_GL_SetAttribute(SDL_GL_CONTEXT_PROFILE_MASK, SDL_GL_CONTEXT_PROFILE_CORE);
    SDL_GL_SetAttribute(SDL_GL_DOUBLEBUFFER, 1);

    SDL_Window* win = SDL_CreateWindow("gkit-media C Viewer (cimgui + SDL3)",
        1200, 800, SDL_WINDOW_OPENGL | SDL_WINDOW_RESIZABLE);
    if (!win) { fprintf(stderr, "Window: %s\n", SDL_GetError()); SDL_Quit(); return 1; }
    SDL_GLContext gl_ctx = SDL_GL_CreateContext(win);
    SDL_GL_SetSwapInterval(1);

    igCreateContext(NULL);
    igStyleColorsDark(NULL);

    ImGui_ImplSDL3_InitForOpenGL(win, gl_ctx);
    ImGui_ImplOpenGL3_Init("#version 150");

    App app; memset(&app, 0, sizeof(app));
    app.thumb_w = 240;
    build_variants(&app);

    int running = 1;
    while (running) {
        SDL_Event evt;
        while (SDL_PollEvent(&evt)) {
            ImGui_ImplSDL3_ProcessEvent(&evt);
            if (evt.type == SDL_EVENT_QUIT) running = 0;
        }
        ImGui_ImplOpenGL3_NewFrame();
        ImGui_ImplSDL3_NewFrame();
        igNewFrame();

        int ww, wh; SDL_GetWindowSize(win, &ww, &wh);

        /* ── Main menu bar ── */
        if (igBeginMainMenuBar()) {
            igTextUnformatted("gkit-media C Viewer (cimgui)", NULL);
            igSameLine(0, 20);
            if (igRadioButton_Bool("Grid View", app.page == 0)) app.page = 0;
            igSameLine(0, 4);
            if (igRadioButton_Bool("Single View", app.page == 1)) app.page = 1;
            igSameLine(0, 20);
            if (igButton("Reload", (ImVec2){0,0})) {
                for (int i = 0; i < app.variant_count; i++) variant_free(&app.variants[i]);
                if (app.orig_i420) { gkit_media_video_frame_destroy(app.orig_i420); app.orig_i420 = NULL; }
                memset(&app, 0, sizeof(App)); app.thumb_w = 240;
                build_variants(&app);
            }
            igSameLine(0, 10);
            igTextUnformatted(app.status, NULL);
            igEndMainMenuBar();
        }

        /* ── Main content ── */
        igSetNextWindowPos((ImVec2){0, igGetFrameHeight()}, ImGuiCond_Always, (ImVec2){0,0});
        igSetNextWindowSize((ImVec2){ (float)ww, (float)(wh - igGetFrameHeight()) }, ImGuiCond_Always);
        igBegin("##main", NULL,
            ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize |
            ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoScrollbar);
        if (!app.loaded) {
            igTextUnformatted("Loading...", NULL);
        } else if (app.page == 0) {
            show_grid_page(&app);
        } else {
            show_single_page(&app);
        }
        igEnd();

        igRender();
        glViewport(0, 0, ww, wh);
        glClearColor(0.15f, 0.15f, 0.15f, 1.0f);
        glClear(GL_COLOR_BUFFER_BIT);
        ImGui_ImplOpenGL3_RenderDrawData(igGetDrawData());
        SDL_GL_SwapWindow(win);
    }

    for (int i = 0; i < app.variant_count; i++) variant_free(&app.variants[i]);
    if (app.orig_i420) gkit_media_video_frame_destroy(app.orig_i420);
    ImGui_ImplOpenGL3_Shutdown();
    ImGui_ImplSDL3_Shutdown();
    igDestroyContext(NULL);
    SDL_GL_DestroyContext(gl_ctx); SDL_DestroyWindow(win); SDL_Quit();
    return 0;
}
