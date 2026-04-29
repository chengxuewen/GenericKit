########################################################################################################################
#
# Library: GenericKit
#
# Copyright (C) 2026~Present ChengXueWen.
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
# http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an “AS IS” BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
#
########################################################################################################################

#-----------------------------------------------------------------------------------------------------------------------
# gkit_set01 finction
#-----------------------------------------------------------------------------------------------------------------------
function(gkit_set01 result)
    if(${ARGN})
        set("${result}" 1 PARENT_SCOPE)
    else()
        set("${result}" 0 PARENT_SCOPE)
    endif()
endfunction()


#-----------------------------------------------------------------------------------------------------------------------
# OpenCTK set system variable
#-----------------------------------------------------------------------------------------------------------------------
message(STATUS "Build in system: ${CMAKE_SYSTEM_NAME}")
set(GKIT_SYSTEM_NAME ${CMAKE_SYSTEM_NAME})
set(GKIT_SYSTEM_VERSION ${CMAKE_SYSTEM_VERSION})
set(GKIT_SYSTEM_PROCESSOR ${CMAKE_SYSTEM_PROCESSOR})
gkit_set01(GKIT_SYSTEM_LINUX
    CMAKE_SYSTEM_NAME STREQUAL "Linux")
gkit_set01(GKIT_SYSTEM_WIN32 WIN32)
gkit_set01(GKIT_SYSTEM_WINCE
    CMAKE_SYSTEM_NAME STREQUAL "WindowsCE")
gkit_set01(GKIT_SYSTEM_WIN
	GKIT_SYSTEM_WINCE OR GKIT_SYSTEM_WIN32 OR CMAKE_SYSTEM_NAME STREQUAL "Windows")
gkit_set01(GKIT_SYSTEM_HPUX
    CMAKE_SYSTEM_NAME STREQUAL "HPUX")
gkit_set01(GKIT_SYSTEM_ANDROID
    CMAKE_SYSTEM_NAME STREQUAL "Android")
gkit_set01(GKIT_SYSTEM_NACL
    CMAKE_SYSTEM_NAME STREQUAL "NaCl")
gkit_set01(GKIT_SYSTEM_INTEGRITY
    CMAKE_SYSTEM_NAME STREQUAL "Integrity")
gkit_set01(GKIT_SYSTEM_VXWORKS
    CMAKE_SYSTEM_NAME STREQUAL "VxWorks")
gkit_set01(GKIT_SYSTEM_QNX
    CMAKE_SYSTEM_NAME STREQUAL "QNX")
gkit_set01(GKIT_SYSTEM_OPENBSD
    CMAKE_SYSTEM_NAME STREQUAL "OpenBSD")
gkit_set01(GKIT_SYSTEM_FREEBSD
    CMAKE_SYSTEM_NAME STREQUAL "FreeBSD")
gkit_set01(GKIT_SYSTEM_NETBSD
    CMAKE_SYSTEM_NAME STREQUAL "NetBSD")
gkit_set01(GKIT_SYSTEM_WASM
    CMAKE_SYSTEM_NAME STREQUAL "Emscripten" OR EMSCRIPTEN)
gkit_set01(GKIT_SYSTEM_SOLARIS
    CMAKE_SYSTEM_NAME STREQUAL "SunOS")
gkit_set01(GKIT_SYSTEM_HURD
    CMAKE_SYSTEM_NAME STREQUAL "GNU")
# This is the only reliable way we can determine the webOS platform as the yocto recipe adds this compile definition
# into its generated toolchain.cmake file
gkit_set01(GKIT_SYSTEM_WEBOS
    CMAKE_CXX_FLAGS MATCHES "-D__WEBOS__")
gkit_set01(GKIT_SYSTEM_BSD
    APPLE OR OPENBSD OR FREEBSD OR NETBSD)
gkit_set01(GKIT_SYSTEM_DARWIN
    APPLE OR CMAKE_SYSTEM_NAME STREQUAL "Darwin")
gkit_set01(GKIT_SYSTEM_IOS
    APPLE AND CMAKE_SYSTEM_NAME STREQUAL "iOS")
gkit_set01(GKIT_SYSTEM_TVOS
    APPLE AND CMAKE_SYSTEM_NAME STREQUAL "tvOS")
gkit_set01(GKIT_SYSTEM_WATCHOS
    APPLE AND CMAKE_SYSTEM_NAME STREQUAL "watchOS")
gkit_set01(GKIT_SYSTEM_UIKIT
    APPLE AND (IOS OR TVOS OR WATCHOS))
gkit_set01(GKIT_SYSTEM_MACOS
	APPLE AND NOT UIKIT)
gkit_set01(GKIT_SYSTEM_UNIX UNIX)
gkit_set01(GKIT_SYSTEM_APPLE APPLE)
gkit_set01(GKIT_SYSTEM_MAC APPLE)


#-----------------------------------------------------------------------------------------------------------------------
# OpenCTK set processor variable
#-----------------------------------------------------------------------------------------------------------------------
message(STATUS "Build in processor: ${CMAKE_SYSTEM_PROCESSOR}")
string(TOLOWER "${CMAKE_SYSTEM_PROCESSOR}" GKIT_SYSTEM_PROCESSOR)
gkit_set01(GKIT_PROCESSOR_I386
    GKIT_SYSTEM_PROCESSOR STREQUAL "i386")
gkit_set01(GKIT_PROCESSOR_I686
    GKIT_SYSTEM_PROCESSOR MATCHES "i686")
gkit_set01(GKIT_PROCESSOR_X86_64
    GKIT_SYSTEM_PROCESSOR MATCHES "x86_64")
gkit_set01(GKIT_PROCESSOR_X86
    GKIT_SYSTEM_PROCESSOR MATCHES "x86")
gkit_set01(GKIT_PROCESSOR_AMD64
    GKIT_SYSTEM_PROCESSOR STREQUAL "amd64")
gkit_set01(GKIT_PROCESSOR_X64
    GKIT_PROCESSOR_X86_64 OR GKIT_PROCESSOR_AMD64)
gkit_set01(GKIT_PROCESSOR_AARCH64
    GKIT_SYSTEM_PROCESSOR STREQUAL "aarch64")
gkit_set01(GKIT_PROCESSOR_ARM64
    GKIT_SYSTEM_PROCESSOR STREQUAL "arm64" OR GKIT_PROCESSOR_AARCH64)
gkit_set01(GKIT_PROCESSOR_ARM32
    GKIT_SYSTEM_PROCESSOR STREQUAL "arm32")
gkit_set01(GKIT_PROCESSOR_ARM
    GKIT_PROCESSOR_AARCH64 OR GKIT_PROCESSOR_ARM64 OR GKIT_PROCESSOR_ARM32)


#-----------------------------------------------------------------------------------------------------------------------
# OpenCTK set cxx compiler variable
#-----------------------------------------------------------------------------------------------------------------------
message(STATUS "Build in cxx compiler: ${CMAKE_CXX_COMPILER_ID}")
set(GKIT_CXX_COMPILER_ID ${CMAKE_CXX_COMPILER_ID})
set(GKIT_CXX_COMPILER_VERSION ${CMAKE_CXX_COMPILER_VERSION})
gkit_set01(GKIT_CXX_COMPILER_GNU
    CMAKE_CXX_COMPILER_ID STREQUAL "GNU")
gkit_set01(GKIT_CXX_COMPILER_MSVC
    MSVC OR CMAKE_CXX_COMPILER_ID STREQUAL "Msvc")
gkit_set01(GKIT_CXX_COMPILER_MINGW
    MINGW OR CMAKE_CXX_COMPILER_ID STREQUAL "Mingw")
gkit_set01(GKIT_CXX_COMPILER_CLANG
    CMAKE_CXX_COMPILER_ID MATCHES "Clang|IntelLLVM")
gkit_set01(GKIT_CXX_COMPILER_APPLE_CLANG
    CMAKE_CXX_COMPILER_ID MATCHES "AppleClang")
gkit_set01(GKIT_CXX_COMPILER_INTEL_LLVM
    CMAKE_CXX_COMPILER_ID STREQUAL "IntelLLVM")
gkit_set01(GKIT_CXX_COMPILER_QCC
    CMAKE_CXX_COMPILER_ID STREQUAL "QCC") # CMP0047

include(CheckLibraryExists)
if(CMAKE_CXX_COMPILER_ID MATCHES "GNU|Clang")
    check_library_exists(stdc++ __cxa_demangle "" GKIT_CXX_COMPILER_USING_LIBSTDCXX)

    if(GKIT_CXX_COMPILER_USING_LIBSTDCXX)
        message(STATUS "Found libstdc++")
    else()
        check_library_exists(c++ __cxa_demangle "" GKIT_CXX_COMPILER_USING_LIBCXX)
        if(GKIT_CXX_COMPILER_USING_LIBCXX)
            message(STATUS "Found libc++")
        endif()
    endif()
endif()


#-----------------------------------------------------------------------------------------------------------------------
# OpenCTK arch size variable
#-----------------------------------------------------------------------------------------------------------------------
if(CMAKE_SIZEOF_VOID_P EQUAL 8)
	set(GKIT_ARCH_BIT 64)
	set(GKIT_ARCH_NAME x64)
    set(GKIT_ARCH_64BIT TRUE)
elseif(CMAKE_SIZEOF_VOID_P EQUAL 4)
	set(GKIT_ARCH_BIT 64)
	set(GKIT_ARCH_NAME x64)
    set(GKIT_ARCH_32BIT TRUE)
endif()


#-----------------------------------------------------------------------------------------------------------------------
# OpenCTK vcpkg triplets variable
#-----------------------------------------------------------------------------------------------------------------------
if(GKIT_PROCESSOR_X86_64 OR GKIT_PROCESSOR_AMD64)
	if (GKIT_ARCH_64BIT)
		set(GKIT_VCPKG_TRIPLET_ARCH x64)
	else()
		set(GKIT_VCPKG_TRIPLET_ARCH x86)
	endif()
    set(GKIT_VCPKG_TRIPLET_ARCH_ARM OFF)
elseif(GKIT_PROCESSOR_I686 OR GKIT_PROCESSOR_I386)
    set(GKIT_VCPKG_TRIPLET_ARCH x86)
    set(GKIT_VCPKG_TRIPLET_ARCH_ARM OFF)
elseif(GKIT_PROCESSOR_ARM64 OR GKIT_PROCESSOR_AARCH64)
    set(GKIT_VCPKG_TRIPLET_ARCH arm64)
    set(GKIT_VCPKG_TRIPLET_ARCH_ARM ON)
elseif(GKIT_PROCESSOR_ARM32)
    set(GKIT_VCPKG_TRIPLET_ARCH arm32)
    set(GKIT_VCPKG_TRIPLET_ARCH_ARM ON)
else()
    message(FATAL_ERROR "Unknown processor arch.")
endif()

if(GKIT_SYSTEM_WIN)
    set(GKIT_VCPKG_TRIPLET_PLATFORM windows)
elseif(GKIT_SYSTEM_IOS)
    set(GKIT_VCPKG_TRIPLET_PLATFORM ios)
elseif(GKIT_SYSTEM_TVOS)
    set(GKIT_VCPKG_TRIPLET_PLATFORM tvos)
elseif(GKIT_SYSTEM_DARWIN)
    set(GKIT_VCPKG_TRIPLET_PLATFORM osx)
elseif(GKIT_SYSTEM_LINUX)
    set(GKIT_VCPKG_TRIPLET_PLATFORM linux)
elseif(GKIT_SYSTEM_ANDROID)
    set(GKIT_VCPKG_TRIPLET_PLATFORM android)
elseif(GKIT_SYSTEM_FREEBSD)
    set(GKIT_VCPKG_TRIPLET_PLATFORM freebsd)
elseif(GKIT_CXX_COMPILER_MINGW)
    set(GKIT_VCPKG_TRIPLET_PLATFORM mingw)
else()
    message(FATAL_ERROR "Unknown system platform.")
endif()
set(GKIT_VCPKG_TRIPLET "${GKIT_VCPKG_TRIPLET_ARCH}-${GKIT_VCPKG_TRIPLET_PLATFORM}" CACHE INTERNAL "" FORCE)
message(STATUS "Vcpkg triplet name: ${GKIT_VCPKG_TRIPLET}")


#-----------------------------------------------------------------------------------------------------------------------
# OpenCTK platform compile arch variable
#-----------------------------------------------------------------------------------------------------------------------
string(TOUPPER "${CMAKE_BUILD_TYPE}" GKIT_UPPER_BUILD_TYPE)
string(TOLOWER "${CMAKE_BUILD_TYPE}" GKIT_LOWER_BUILD_TYPE)
string(TOLOWER "${CMAKE_SYSTEM_NAME}" GKIT_LOWER_SYSTEM_NAME)
string(TOLOWER "${CMAKE_CXX_COMPILER_ID}" GKIT_LOWER_CXX_COMPILER_ID)
string(TOLOWER "${CMAKE_SYSTEM_PROCESSOR}" GKIT_LOWER_SYSTEM_PROCESSOR)
set(GKIT_X64_PROCESSORS "amd64" "x64" "x86_64")
set(GKIT_X86_PROCESSORS "i386" "i686" "x86")
set(GKIT_ARM32_PROCESSORS "arm32" "arm")
set(GKIT_ARM64_PROCESSORS "aarch64" "arm64")
if(GKIT_LOWER_SYSTEM_PROCESSOR IN_LIST GKIT_X64_PROCESSORS)
    set(GKIT_PROCESSOR_MERGE_NAME x64)
elseif(GKIT_LOWER_SYSTEM_PROCESSOR IN_LIST GKIT_X86_PROCESSORS)
    set(GKIT_PROCESSOR_MERGE_NAME x86)
elseif(GKIT_LOWER_SYSTEM_PROCESSOR IN_LIST GKIT_ARM32_PROCESSORS)
    set(GKIT_PROCESSOR_MERGE_NAME arm32)
elseif(GKIT_LOWER_SYSTEM_PROCESSOR IN_LIST GKIT_ARM64_PROCESSORS)
    set(GKIT_PROCESSOR_MERGE_NAME arm64)
else()
    message(FATAL_ERROR "Unknown system processor.")
endif()
set(GKIT_PLATFORM_NAME "${GKIT_LOWER_SYSTEM_NAME}-${GKIT_PROCESSOR_MERGE_NAME}")
set(GKIT_PLATFORM_COMPILER_NAME "${GKIT_PLATFORM_NAME}-${GKIT_LOWER_CXX_COMPILER_ID}")
message(STATUS "Platform name: ${GKIT_PLATFORM_NAME}")
message(STATUS "Platform compiler name: ${GKIT_PLATFORM_COMPILER_NAME}")


#-----------------------------------------------------------------------------------------------------------------------
# OpenCTK build type variable
#-----------------------------------------------------------------------------------------------------------------------
if(CMAKE_BUILD_TYPE MATCHES "Debug")
    set(GKIT_BUILD_DEBUG ON)
else()
    set(GKIT_BUILD_DEBUG OFF)
endif()


#-----------------------------------------------------------------------------------------------------------------------
# OpenCTK arch size variable
#-----------------------------------------------------------------------------------------------------------------------
if(win32)
    set(GKIT_EXECUTABLE_SUFFIX ".exe")
else()
    set(GKIT_EXECUTABLE_SUFFIX "")
endif()


#-----------------------------------------------------------------------------------------------------------------------
# OpenCTK mkspecs version
#-----------------------------------------------------------------------------------------------------------------------
if(GKIT_SYSTEM_WIN32)
    set(GKIT_DEFAULT_PLATFORM_DEFINITIONS WIN32 _ENABLE_EXTENDED_ALIGNED_STORAGE)
    if(GKIT_ARCH_64BIT)
        list(APPEND GKIT_DEFAULT_PLATFORM_DEFINITIONS WIN64 _WIN64)
    endif()
    if(GKIT_CXX_COMPILER_MSVC)
        if(GKIT_CXX_COMPILER_CLANG)
            set(GKIT_DEFAULT_MKSPEC win32-clang-msvc)
        elseif(GKIT_PROCESSOR_ARM64)
            set(GKIT_DEFAULT_MKSPEC win32-arm64-msvc)
        else()
            set(GKIT_DEFAULT_MKSPEC win32-msvc)
        endif()
    elseif(GKIT_CXX_COMPILER_CLANG AND GKIT_CXX_COMPILER_MINGW)
        set(GKIT_DEFAULT_MKSPEC win32-clang-g++)
    elseif(GKIT_CXX_COMPILER_MINGW)
        set(GKIT_DEFAULT_MKSPEC win32-g++)
    endif()

    if(GKIT_CXX_COMPILER_MINGW)
        list(APPEND GKIT_DEFAULT_PLATFORM_DEFINITIONS MINGW_HAS_SECURE_API=1)
    endif()
elseif(GKIT_SYSTEM_LINUX)
    if(GKIT_CXX_COMPILER_GNU)
        set(GKIT_DEFAULT_MKSPEC linux-g++)
    elseif(GKIT_CXX_COMPILER_CLANG)
        set(GKIT_DEFAULT_MKSPEC linux-clang)
    endif()
elseif(GKIT_SYSTEM_ANDROID)
    if(GKIT_CXX_COMPILER_GNU)
        set(GKIT_DEFAULT_MKSPEC android-g++)
    elseif(GKIT_CXX_COMPILER_CLANG)
        set(GKIT_DEFAULT_MKSPEC android-clang)
    endif()
elseif(GKIT_SYSTEM_IOS)
    set(GKIT_DEFAULT_MKSPEC macx-ios-clang)
elseif(GKIT_SYSTEM_APPLE)
    set(GKIT_DEFAULT_MKSPEC macx-clang)
elseif(GKIT_SYSTEM_WASM)
    set(GKIT_DEFAULT_MKSPEC wasm-emscripten)
elseif(GKIT_SYSTEM_QNX)
    # Certain POSIX defines are not set if we don't compile with -std=gnuXX
    set(GKIT_ENABLE_CXX_EXTENSIONS ON)

    list(APPEND GKIT_DEFAULT_PLATFORM_DEFINITIONS _FORTIFY_SOURCE=2 _REENTRANT)

    set(compiler_aarch64le aarch64le)
    set(compiler_armle-v7 armv7le)
    set(compiler_x86-64 x86_64)
    set(compiler_x86 x86)
    foreach(arch aarch64le armle-v7 x86-64 x86)
        if(CMAKE_CXX_COMPILER_TARGET MATCHES "${compiler_${arch}}$")
            set(GKIT_DEFAULT_MKSPEC qnx-${arch}-qcc)
        endif()
    endforeach()
elseif(GKIT_SYSTEM_FREEBSD)
    if(GKIT_CXX_COMPILER_CLANG)
        set(GKIT_DEFAULT_MKSPEC freebsd-clang)
    elseif(GKIT_CXX_COMPILER_GNU)
        set(GKIT_DEFAULT_MKSPEC freebsd-g++)
    endif()
elseif(GKIT_SYSTEM_NETBSD)
    set(GKIT_DEFAULT_MKSPEC netbsd-g++)
elseif(GKIT_SYSTEM_OPENBSD)
    set(GKIT_DEFAULT_MKSPEC openbsd-g++)
elseif(GKIT_SYSTEM_SOLARIS)
    if(GKIT_CXX_COMPILER_GNU)
        if(GKIT_ARCH_64BIT)
            set(GKIT_DEFAULT_MKSPEC solaris-g++-64)
        else()
            set(GKIT_DEFAULT_MKSPEC solaris-g++)
        endif()
    else()
        if(GKIT_ARCH_64BIT)
            set(GKIT_DEFAULT_MKSPEC solaris-cc-64)
        else()
            set(GKIT_DEFAULT_MKSPEC solaris-cc)
        endif()
    endif()
elseif(GKIT_SYSTEM_HURD)
    set(GKIT_DEFAULT_MKSPEC hurd-g++)
endif()

if(NOT GKIT_DEFAULT_MKSPEC)
    message(FATAL_ERROR "mkspec not Detected!")
else()
    message(STATUS "Build in mkspec: ${GKIT_DEFAULT_MKSPEC}")
endif()

if(NOT DEFINED GKIT_DEFAULT_PLATFORM_DEFINITIONS)
    set(GKIT_DEFAULT_PLATFORM_DEFINITIONS "")
endif()

set(GKIT_PLATFORM_DEFINITIONS ${GKIT_DEFAULT_PLATFORM_DEFINITIONS} CACHE STRING "OpenCTK platform specific pre-processor defines")


#-----------------------------------------------------------------------------------------------------------------------
# OpenCTK parse version
#-----------------------------------------------------------------------------------------------------------------------
# Parses a version string like "xx.yy.zz" and sets the major, minor and patch variables.
function(gkit_parse_version_string version_string out_var_prefix)
    string(REPLACE "." ";" version_list ${version_string})
    list(LENGTH version_list length)

    set(out_var "${out_var_prefix}_MAJOR")
    set(value "")
    if(length GREATER 0)
        list(GET version_list 0 value)
        list(REMOVE_AT version_list 0)
        math(EXPR length "${length}-1")
    endif()
    set(${out_var} "${value}" PARENT_SCOPE)

    set(out_var "${out_var_prefix}_MINOR")
    set(value "")
    if(length GREATER 0)
        list(GET version_list 0 value)
        set(${out_var} "${value}" PARENT_SCOPE)
        list(REMOVE_AT version_list 0)
        math(EXPR length "${length}-1")
    endif()
    set(${out_var} "${value}" PARENT_SCOPE)

    set(out_var "${out_var_prefix}_PATCH")
    set(value "")
    if(length GREATER 0)
        list(GET version_list 0 value)
        set(${out_var} "${value}" PARENT_SCOPE)
        list(REMOVE_AT version_list 0)
        math(EXPR length "${length}-1")
    endif()
    set(${out_var} "${value}" PARENT_SCOPE)
endfunction()

# Set up the separate version components for the compiler version, to allow mapping of qmake
# conditions like 'equals(GKIT_GCC_MAJOR_VERSION,5)'.
if(CMAKE_CXX_COMPILER_VERSION)
    gkit_parse_version_string("${CMAKE_CXX_COMPILER_VERSION}" "GKIT_COMPILER_VERSION")
endif()
