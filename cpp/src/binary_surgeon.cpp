#include "binary_surgeon.h"
#include <cstring>
#include <fstream>

namespace fish::surgeon {

std::vector<uint8_t> BinarySurgeon::read_bytes(
    const std::string& binary_path,
    uint64_t offset,
    size_t size
) {
    std::vector<uint8_t> result;
    std::ifstream file(binary_path, std::ios::binary);
    if (!file.is_open()) {
        return result;
    }

    file.seekg(static_cast<std::streamoff>(offset), std::ios::beg);
    result.resize(size);
    if (!file.read(reinterpret_cast<char*>(result.data()), static_cast<std::streamsize>(size))) {
        result.clear();
    }
    return result;
}

bool BinarySurgeon::apply_patch(const std::string& binary_path, const PatchEntry& patch) {
    if (patch.new_bytes.empty()) {
        return false;
    }

    std::fstream file(binary_path, std::ios::binary | std::ios::in | std::ios::out);
    if (!file.is_open()) {
        return false;
    }

    if (!patch.original_bytes.empty()) {
        file.seekg(static_cast<std::streamoff>(patch.file_offset), std::ios::beg);
        std::vector<uint8_t> current(patch.original_bytes.size());
        if (!file.read(reinterpret_cast<char*>(current.data()), static_cast<std::streamsize>(current.size()))) {
            return false;
        }
        if (std::memcmp(current.data(), patch.original_bytes.data(), current.size()) != 0) {
            return false;
        }
    }

    file.seekp(static_cast<std::streamoff>(patch.file_offset), std::ios::beg);
    if (!file.write(reinterpret_cast<const char*>(patch.new_bytes.data()), static_cast<std::streamsize>(patch.new_bytes.size()))) {
        return false;
    }

    file.flush();
    return true;
}

bool BinarySurgeon::apply_trampoline_rel32(
    const std::string& binary_path,
    uint64_t target_offset,
    int32_t relative_displacement
) {
    std::vector<uint8_t> jmp_instruction(5, 0);
    jmp_instruction[0] = 0xE9;
    std::memcpy(&jmp_instruction[1], &relative_displacement, sizeof(relative_displacement));

    PatchEntry patch;
    patch.file_offset = target_offset;
    patch.new_bytes = std::move(jmp_instruction);

    return apply_patch(binary_path, patch);
}

}
