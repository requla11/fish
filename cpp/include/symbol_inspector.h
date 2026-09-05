#pragma once

#include "fish_shim.h"
#include <cstdint>
#include <string>
#include <vector>

namespace fish::symbols {

struct SymbolEntry {
    std::string name;
    uint64_t rva;
    bool is_function;
    bool is_exported;
};

struct BinaryMetadata {
    std::string architecture;
    std::vector<std::string> sections;
    std::vector<SymbolEntry> exported_symbols;
    std::vector<std::string> imported_modules;
    std::string pdb_path;
    bool is_valid{false};
};

class SymbolInspector {
public:
    FISH_SHIM_API static BinaryMetadata inspect_binary(const std::string& binary_path);
};

}
