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


message(STATUS "=============================================GenericKit===============================================")
#-----------------------------------------------------------------------------------------------------------------------
# Set cmake config
#-----------------------------------------------------------------------------------------------------------------------
cmake_minimum_required(VERSION 3.22...3.31)
include(.cmake.conf)


#-----------------------------------------------------------------------------------------------------------------------
# Set project name and version
#-----------------------------------------------------------------------------------------------------------------------
# Read version from single source of truth
file(STRINGS "${CMAKE_CURRENT_SOURCE_DIR}/VERSION.txt" _gkit_version_raw LIMIT_COUNT 1 LENGTH_MINIMUM 1)
string(STRIP "${_gkit_version_raw}" _gkit_version_raw)
if(NOT _gkit_version_raw MATCHES "^([0-9]+)\\.([0-9]+)\\.([0-9]+)(\\.([0-9]+))?$")
	message(FATAL_ERROR "Failed to parse VERSION.txt. Expected format: MAJOR.MINOR.PATCH[.TWEAK], got: '${_gkit_version_raw}'")
endif()
set(GKIT_VERSION_MAJOR "${CMAKE_MATCH_1}")
set(GKIT_VERSION_MINOR "${CMAKE_MATCH_2}")
set(GKIT_VERSION_PATCH "${CMAKE_MATCH_3}")
if(CMAKE_MATCH_5)
	set(GKIT_VERSION_TWEAK "${CMAKE_MATCH_5}")
else()
	set(GKIT_VERSION_TWEAK "0")
endif()

project(GenericKit VERSION ${GKIT_VERSION_MAJOR}.${GKIT_VERSION_MINOR}.${GKIT_VERSION_PATCH}.${GKIT_VERSION_TWEAK} LANGUAGES CXX C)
set(GKIT_VERSION_NAME ${PROJECT_VERSION})
set(GKIT_VERSION "${GKIT_VERSION_MAJOR}.${GKIT_VERSION_MINOR}.${GKIT_VERSION_PATCH}")
set(GKIT_VERSION_STR "${GKIT_VERSION}")
set(GKIT_COPYRIGHT "Copyright (C) 2025~Present ChengXueWen.")
set(GKIT_LICENSE "Apache-2.0 license")
set(GKIT_PRODUCT_NAME "GenericKit")
math(EXPR GKIT_VERSION_CALC "${GKIT_VERSION_MAJOR}*1000 + ${GKIT_VERSION_MINOR}*100 + ${GKIT_VERSION_PATCH}")
message(STATUS "Project version: ${GKIT_VERSION} (from VERSION.txt)")

# Sync VERSION.txt → Cargo.toml [workspace.package].version
file(READ "${PROJECT_SOURCE_DIR}/Cargo.toml" _gkit_cargo_toml)
string(REGEX REPLACE "(\\[workspace\\.package\\]\nversion[ \t]*=[ \t]*\")[^\"]*"
	"\\1${GKIT_VERSION_STR}" _gkit_cargo_toml_new "${_gkit_cargo_toml}")
if(NOT "${_gkit_cargo_toml_new}" STREQUAL "${_gkit_cargo_toml}")
	file(WRITE "${PROJECT_SOURCE_DIR}/Cargo.toml" "${_gkit_cargo_toml_new}")
	message(STATUS "Updated Cargo.toml workspace version to ${GKIT_VERSION_STR}")
endif()
message(STATUS "Project copyright: ${GKIT_COPYRIGHT}")
message(STATUS "Project license: ${GKIT_LICENSE}")


#-----------------------------------------------------------------------------------------------------------------------
# Decide whether output should be verbose or not. # Default to verbose (--log-level=STATUS) in a developer-build and
# non-verbose (--log-level=NOTICE) otherwise. If a custom CMAKE_MESSAGE_LOG_LEVEL was specified, it takes priority.
# Passing an explicit --log-level=Foo has the highest priority.
#-----------------------------------------------------------------------------------------------------------------------
if(NOT CMAKE_MESSAGE_LOG_LEVEL)
	if(GKIT_FEATURE_DEV_BUILD)
		set(CMAKE_MESSAGE_LOG_LEVEL "STATUS")
	else()
		set(CMAKE_MESSAGE_LOG_LEVEL "NOTICE")
	endif()
endif()
set(CMAKE_MESSAGE_LOG_LEVEL "DEBUG")


#-----------------------------------------------------------------------------------------------------------------------
# Set build type
#-----------------------------------------------------------------------------------------------------------------------
set(GKIT_DEFAULT_BUILD_TYPE "Release")
if(GKIT_FEATURE_DEV_BUILD)
	set(GKIT_DEFAULT_BUILD_TYPE "Debug")
endif()
if(NOT CMAKE_BUILD_TYPE)
	message(STATUS "Setting build type to '${GKIT_DEFAULT_BUILD_TYPE}' as none was specified.")
	set(CMAKE_BUILD_TYPE "${GKIT_DEFAULT_BUILD_TYPE}" CACHE STRING "Choose the type of build." FORCE)
	set_property(CACHE CMAKE_BUILD_TYPE
		PROPERTY STRINGS
		"Debug" "Release" "MinSizeRel" "RelWithDebInfo") # Set the possible values for cmake-gui.
endif()
message(STATUS "CMAKE_BUILD_TYPE was set to: '${CMAKE_BUILD_TYPE}'")
if(CMAKE_CONFIGURATION_TYPES)
	message(STATUS "CMAKE_CONFIGURATION_TYPES was set to: '${CMAKE_BUILD_TYPE}'")
	set(CMAKE_CONFIGURATION_TYPES "${CMAKE_BUILD_TYPE}" CACHE INTERNAL "Force disable CMAKE_CONFIGURATION_TYPES." FORCE)
endif()

# Override the generic debug postfixes above with custom debug postfixes (even in a single config build) to follow the
# conventions we had. lib/libgkit_core_debug.6.3.0.dylib
if(WIN32)
	if(MINGW)
		# On MinGW we don't have "d" suffix for debug libraries like on Linux, unless we're building debug and release
		# libraries in one go.
		if(GKIT_GENERATOR_IS_MULTI_CONFIG)
			set(CMAKE_DEBUG_POSTFIX "d")
		endif()
	else()
		set(CMAKE_DEBUG_POSTFIX "d")
	endif()
elseif(APPLE)
	set(CMAKE_DEBUG_POSTFIX "_debug")
	set(CMAKE_FRAMEWORK_MULTI_CONFIG_POSTFIX_DEBUG "_debug")
endif()


#-----------------------------------------------------------------------------------------------------------------------
# build directory check
#-----------------------------------------------------------------------------------------------------------------------
if(${PROJECT_SOURCE_DIR} STREQUAL ${CMAKE_BINARY_DIR})
	message(FATAL_ERROR "In-source builds not allowed. Please make a new directory (called a build directory) and run "
		"CMake from there. You may need to remove CMakeCache.txt.")
endif()


#-----------------------------------------------------------------------------------------------------------------------
# top project check
#-----------------------------------------------------------------------------------------------------------------------
if("${PROJECT_SOURCE_DIR}" STREQUAL "${CMAKE_CURRENT_SOURCE_DIR}")
	set(GKIT_PROJECT_IS_TOP_LEVEL ON)
else()
	set(GKIT_PROJECT_IS_TOP_LEVEL OFF)
endif()


#-----------------------------------------------------------------------------------------------------------------------
# Detect prefix and install
#-----------------------------------------------------------------------------------------------------------------------
# Pre-calculate the developer_build feature if it's set by the user via INPUT_GKIT_FEATURE_DEV_BUILD
if(NOT DEFINED GKIT_FEATURE_DEV_BUILD)
	if(NOT DEFINED INPUT_GKIT_FEATURE_DEV_BUILD)
		set(GKIT_FEATURE_DEV_BUILD ON)
	else()
		set(GKIT_FEATURE_DEV_BUILD ${INPUT_GKIT_FEATURE_DEV_BUILD})
	endif()
endif()
message(STATUS "Project feature dev build ${GKIT_FEATURE_DEV_BUILD}")

# Pre-calculate the no_prefix feature if it's set by configure via INPUT_GKIT_FEATURE_NO_PREFIX.
# This needs to be done before configure.cmake is processed.
if(NOT DEFINED GKIT_FEATURE_NO_PREFIX)
	if(NOT DEFINED INPUT_GKIT_FEATURE_NO_PREFIX)
		set(GKIT_FEATURE_NO_PREFIX ON)
	else()
		set(GKIT_FEATURE_NO_PREFIX ${INPUT_GKIT_FEATURE_NO_PREFIX})
	endif()
endif()

set(GKIT_BINARY_DIR ${GenericKit_BINARY_DIR})
# here we specify the installation directory
# Handle both FEATURE_ and GKIT_FEATURE_ cases when they are specified on the command line explicitly.
# It's possible for one to be set, but not the other, because core/configure.cmake is not processed by this point.
if((GKIT_FEATURE_DEV_BUILD OR GKIT_FEATURE_NO_PREFIX) AND NOT CMAKE_STAGING_PREFIX)
	# Handle non-prefix builds by setting the CMake install prefix to point to GenericKit's build dir.
	set(GKIT_DEFAULT_PREFIX "${GKIT_BINARY_DIR}/install")
	if(GKIT_PROJECT_IS_TOP_LEVEL)
		set(CMAKE_INSTALL_PREFIX "${GKIT_DEFAULT_PREFIX}")
	endif()
else()
	if(CMAKE_HOST_WIN32)
		set(GKIT_DEFAULT_PREFIX "C:/GenericKit/")
	else()
		set(GKIT_DEFAULT_PREFIX "/usr/local/")
	endif()
	string(APPEND GKIT_DEFAULT_PREFIX "GenericKit-${GKIT_VERSION}")
endif()
set(GKIT_DEFAULT_PREFIX "${GKIT_BINARY_DIR}/install")
message(STATUS "Project default prefix set as '${GKIT_DEFAULT_PREFIX}'.")

if(NOT DEFINED INPUT_GKIT_FEATURE_INSTALL_PREFIX)
	if(GKIT_FEATURE_DEV_BUILD OR GKIT_FEATURE_NO_PREFIX OR NOT CMAKE_INSTALL_PREFIX)
		set(GKIT_INSTALL_PREFIX "${GKIT_DEFAULT_PREFIX}")
	else()
		set(GKIT_INSTALL_PREFIX "${CMAKE_INSTALL_PREFIX}")
	endif()
else()
	set(GKIT_INSTALL_PREFIX ${INPUT_GKIT_FEATURE_INSTALL_PREFIX})
endif()
message(STATUS "Project install prefix set as '${GKIT_INSTALL_PREFIX}'.")

set(GKIT_INSTALL_DIR "${GKIT_INSTALL_PREFIX}")
set(GKIT_BUILD_DIR "${CMAKE_BINARY_DIR}")
message(STATUS "Project build dir set as '${GKIT_BUILD_DIR}'.")
message(STATUS "Project install dir set as '${GKIT_INSTALL_DIR}'.")

if(NOT "${CMAKE_STAGING_PREFIX}" STREQUAL "")
	set(GKIT_STAGING_PREFIX "${CMAKE_STAGING_PREFIX}")
else()
	set(GKIT_STAGING_PREFIX "${GKIT_DEFAULT_PREFIX}")
endif()
message(STATUS "Project staging prefix set as '${GKIT_DEFAULT_PREFIX}'.")

if(GKIT_INSTALL_PREFIX STREQUAL GKIT_BINARY_DIR)
	set(GKIT_WILL_INSTALL_VALUE OFF)
else()
	set(GKIT_WILL_INSTALL_VALUE ON)
endif()

if(GKIT_FEATURE_DEV_BUILD)
	if(DEFINED GKIT_CMAKE_EXPORT_COMPILE_COMMANDS)
		set(CMAKE_EXPORT_COMPILE_COMMANDS ${GKIT_CMAKE_EXPORT_COMPILE_COMMANDS})
	else()
		set(CMAKE_EXPORT_COMPILE_COMMANDS ON)
	endif()
	set(GKIT_DEFAULT_BUILD_WARNINGS_ARE_ERRORS ON)
	set(GKIT_DEFAULT_BUILD_TESTS ON)
	set(GKIT_DEFAULT_BUILD_BENCHMARKS ON)

	# Tests are not built by default with qmake for iOS and friends, and thus the overall build
	# tends to fail. Disable them by default when targeting uikit.
	if(UIKIT OR ANDROID)
		set(GKIT_DEFAULT_BUILD_TESTS OFF)
	endif()

	# Disable benchmarks for single configuration generators which do not build
	# with release configuration.
	if(CMAKE_BUILD_TYPE AND CMAKE_BUILD_TYPE STREQUAL Debug)
		set(GKIT_DEFAULT_BUILD_BENCHMARKS OFF)
	endif()
else()
	set(GKIT_DEFAULT_BUILD_WARNINGS_ARE_ERRORS OFF)
	set(GKIT_DEFAULT_BUILD_TESTS OFF)
	set(GKIT_DEFAULT_BUILD_BENCHMARKS OFF)
endif()
message(STATUS "GenericKit cmake install prefix set as '${CMAKE_INSTALL_PREFIX}'.")


#-----------------------------------------------------------------------------------------------------------------------
# Set Position independent
#-----------------------------------------------------------------------------------------------------------------------
## Position independent code: PIC
set(CMAKE_POSITION_INDEPENDENT_CODE ON)
# Does the linker support position independent code? 
include(CheckPIESupported)
check_pie_supported()
# Do not relink dependent libraries when no header has changed:
set(CMAKE_LINK_DEPENDS_NO_SHARED ON)


#-----------------------------------------------------------------------------------------------------------------------
# Specify the C++ standard and flags
#-----------------------------------------------------------------------------------------------------------------------
if(CMAKE_CXX_STANDARD)
	set(GKIT_CMAKE_CXX_STANDARD ${CMAKE_CXX_STANDARD} CACHE STRING "C++ standard" FORCE)
else()
	set(GKIT_CMAKE_CXX_STANDARD 11 CACHE STRING "C++ standard")
endif()
set(GKIT_CMAKE_CXX_STANDARD_VALID OFF)
set(GKIT_CMAKE_CXX_STANDARD_VALUES 11 14 17 20 23 26)
foreach(cxx_standard ${GKIT_CMAKE_CXX_STANDARD_VALUES})
	if("${GKIT_CMAKE_CXX_STANDARD}" VERSION_GREATER_EQUAL "${cxx_standard}")
		set(GKIT_CMAKE_CXX_STANDARD_${cxx_standard} 1)
	else()
		set(GKIT_CMAKE_CXX_STANDARD_${cxx_standard} 0)
	endif()
	if("${cxx_standard}" STREQUAL "${GKIT_CMAKE_CXX_STANDARD}")
		set(GKIT_CMAKE_CXX_STANDARD_VALID ON)
	endif()
endforeach()
if(NOT GKIT_CMAKE_CXX_STANDARD_VALID)
	message(FATAL_ERROR " Unsupport C++ standard: ${GKIT_CMAKE_CXX_STANDARD}.")
endif()
set(CMAKE_CXX_STANDARD_REQUIRED ON)
message(STATUS "Build type: ${CMAKE_BUILD_TYPE}")
message(STATUS "Using C++: ${GKIT_CMAKE_CXX_STANDARD}")


#-----------------------------------------------------------------------------------------------------------------------
# Check the C++ compiler version
#-----------------------------------------------------------------------------------------------------------------------
set(GKIT_MSVC_MIN_VERSION 16.0)
set(GKIT_GCC_MIN_VERSION 4.7)
if(CMAKE_CXX_COMPILER_ID STREQUAL "MSVC")
	if(CMAKE_CXX_COMPILER_VERSION VERSION_LESS ${GKIT_MSVC_MIN_VERSION})
		message(FATAL_ERROR "MSVC too old: ${CMAKE_CXX_COMPILER_VERSION}. Need >= ${GKIT_MSVC_MIN_VERSION}")
	endif()
elseif(CMAKE_CXX_COMPILER_ID STREQUAL "GNU")
	if(CMAKE_CXX_COMPILER_VERSION VERSION_LESS ${GKIT_GCC_MIN_VERSION})
		message(FATAL_ERROR "GCC too old: ${CMAKE_CXX_COMPILER_VERSION}. Need >= ${GKIT_GCC_MIN_VERSION}")
	endif()
endif()


#-----------------------------------------------------------------------------------------------------------------------
# Set CMAKE_OBJECT_PATH_MAX, avoid The maximum full path to an object file warning.
#-----------------------------------------------------------------------------------------------------------------------
set(CMAKE_OBJECT_PATH_MAX 1024)


#-----------------------------------------------------------------------------------------------------------------------
# Fetch processor count
#-----------------------------------------------------------------------------------------------------------------------
include(ProcessorCount)
processorcount(GKIT_PROCESSOR_COUNT)
set(GKIT_NUMBER_OF_ASYNC_JOBS ${GKIT_PROCESSOR_COUNT})


#-----------------------------------------------------------------------------------------------------------------------
# Add CMake function(s) and macro(s)
#-----------------------------------------------------------------------------------------------------------------------
set(GKIT_CMAKE_DIR "${PROJECT_SOURCE_DIR}/cmake")
set(CMAKE_MODULE_PATH ${GKIT_CMAKE_DIR} ${CMAKE_MODULE_PATH})
include(GKitCMakeHelpers)
include(GKitCargoHelpers)
include(GKitOptionHelpers)
include(GKitInstallHelpers)
include(GKitPlatformHelpers)
include(GKitFindPackageHelpers)
include(GKitCargoPlugin)


#-----------------------------------------------------------------------------------------------------------------------
# Set build options
#-----------------------------------------------------------------------------------------------------------------------
gkit_option(GKIT_BUILD_ALL "Enable this to build all artifacts" OFF)
gkit_option(GKIT_BUILD_SHARED_LIBS "Enable this to build as dynamically" ON
	SET BUILD_SHARED_LIBS)
gkit_option(GKIT_BUILD_USE_PCH "Enable this to build use precompiled header files for compilation" ${BUILD_SHARED_LIBS}
	DEPENDS BUILD_SHARED_LIBS)
gkit_option(GKIT_BUILD_COMPILER_WARNING "Enable this to build target with compiler warnings" OFF)
gkit_option(GKIT_BUILD_WARNINGS_ARE_ERRORS "Enable this to build with warnings as errors"
	${GKIT_DEFAULT_BUILD_WARNINGS_ARE_ERRORS})
gkit_option(GKIT_BUILD_BENCHMARKS "Enable this to build the benchmarks" ${GKIT_DEFAULT_BUILD_BENCHMARKS}
	SET GKIT_DEFAULT_BUILD_TESTS)
gkit_option(GKIT_BUILD_DOCS "Enable this to build the documentation" OFF)
gkit_option(GKIT_BUILD_APPS "Enable this to build apps" ON
	OR_CONDITION GKIT_BUILD_ALL)
gkit_option(GKIT_BUILD_TESTS "Enable this to build tests" ${GKIT_DEFAULT_BUILD_TESTS}
	OR_CONDITION GKIT_BUILD_ALL)
gkit_option(GKIT_BUILD_EXAMPLES "Enable this to build examples" ON
	OR_CONDITION GKIT_BUILD_ALL)
gkit_option(GKIT_BUILD_INSTALL "Enable this to build the installer" ${GKIT_WILL_INSTALL_VALUE}
	OR_CONDITION GKIT_BUILD_ALL)

gkit_option(GKIT_BUILD_CRATE_FFI "Enable this to build C FFI bindings" ON
	OR_CONDITION GKIT_BUILD_ALL)
gkit_option(GKIT_BUILD_CRATE_WASM "Enable this to build WebAssembly bindings" OFF
	OR_CONDITION GKIT_BUILD_ALL)
gkit_option(GKIT_BUILD_CRATE_UNIFFI "Enable this to build UniFFI bindings (Kotlin, Swift, Python, Ruby)" OFF
	OR_CONDITION GKIT_BUILD_ALL)

gkit_option(GKIT_BUILD_PACKAGE_CPP "Enable this to build C++ packages" ON
	DEPENDS GKIT_BUILD_CRATE_FFI
	OR_CONDITION GKIT_BUILD_ALL)

# WebRTC backend selection
if(CMAKE_SYSTEM_PROCESSOR MATCHES "wasm32" OR EMSCRIPTEN)
	set(_gkit_feature_media_webrtc_backend_default "wasm")
else()
	set(_gkit_feature_media_webrtc_backend_default "webrtc-rs")
endif()
set(GKIT_FEATURE_MEDIA_WEBRTC_BACKEND "${_gkit_feature_media_webrtc_backend_default}" CACHE STRING "WebRTC backend: webrtc-rs, google, or wasm")
set_property(CACHE GKIT_FEATURE_MEDIA_WEBRTC_BACKEND PROPERTY STRINGS "webrtc-rs" "google" "wasm")
if(NOT GKIT_FEATURE_MEDIA_WEBRTC_BACKEND MATCHES "^(webrtc-rs|google|wasm)$")
	message(FATAL_ERROR "GKIT_FEATURE_MEDIA_WEBRTC_BACKEND must be one of: webrtc-rs, google, wasm (got: ${GKIT_FEATURE_MEDIA_WEBRTC_BACKEND})")
endif()
if(CMAKE_SYSTEM_PROCESSOR MATCHES "wasm32" OR EMSCRIPTEN)
	if(NOT GKIT_FEATURE_MEDIA_WEBRTC_BACKEND STREQUAL "wasm")
		message(FATAL_ERROR "GKIT_FEATURE_MEDIA_WEBRTC_BACKEND=wasm is required for wasm32 targets")
	endif()
endif()


#-----------------------------------------------------------------------------------------------------------------------
# Set install and paths
#-----------------------------------------------------------------------------------------------------------------------
# Install locations:
gkit_configure_process_path(GKIT_INSTALL_LIBDIR "lib" "Libraries [PREFIX/lib]")
gkit_configure_process_path(GKIT_INSTALL_BINDIR "bin" "Executables [PREFIX/bin]")
gkit_configure_process_path(GKIT_INSTALL_ARCHDATADIR "." "Arch-dependent data [PREFIX]")
gkit_configure_process_path(GKIT_INSTALL_INCLUDEDIR "include" "Header files [PREFIX/include]")

if(WIN32)
	set(GKIT_DEFAULT_DLLDIR "bin")
	set(GKIT_DEFAULT_TESTSDIR "bin")
	set(GKIT_DEFAULT_EXAMPLESDIR "bin")
	set(GKIT_DEFAULT_LIBEXEC "${GKIT_INSTALL_ARCHDATADIR}/bin")
else()
	set(GKIT_DEFAULT_DLLDIR "lib")
	set(GKIT_DEFAULT_TESTSDIR "tests")
	set(GKIT_DEFAULT_EXAMPLESDIR "examples")
	set(GKIT_DEFAULT_LIBEXEC "${GKIT_INSTALL_ARCHDATADIR}/libexec")
endif()

gkit_configure_process_path(GKIT_INSTALL_DLLDIR
	"${GKIT_DEFAULT_DLLDIR}" "[PREFIX/bin] on Windows, [PREFIX/lib] otherwise")
gkit_configure_process_path(GKIT_INSTALL_TESTSDIR
	"${GKIT_DEFAULT_TESTSDIR}" "[PREFIX/bin] on Windows, [PREFIX/tests] otherwise")
gkit_configure_process_path(GKIT_INSTALL_EXAMPLESDIR
	"${GKIT_DEFAULT_EXAMPLESDIR}" "[PREFIX/bin] on Windows, [PREFIX/examples] otherwise")
gkit_configure_process_path(GKIT_INSTALL_LIBEXECDIR
	"${GKIT_DEFAULT_LIBEXEC}" "Helper programs [ARCHDATADIR/bin on Windows, ARCHDATADIR/libexec otherwise]")
gkit_configure_process_path(GKIT_INSTALL_DATADIR "." "Arch-independent data [PREFIX]")
gkit_configure_process_path(GKIT_INSTALL_DOCDIR "${GKIT_INSTALL_DATADIR}/doc" "Documentation [DATADIR/doc]")

if(APPLE)
	set(GKIT_DEFAULT_SYS_CONF_DIR "/Library/Preferences/GenericKit")
else()
	set(GKIT_DEFAULT_SYS_CONF_DIR "etc/xdg")
endif()
gkit_configure_process_path(GKIT_INSTALL_SYSCONFDIR
	"${GKIT_DEFAULT_SYS_CONF_DIR}" "Settings used by GenericKit programs [PREFIX/etc/xdg]/[/Library/Preferences/GenericKit]")
gkit_configure_process_path(GKIT_INSTALL_DESCRIPTIONSDIR
	"${GKIT_INSTALL_DATADIR}/modules" "Module description files directory")

# set CONFIG_INSTALL_DIR and CONFIG_BUILD_DIR
set(GKIT_CONFIG_BUILD_DIR "${GKIT_BUILD_DIR}/${GKIT_INSTALL_LIBDIR}/cmake")
# set(GKIT_CONFIG_INSTALL_DIR "${GKIT_INSTALL_DIR}")
if(GKIT_CONFIG_INSTALL_DIR)
    string(APPEND GKIT_CONFIG_INSTALL_DIR "/")
endif()
string(APPEND GKIT_CONFIG_INSTALL_DIR "${GKIT_INSTALL_LIBDIR}/cmake")

# the default RPATH to be used when installing, but only if it's not a system directory
list(FIND CMAKE_PLATFORM_IMPLICIT_LINK_DIRECTORIES "${CMAKE_INSTALL_PREFIX}/${GKIT_INSTALL_LIBDIR}" isSystemDir)
if("${isSystemDir}" STREQUAL "-1")
	set(_default_install_rpath "${CMAKE_INSTALL_PREFIX}/${GKIT_INSTALL_LIBDIR}")
endif("${isSystemDir}" STREQUAL "-1")

# The default rpath settings for installed targets is empty.
# The rpaths will instead be computed for each target separately using gkit_apply_rpaths().
# Additional rpaths can be passed via GKIT_EXTRA_RPATHS.
# By default this will include $ORIGIN / @loader_path, so the installation is relocatable.
# Bottom line: No need to pass anything to CMAKE_INSTALL_RPATH.
set(CMAKE_INSTALL_RPATH "" CACHE STRING "RPATH for installed binaries")

# By default, don't embed auto-determined RPATHs pointing to directories
# outside of the build tree, into the installed binaries.
# This ended up adding rpaths like ${CMAKE_INSTALL_PREFIX}/lib (or /Users/gkit/work/install/lib into
# the official libraries created by the CI) into the non-gkitbase libraries, plugins, etc.
#
# It should not be necessary, given that gkit_apply_rpaths() already adds the necessary rpaths, either
# relocatable ones or absolute ones, depending on what the platform supports.
if(NOT GKIT_NO_DISABLE_CMAKE_INSTALL_RPATH_USE_LINK_PATH)
	set(CMAKE_INSTALL_RPATH_USE_LINK_PATH FALSE)
endif()

# Ensure that GNUInstallDirs's CMAKE_INSTALL_LIBDIR points to the same lib dir that GenericKit was configured with.
set(CMAKE_INSTALL_LIBDIR "${GKIT_INSTALL_LIBDIR}")

function(gkit_setup_tool_path_command)
	if(NOT CMAKE_HOST_WIN32)
		return()
	endif()
	set(bindir "${GKIT_BUILD_INTERNALS_RELOCATABLE_INSTALL_PREFIX}/${GKIT_INSTALL_BINDIR}")
	file(TO_NATIVE_PATH "${bindir}" bindir)
	list(APPEND command COMMAND)
	list(APPEND command set PATH=${bindir}$<SEMICOLON>%PATH%)
	set(GKIT_TOOL_PATH_SETUP_COMMAND "${command}" CACHE INTERNAL "internal command prefix for tool invocations" FORCE)
	# GKIT_TOOL_PATH_SETUP_COMMAND is deprecated. Please use _gkit_internal_get_wrap_tool_script_path instead.
endfunction()
gkit_setup_tool_path_command()

file(MAKE_DIRECTORY "${GKIT_BUILD_DIR}/${GKIT_INSTALL_LIBDIR}")
file(MAKE_DIRECTORY "${GKIT_BUILD_DIR}/${GKIT_INSTALL_BINDIR}")
file(MAKE_DIRECTORY "${GKIT_BUILD_DIR}/${GKIT_INSTALL_DLLDIR}")
file(MAKE_DIRECTORY "${GKIT_BUILD_DIR}/${GKIT_INSTALL_LIBEXECDIR}")

if(CMAKE_HOST_WIN32)
	set(CMAKE_RUNTIME_OUTPUT_DIRECTORY_DEBUG "${GKIT_BUILD_DIR}/${GKIT_INSTALL_BINDIR}")
	set(CMAKE_LIBRARY_OUTPUT_DIRECTORY_DEBUG "${GKIT_BUILD_DIR}/${GKIT_INSTALL_LIBDIR}")
	set(CMAKE_ARCHIVE_OUTPUT_DIRECTORY_DEBUG "${GKIT_BUILD_DIR}/${GKIT_INSTALL_LIBDIR}")
	set(CMAKE_RUNTIME_OUTPUT_DIRECTORY_RELEASE "${GKIT_BUILD_DIR}/${GKIT_INSTALL_BINDIR}")
	set(CMAKE_LIBRARY_OUTPUT_DIRECTORY_RELEASE "${GKIT_BUILD_DIR}/${GKIT_INSTALL_LIBDIR}")
	set(CMAKE_ARCHIVE_OUTPUT_DIRECTORY_RELEASE "${GKIT_BUILD_DIR}/${GKIT_INSTALL_LIBDIR}")
	set(CMAKE_RUNTIME_OUTPUT_DIRECTORY_RELWITHDEBINFO "${GKIT_BUILD_DIR}/${GKIT_INSTALL_BINDIR}")
	set(CMAKE_LIBRARY_OUTPUT_DIRECTORY_RELWITHDEBINFO "${GKIT_BUILD_DIR}/${GKIT_INSTALL_LIBDIR}")
	set(CMAKE_ARCHIVE_OUTPUT_DIRECTORY_RELWITHDEBINFO "${GKIT_BUILD_DIR}/${GKIT_INSTALL_LIBDIR}")
endif()


#-----------------------------------------------------------------------------------------------------------------------
# Set Platform and InternalTargets define
#-----------------------------------------------------------------------------------------------------------------------
#include(GenericKitPlatformSupport)
#include(GenericKitInternalTargets)


#-----------------------------------------------------------------------------------------------------------------------
# Set namespace and separator
#-----------------------------------------------------------------------------------------------------------------------
set(GKIT_NAMESPACE_VERSION ${GKIT_VERSION_MAJOR})
set(GKIT_NAMESPACE "GenericKit" CACHE INTERNAL "GenericKit Namespace" FORCE)
if(NOT GKIT_CMAKE_INSTALL_NAMESPACE)
	set(GKIT_CMAKE_INSTALL_NAMESPACE "${GKIT_NAMESPACE}${GKIT_NAMESPACE_VERSION}"
		CACHE STRING "CMake namespace [${GKIT_NAMESPACE}${GKIT_NAMESPACE_VERSION}]")
endif()
if(NOT GKIT_CMAKE_EXPORT_NAMESPACE)
	set(GKIT_CMAKE_EXPORT_NAMESPACE "${GKIT_NAMESPACE}${GKIT_NAMESPACE_VERSION}"
		CACHE STRING "CMake namespace used when exporting targets [${GKIT_NAMESPACE}${GKIT_NAMESPACE_VERSION}]")
endif()

set(GKIT_KNOWN_LIBRARIES_WITH_TOOLS "" CACHE INTERNAL "Known GenericKit modules with tools" FORCE)

# For adjusting variables when running tests, we need to know what
# the correct variable is for separating entries in PATH-alike
# variables.
if(CMAKE_HOST_SYSTEM_NAME STREQUAL "Windows")
	set(GKIT_PATH_SEPARATOR "\\;")
else()
	set(GKIT_PATH_SEPARATOR ":")
endif()

# Save the value of the current first project source dir.
# This will be /path/to/GKITCore for GKITCore both in a super-build and a non super-build.
# This will be /path/to/GKITCore/tests when building standalone tests.
set(GKIT_TOP_LEVEL_SOURCE_DIR "${CMAKE_CURRENT_SOURCE_DIR}" CACHE INTERNAL "GenericKit top level source dir" FORCE)
set(GKIT_INSTALL_COMPONENT_NAME "${GKIT_CMAKE_INSTALL_NAMESPACE}sdk")


#-----------------------------------------------------------------------------------------------------------------------
# Prevent warnings about object files without any symbols. This is a common thing in GenericKit as we tend to build files
# unconditionally, and then use ifdefs to compile out parts that are not relevant.
#-----------------------------------------------------------------------------------------------------------------------
if(CMAKE_HOST_APPLE AND APPLE)
	foreach(lang ASM C CXX)
		# We have to tell 'ar' to not run ranlib by itself, by passing the 'S' option
		set(CMAKE_${lang}_ARCHIVE_CREATE "<CMAKE_AR> qcS <TARGET> <LINK_FLAGS> <OBJECTS>")
		set(CMAKE_${lang}_ARCHIVE_APPEND "<CMAKE_AR> qS <TARGET> <LINK_FLAGS> <OBJECTS>")
		set(CMAKE_${lang}_ARCHIVE_FINISH "<CMAKE_RANLIB> -no_warning_for_no_symbols <TARGET>")
	endforeach()
endif()


#-----------------------------------------------------------------------------------------------------------------------
# Output static runtime info
#-----------------------------------------------------------------------------------------------------------------------
if(GKIT_MSVC_STATIC_RUNTIME)
	message(STATUS "Build as static runtime")
endif()


#-----------------------------------------------------------------------------------------------------------------------
# Set custom build target
#-----------------------------------------------------------------------------------------------------------------------
if(GKIT_PROJECT_IS_TOP_LEVEL)
	add_custom_target(gkit-build
		COMMAND ${CMAKE_COMMAND} --build ./ --parallel ${GKIT_NUMBER_OF_ASYNC_JOBS} --config ${CMAKE_BUILD_TYPE}
		WORKING_DIRECTORY "${GKIT_BUILD_DIR}")
	add_custom_target(gkit-build-install
		COMMAND ${CMAKE_COMMAND} --build ./ --parallel ${GKIT_NUMBER_OF_ASYNC_JOBS} --config ${CMAKE_BUILD_TYPE}
		COMMAND ${CMAKE_COMMAND} --install ./
		WORKING_DIRECTORY "${GKIT_BUILD_DIR}")
endif()


#-----------------------------------------------------------------------------------------------------------------------
# Set default param args
#-----------------------------------------------------------------------------------------------------------------------
set(GKIT_DEFAULT_PRIVATE_ARGS
	DEFINES
	SOURCES
	LIBRARIES
	INCLUDE_DIRECTORIES
	FEATURE_DEPENDENCIES
	COMPILE_OPTIONS
	LINK_OPTIONS
	DISABLE_AUTOGEN_TOOLS
	ENABLE_AUTOGEN_TOOLS
	PLUGIN_TYPES)
set(GKIT_DEFAULT_PUBLIC_ARGS
	PUBLIC_DEFINES
	PUBLIC_LIBRARIES
	PUBLIC_INCLUDE_DIRECTORIES
	PUBLIC_COMPILE_OPTIONS
	PUBLIC_LINK_OPTIONS)
set(GKIT_DEFAULT_PRIVATE_LIBRARY_ARGS
	PRIVATE_LIBRARY_INTERFACE)
set(GKIT_DEFAULT_TARGET_INFO_ARGS
	TARGET_VERSION
	TARGET_PRODUCT
	TARGET_DESCRIPTION
	TARGET_COMPANY
	TARGET_COPYRIGHT)

# Collection of arguments so they can be shared across gkit_internal_add_executable and gkit_internal_add_test_helper.
set(GKIT_INTERNAL_ADD_EXECUTABLE_OPTIONAL_ARGS
	GUI
	INSTALL
	EXCEPTIONS
	DELAY_RC
	DELAY_TARGET_INFO
	GKIT_APP)
set(GKIT_INTERNAL_ADD_EXECUTABLE_SINGLE_ARGS
	CORE_LIBRARY
	OUTPUT_SUBDIR
	OUTPUT_DIRECTORY
	INSTALL_DIRECTORY
	VERSION
	${GKIT_DEFAULT_TARGET_INFO_ARGS})
set(GKIT_INTERNAL_ADD_EXECUTABLE_MULTI_ARGS
	${GKIT_DEFAULT_PRIVATE_ARGS}
	${GKIT_DEFAULT_PUBLIC_ARGS})


#-----------------------------------------------------------------------------------------------------------------------
# add 3rdparty subdirectory
#-----------------------------------------------------------------------------------------------------------------------
#include(InstallPython)
#include(InstallPkgconf)
include(InstallCorrosion)
include(InstallVcpkg)
include(InstallRequiredSystemLibraries)

if(GKIT_BUILD_DOCS)
	include(InstallDoxygen)
endif()
if(GKIT_BUILD_TESTS)
	include(CTest)
	enable_testing()
	gkit_find_package(WrapUnity PROVIDED_TARGETS GKitWrapUnity::WrapUnity)
	gkit_find_package(WrapGTest PROVIDED_TARGETS GKitWrapGTest::WrapGTest)
	gkit_find_package(WrapBenchmark PROVIDED_TARGETS GKitWrapBenchmark::WrapBenchmark)

	find_program(GKIT_CARGO_EXECUTABLE cargo REQUIRED)
	add_test(NAME gkit-media-tests-native
		COMMAND "${GKIT_CARGO_EXECUTABLE}" test -p gkit-media --features backend-native
		WORKING_DIRECTORY "${PROJECT_SOURCE_DIR}")
endif()


#-----------------------------------------------------------------------------------------------------------------------
# add subdirectory
#-----------------------------------------------------------------------------------------------------------------------
set(_gkit_corrosion_crates gkit-core gkit-media gkit-native gkit-crash gkit-network gkit-service)
if(GKIT_BUILD_CRATE_FFI)
	list(APPEND _gkit_corrosion_crates gkit-core-c gkit-media-c)
endif()
list(APPEND _gkit_corrosion_crates gkit-core-uniffi gkit-media-uniffi)

# --- Plugin crates ---
gkit_cargo_add_plugin(
    NAME gkit-plugin-webrtc-libwebrtc
    CATEGORY webrtc
    PLATFORMS macos;linux;windows
)
list(APPEND _gkit_corrosion_crates ${_GKIT_CARGO_PLUGIN_LIST})

# WASM plugin: rlib crate (static linking, not cdylib)
set(_gkit_wasm_plugin_crates gkit-plugin-webrtc-web-sys)
list(APPEND _gkit_corrosion_crates ${_gkit_wasm_plugin_crates})

corrosion_import_crate(MANIFEST_PATH Cargo.toml CRATES ${_gkit_corrosion_crates})

gkit_cargo_setup_plugins()

# WASM rlib plugins: no post-build copy needed, just IDE folder
if(TARGET gkit_plugin_webrtc_web_sys)
    get_target_property(_wstype gkit_plugin_webrtc_web_sys TYPE)
    if(NOT _wstype STREQUAL "INTERFACE_LIBRARY")
        set_target_properties(gkit_plugin_webrtc_web_sys PROPERTIES
            FOLDER "gkit-media/plugins/webrtc")
    endif()
endif()

# Direct internal/binding crate output to a cargo subdir to avoid cluttering build/
set(_gkit_redirect_crates gkit_core gkit_media gkit_native gkit_crash gkit_network gkit_service)
list(APPEND _gkit_redirect_crates gkit-core-uniffi gkit-media-uniffi)
foreach(_gkit_crate ${_gkit_redirect_crates})
	if(TARGET ${_gkit_crate})
		set_target_properties(${_gkit_crate} PROPERTIES
			ARCHIVE_OUTPUT_DIRECTORY "${GKIT_BUILD_DIR}/cargo/${_gkit_crate}")
		if(WIN32)
			set_target_properties(${_gkit_crate} PROPERTIES
				RUNTIME_OUTPUT_DIRECTORY "${GKIT_BUILD_DIR}/cargo/${_gkit_crate}")
		else()
			set_target_properties(${_gkit_crate} PROPERTIES
				LIBRARY_OUTPUT_DIRECTORY "${GKIT_BUILD_DIR}/cargo/${_gkit_crate}")
		endif()
	endif()
endforeach()

# organize corrosion targets into IDE folder tree
gkit_cargo_set_folder("gkit-core"    "gkit-core")
gkit_cargo_set_folder("gkit-media"   "gkit-media")
gkit_cargo_set_folder("gkit-native"  "gkit-native")
gkit_cargo_set_folder("gkit-crash"   "gkit-crash")
gkit_cargo_set_folder("gkit-network" "gkit-network")
gkit_cargo_set_folder("gkit-service" "gkit-service")
if(GKIT_BUILD_CRATE_FFI)
	gkit_cargo_set_folder("gkit-core-c"  "gkit-core-ffi")
	gkit_cargo_set_folder("gkit-media-c" "gkit-media-ffi")
endif()
gkit_cargo_set_folder("gkit-core-uniffi" "gkit-core-uniffi")
gkit_cargo_set_folder("gkit-media-uniffi" "gkit-media-uniffi")

# WASM binding targets: build with cargo + wasm-bindgen-cli (not Corrosion)
if(GKIT_BUILD_CRATE_WASM)
    gkit_cargo_setup_wasm_target(gkit-core-wasm)
    gkit_cargo_setup_wasm_target(gkit-media-wasm)
    add_subdirectory(crates/gkit-media-wasm/examples/webrtc-loopback)
    add_subdirectory(crates/gkit-media-wasm/examples/square-gen)
endif()

# add API subdirectories (conditional on build options)
if(GKIT_BUILD_CRATE_FFI)
	add_subdirectory(crates/gkit-media-ffi)
	add_subdirectory(crates/gkit-core-ffi)
endif()
if(GKIT_BUILD_PACKAGE_CPP)
	add_subdirectory(packages/cpp)
endif()
#add_subdirectory(src/tools)
#add_subdirectory(src/libs)

# Rust examples
if(GKIT_BUILD_EXAMPLES)
	add_subdirectory(crates/gkit-media/examples/gkit-media-viewer)
	add_subdirectory(crates/gkit-media/examples/gkit-media-square-gen)
	add_subdirectory(crates/gkit-media/examples/gkit-media-webrtc-loopback)
endif()

# Rust tools
add_subdirectory(tools/gkit-vcpkg)