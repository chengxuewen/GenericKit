gkit_find_package(WrapImGui PROVIDED_TARGETS GKitWrapImGui::WrapImGui)
gkit_find_package(WrapSDL3 PROVIDED_TARGETS GKitWrapSDL3::WrapSDL3)
find_package(OpenGL REQUIRED)

set(_example_name gkit_media_cpp_example_square_gen)
set(_source_dir "${CMAKE_CURRENT_SOURCE_DIR}")

add_executable(${_example_name}
    "${_source_dir}/main.cpp"
)

target_link_libraries(${_example_name}
    gkit_media_cpp
    gkit_media_c
    GKitWrapImGui::WrapImGui
    GKitWrapSDL3::WrapSDL3
    OpenGL::GL
)

target_include_directories(${_example_name} PRIVATE
    "${_source_dir}"
)

set_target_properties(${_example_name} PROPERTIES
    FOLDER "gkit_media/bindings/cpp/examples"
    RUNTIME_OUTPUT_DIRECTORY "${GKIT_BUILD_DIR}/${GKIT_INSTALL_BINDIR}"
    CXX_STANDARD 17
)
