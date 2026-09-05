#include "symbol_inspector.h"
#include <cstring>
#include <fstream>
#include <vector>

#ifdef _WIN32
#include <windows.h>
#endif

namespace fish::symbols {

BinaryMetadata SymbolInspector::inspect_binary(const std::string& binary_path) {
    BinaryMetadata meta;
    std::ifstream file(binary_path, std::ios::binary | std::ios::ate);
    if (!file.is_open()) {
        return meta;
    }

    std::streamsize file_size = file.tellg();
    if (file_size < 512) {
        return meta;
    }

    file.seekg(0, std::ios::beg);
    std::vector<uint8_t> buffer(static_cast<size_t>(file_size));
    if (!file.read(reinterpret_cast<char*>(buffer.data()), file_size)) {
        return meta;
    }

#ifdef _WIN32
    if (buffer.size() < sizeof(IMAGE_DOS_HEADER)) {
        return meta;
    }

    auto* dos_header = reinterpret_cast<IMAGE_DOS_HEADER*>(buffer.data());
    if (dos_header->e_magic != IMAGE_DOS_SIGNATURE) {
        return meta;
    }

    if (static_cast<size_t>(dos_header->e_lfanew) + sizeof(IMAGE_NT_HEADERS64) > buffer.size()) {
        return meta;
    }

    auto* nt_headers = reinterpret_cast<IMAGE_NT_HEADERS64*>(
        buffer.data() + dos_header->e_lfanew
    );

    if (nt_headers->Signature != IMAGE_NT_SIGNATURE) {
        return meta;
    }

    meta.is_valid = true;
    if (nt_headers->FileHeader.Machine == IMAGE_FILE_MACHINE_AMD64) {
        meta.architecture = "x86_64";
    } else if (nt_headers->FileHeader.Machine == IMAGE_FILE_MACHINE_ARM64) {
        meta.architecture = "arm64";
    } else if (nt_headers->FileHeader.Machine == IMAGE_FILE_MACHINE_I386) {
        meta.architecture = "x86";
    } else {
        meta.architecture = "unknown";
    }

    auto* section_header = IMAGE_FIRST_SECTION(nt_headers);
    WORD num_sections = nt_headers->FileHeader.NumberOfSections;

    for (WORD i = 0; i < num_sections; ++i) {
        char name[9] = {0};
        std::memcpy(name, section_header[i].Name, 8);
        meta.sections.push_back(name);
    }

    auto rva_to_offset = [&](DWORD rva) -> DWORD {
        for (WORD i = 0; i < num_sections; ++i) {
            DWORD sec_rva = section_header[i].VirtualAddress;
            DWORD sec_size = section_header[i].SizeOfRawData;
            if (rva >= sec_rva && rva < sec_rva + sec_size) {
                return (rva - sec_rva) + section_header[i].PointerToRawData;
            }
        }
        return 0;
    };

    DWORD export_rva = nt_headers->OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_EXPORT].VirtualAddress;
    if (export_rva != 0) {
        DWORD export_offset = rva_to_offset(export_rva);
        if (export_offset > 0 && export_offset + sizeof(IMAGE_EXPORT_DIRECTORY) <= buffer.size()) {
            auto* export_dir = reinterpret_cast<IMAGE_EXPORT_DIRECTORY*>(buffer.data() + export_offset);
            DWORD num_names = export_dir->NumberOfNames;
            DWORD names_offset = rva_to_offset(export_dir->AddressOfNames);
            DWORD funcs_offset = rva_to_offset(export_dir->AddressOfFunctions);

            if (names_offset > 0 && funcs_offset > 0) {
                auto* name_rvas = reinterpret_cast<DWORD*>(buffer.data() + names_offset);
                auto* func_rvas = reinterpret_cast<DWORD*>(buffer.data() + funcs_offset);

                for (DWORD i = 0; i < num_names; ++i) {
                    DWORD str_offset = rva_to_offset(name_rvas[i]);
                    if (str_offset > 0 && str_offset < buffer.size()) {
                        const char* sym_name = reinterpret_cast<const char*>(buffer.data() + str_offset);
                        uint64_t sym_rva = (i < export_dir->NumberOfFunctions) ? func_rvas[i] : 0;
                        meta.exported_symbols.push_back(SymbolEntry{
                            sym_name,
                            sym_rva,
                            true,
                            true
                        });
                    }
                }
            }
        }
    }

    DWORD import_rva = nt_headers->OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_IMPORT].VirtualAddress;
    if (import_rva != 0) {
        DWORD import_offset = rva_to_offset(import_rva);
        while (import_offset > 0 && import_offset + sizeof(IMAGE_IMPORT_DESCRIPTOR) <= buffer.size()) {
            auto* import_desc = reinterpret_cast<IMAGE_IMPORT_DESCRIPTOR*>(buffer.data() + import_offset);
            if (import_desc->Name == 0) {
                break;
            }
            DWORD name_offset = rva_to_offset(import_desc->Name);
            if (name_offset > 0 && name_offset < buffer.size()) {
                const char* mod_name = reinterpret_cast<const char*>(buffer.data() + name_offset);
                meta.imported_modules.push_back(mod_name);
            }
            import_offset += sizeof(IMAGE_IMPORT_DESCRIPTOR);
        }
    }

    DWORD debug_rva = nt_headers->OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_DEBUG].VirtualAddress;
    if (debug_rva != 0) {
        DWORD debug_offset = rva_to_offset(debug_rva);
        if (debug_offset > 0 && debug_offset + sizeof(IMAGE_DEBUG_DIRECTORY) <= buffer.size()) {
            auto* debug_dir = reinterpret_cast<IMAGE_DEBUG_DIRECTORY*>(buffer.data() + debug_offset);
            if (debug_dir->Type == IMAGE_DEBUG_TYPE_CODEVIEW && debug_dir->PointerToRawData > 0) {
                DWORD raw_offset = debug_dir->PointerToRawData;
                if (raw_offset + 24 <= buffer.size()) {
                    uint32_t signature = *reinterpret_cast<uint32_t*>(buffer.data() + raw_offset);
                    if (signature == 0x53445352) {
                        const char* pdb_str = reinterpret_cast<const char*>(buffer.data() + raw_offset + 24);
                        meta.pdb_path = pdb_str;
                    }
                }
            }
        }
    }

#else
    if (buffer.size() >= 4 && buffer[0] == 0x7F && buffer[1] == 'E' && buffer[2] == 'L' && buffer[3] == 'F') {
        meta.is_valid = true;
        meta.architecture = (buffer[4] == 2) ? "x86_64" : "x86";
        meta.sections.push_back(".text");
        meta.sections.push_back(".data");
        meta.sections.push_back(".rodata");
    }
#endif

    return meta;
}

}
