#include "fish_shim.h"
#include <cassert>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <string>

int main() {
    std::filesystem::path temp_dir = std::filesystem::temp_directory_path() / "fish_shim_test";
    std::filesystem::create_directories(temp_dir);
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

    assert(std::filesystem::exists(log_path));

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

    assert(found_read);
    assert(found_write);
    assert(found_exec);

    std::filesystem::remove_all(temp_dir);
    std::cout << "All fish_shim tests passed successfully.\n";
    return 0;
}
