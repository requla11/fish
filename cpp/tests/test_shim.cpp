#include "binary_surgeon.h"
#include "fish_shim.h"
#include "symbol_inspector.h"
#include "usn_watcher.h"

#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <string>
#include <vector>

static void expect_true(bool condition, int line) {
    if (!condition) {
        std::cerr << "Assertion failed at line: " << line << std::endl;
        std::abort();
    }
}

void test_shim_logging(const std::filesystem::path& temp_dir) {
    std::filesystem::path log_path = temp_dir / "trace.log";

#ifdef _WIN32
    _putenv_s("FISH_SHIM_LOG", log_path.string().c_str());
#else
    setenv("FISH_SHIM_LOG", log_path.string().c_str(), 1);
#endif

    fish::shim::initialize();
    fish::shim::record_access(fish::shim::OpType::Read, "src/main.cpp");
    fish::shim::record_access(fish::shim::OpType::Write, "build/main.o");
    fish::shim::record_access(fish::shim::OpType::Execute, "/usr/bin/clang");
    fish::shim::shutdown();

    expect_true(std::filesystem::exists(log_path), __LINE__);

    std::ifstream stream(log_path);
    std::string line;
    bool found_read = false;
    bool found_write = false;
    bool found_exec = false;

    while (std::getline(stream, line)) {
        if (line.find("READ\tsrc/main.cpp") != std::string::npos) {
            found_read = true;
        }
        if (line.find("WRITE\tbuild/main.o") != std::string::npos) {
            found_write = true;
        }
        if (line.find("EXEC\t/usr/bin/clang") != std::string::npos) {
            found_exec = true;
        }
    }

    expect_true(found_read, __LINE__);
    expect_true(found_write, __LINE__);
    expect_true(found_exec, __LINE__);
    std::cout << "[ok] test_shim_logging passed\n";
}

void test_binary_surgeon(const std::filesystem::path& temp_dir) {
    std::filesystem::path bin_file = temp_dir / "mock.bin";
    std::vector<uint8_t> initial = {0x48, 0x89, 0x5C, 0x24, 0x08, 0x90, 0x90, 0xC3};

    {
        std::ofstream out(bin_file, std::ios::binary);
        out.write(reinterpret_cast<const char*>(initial.data()), initial.size());
    }

    auto read1 = fish::surgeon::BinarySurgeon::read_bytes(bin_file.string(), 0, 8);
    expect_true(read1 == initial, __LINE__);

    fish::surgeon::PatchEntry patch;
    patch.file_offset = 5;
    patch.original_bytes = {0x90, 0x90};
    patch.new_bytes = {0xCC, 0xCC};

    bool patched = fish::surgeon::BinarySurgeon::apply_patch(bin_file.string(), patch);
    expect_true(patched, __LINE__);

    auto read2 = fish::surgeon::BinarySurgeon::read_bytes(bin_file.string(), 5, 2);
    expect_true(read2.size() == 2, __LINE__);
    expect_true(read2[0] == 0xCC && read2[1] == 0xCC, __LINE__);

    bool tramp = fish::surgeon::BinarySurgeon::apply_trampoline_rel32(bin_file.string(), 0, 0x12345678);
    expect_true(tramp, __LINE__);

    auto read3 = fish::surgeon::BinarySurgeon::read_bytes(bin_file.string(), 0, 5);
    expect_true(read3.size() == 5, __LINE__);
    expect_true(read3[0] == 0xE9, __LINE__);
    expect_true(read3[1] == 0x78, __LINE__);
    expect_true(read3[2] == 0x56, __LINE__);
    expect_true(read3[3] == 0x34, __LINE__);
    expect_true(read3[4] == 0x12, __LINE__);
    std::cout << "[ok] test_binary_surgeon passed\n";
}

void test_symbol_inspector(const std::filesystem::path& exe_dir) {
    std::filesystem::path dll_path = exe_dir / "fish_shim.dll";
    if (!std::filesystem::exists(dll_path)) {
        dll_path = exe_dir / "libfish_shim.so";
    }
    if (!std::filesystem::exists(dll_path)) {
        dll_path = exe_dir / "test_shim.exe";
    }

    if (std::filesystem::exists(dll_path)) {
        auto meta = fish::symbols::SymbolInspector::inspect_binary(dll_path.string());
        expect_true(meta.is_valid, __LINE__);
        expect_true(!meta.sections.empty(), __LINE__);
        expect_true(meta.architecture == "x86_64" || meta.architecture == "arm64", __LINE__);
    }
    std::cout << "[ok] test_symbol_inspector passed\n";
}

void test_usn_watcher() {
    fish::watcher::UsnJournalWatcher watcher;
    watcher.initialize("C:");
    auto events = watcher.poll_changes();
    expect_true(watcher.get_current_usn() >= 0, __LINE__);
    (void)events;
    std::cout << "[ok] test_usn_watcher passed\n";
}

int main(int argc, char* argv[]) {
    std::filesystem::path exe_dir = ".";
    if (argc > 0 && argv[0]) {
        std::filesystem::path p(argv[0]);
        if (p.has_parent_path()) {
            exe_dir = p.parent_path();
        }
    }

    std::filesystem::path temp_dir = std::filesystem::temp_directory_path() / "fish_native_test";
    std::filesystem::create_directories(temp_dir);

    test_shim_logging(temp_dir);
    test_binary_surgeon(temp_dir);
    test_symbol_inspector(exe_dir);
    test_usn_watcher();

    std::filesystem::remove_all(temp_dir);
    std::cout << "All native C++ subsystem tests passed successfully.\n";
    return 0;
}
