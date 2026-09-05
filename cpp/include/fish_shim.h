#pragma once

#include <string>
#include <string_view>

#ifdef _WIN32
  #ifdef FISH_SHIM_EXPORTS
    #define FISH_SHIM_API __declspec(dllexport)
  #else
    #define FISH_SHIM_API __declspec(dllimport)
  #endif
#else
  #define FISH_SHIM_API __attribute__((visibility("default")))
#endif

namespace fish::shim {

enum class OpType {
    Read,
    Write,
    Execute,
};

FISH_SHIM_API void initialize();
FISH_SHIM_API void shutdown();
FISH_SHIM_API void record_access(OpType op, std::string_view path);

}
