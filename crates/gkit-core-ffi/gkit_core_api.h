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

#ifndef _GKIT_CORE_API_H
#define _GKIT_CORE_API_H

/***********************************************************************************************************************
 * compiler dll visibility macros — shared by all gkit modules
 ***********************************************************************************************************************/
#if defined(__GNUC__) && (__GNUC__ >= 4)
#   define GKIT_DECLARE_EXPORT __attribute__((visibility("default")))
#   define GKIT_DECLARE_IMPORT __attribute__((visibility("default")))
#   define GKIT_DECLARE_HIDDEN __attribute__((visibility("hidden")))
#elif defined(_WIN32)
#   define GKIT_DECLARE_EXPORT __declspec(dllexport)
#   define GKIT_DECLARE_IMPORT __declspec(dllimport)
#   define GKIT_DECLARE_HIDDEN
#else
#   define GKIT_DECLARE_EXPORT __attribute__((visibility("default")))
#   define GKIT_DECLARE_IMPORT __attribute__((visibility("default")))
#   define GKIT_DECLARE_HIDDEN
#endif

#ifndef GKIT_DECLARE_EXPORT
#   define GKIT_DECLARE_EXPORT
#endif
#ifndef GKIT_DECLARE_IMPORT
#   define GKIT_DECLARE_IMPORT
#endif
#ifndef GKIT_DECLARE_HIDDEN
#   define GKIT_DECLARE_HIDDEN
#endif

/***********************************************************************************************************************
 * per-module API macros
 ***********************************************************************************************************************/

/* GKIT_CORE_API */
#ifdef GKIT_BUILD_SHARED
#   ifdef GKIT_BUILDING_CORE_LIB
#       define GKIT_CORE_API GKIT_DECLARE_EXPORT
#   else
#       define GKIT_CORE_API GKIT_DECLARE_IMPORT
#   endif
#   define GKIT_CORE_HIDDEN GKIT_DECLARE_HIDDEN
#else
#   define GKIT_CORE_API
#   define GKIT_CORE_HIDDEN
#endif

#endif // _GKIT_CORE_API_H
