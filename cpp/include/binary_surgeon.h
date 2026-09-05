#pragma once

#include "fish_shim.h"
#include <cstdint>
#include <string>
#include <vector>

namespace fish::surgeon {

struct PatchEntry {
    uint64_t file_offset;
    std::vector<uint8_t> original_bytes;
    std::vector<uint8_t> new_bytes;
};

class BinarySurgeon {
public:
    FISH_SHIM_API static bool apply_patch(const std::string& binary_path, const PatchEntry& patch);
    FISH_SHIM_API static bool apply_trampoline_rel32(
        const std::string& binary_path,
        uint64_t target_offset,
        int32_t relative_displacement
    );
    FISH_SHIM_API static std::vector<uint8_t> read_bytes(
        const std::string& binary_path,
        uint64_t offset,
        size_t size
    );
};

}
