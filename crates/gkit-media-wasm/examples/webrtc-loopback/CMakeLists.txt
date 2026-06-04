set(example_dir "${CMAKE_CURRENT_SOURCE_DIR}")
set(output_dir "${GKIT_BUILD_DIR}/examples/web/webrtc-loopback")
set(wasm_dir "${GKIT_BUILD_DIR}/wasm/gkit-media-wasm")

add_custom_target(copy-gkit-wasm-webrtc-loopback ALL
    COMMAND ${CMAKE_COMMAND} -E make_directory "${output_dir}"
    COMMAND ${CMAKE_COMMAND} -E copy "${example_dir}/index.html" "${output_dir}/"
    COMMAND ${CMAKE_COMMAND} -E copy "${wasm_dir}/gkit_media_wasm.js" "${output_dir}/"
    COMMAND ${CMAKE_COMMAND} -E copy "${wasm_dir}/gkit_media_wasm_bg.wasm" "${output_dir}/"
    COMMAND ${CMAKE_COMMAND} -E copy "${wasm_dir}/gkit_media_wasm.d.ts" "${output_dir}/"
    COMMAND ${CMAKE_COMMAND} -E copy "${wasm_dir}/gkit_media_wasm_bg.wasm.d.ts" "${output_dir}/"
    DEPENDS cargo-build_gkit_media_wasm_optimize
    COMMENT "Deploying WebRTC P2P loopback example to ${output_dir}"
)

set_target_properties(copy-gkit-wasm-webrtc-loopback PROPERTIES
    FOLDER "gkit-media-wasm/examples")
