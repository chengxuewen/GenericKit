#pragma once

extern "C" {
#include "gkit_media.h"
}

#include <cstdint>

namespace gkit {

inline void media_hello()
{
    gkit_media_hello();
}

} // namespace gkit
