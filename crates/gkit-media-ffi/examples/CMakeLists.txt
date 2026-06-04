########################################################################################################################
# C example: gkit-media-viewer-c — VideoFrame transform demo (cimgui + SDL3 + OpenGL3)

gkit_find_package(WrapCImGui PROVIDED_TARGETS GKitWrapCImGui::WrapCImGui)
gkit_find_package(WrapSDL3 PROVIDED_TARGETS GKitWrapSDL3::WrapSDL3)
find_package(OpenGL REQUIRED)

set(_example_name gkit-media-c-example-viewer)
set(_source_dir "${CMAKE_CURRENT_SOURCE_DIR}")

add_executable(${_example_name}
    "${_source_dir}/viewer.c"
)

target_link_libraries(${_example_name}
    gkit-media-c
    GKitWrapCImGui::WrapCImGui
    GKitWrapSDL3::WrapSDL3
    OpenGL::GL
)

target_include_directories(${_example_name} PRIVATE "${_source_dir}")
set_source_files_properties("${_source_dir}/viewer.c" PROPERTIES LANGUAGE CXX)

set_target_properties(${_example_name} PROPERTIES
    FOLDER "gkit-media-ffi/examples"
    RUNTIME_OUTPUT_DIRECTORY "${GKIT_BUILD_DIR}/${GKIT_INSTALL_BINDIR}"
    CXX_STANDARD 17
)

add_custom_target(gkit-c-examples-assets
    COMMAND ${CMAKE_COMMAND} -E copy_directory
        "${PROJECT_SOURCE_DIR}/assets"
        "${GKIT_BUILD_DIR}/assets"
    COMMENT "Copying assets for C examples"
    VERBATIM)

set_target_properties(gkit-c-examples-assets PROPERTIES
    FOLDER "gkit-media-ffi/examples")
