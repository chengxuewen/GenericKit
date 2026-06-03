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

#ifndef _GKIT_MEDIA_API_H
#define _GKIT_MEDIA_API_H

#include <gkit_core_api.h>

/* GKIT_MEDIA_API */
#ifdef GKIT_BUILD_SHARED
#   ifdef GKIT_BUILDING_MEDIA_LIB
#       define GKIT_MEDIA_API GKIT_DECLARE_EXPORT
#   else
#       define GKIT_MEDIA_API GKIT_DECLARE_IMPORT
#   endif
#   define GKIT_MEDIA_HIDDEN GKIT_DECLARE_HIDDEN
#else
#   define GKIT_MEDIA_API
#   define GKIT_MEDIA_HIDDEN
#endif

#endif // _GKIT_MEDIA_API_H
