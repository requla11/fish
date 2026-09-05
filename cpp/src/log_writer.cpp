#include "fish_shim.h"
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <mutex>
#include <string>

namespace fish::shim {

namespace {
std::mutex g_log_mutex;
std::ofstream g_log_file;
bool g_initialized = false;

std::string get_log_path() {
#ifdef _WIN32
    char* buf = nullptr;
    size_t sz = 0;
    if (_dupenv_s(&buf, &sz, "FISH_SHIM_LOG") == 0 && buf != nullptr) {
        std::string res(buf);
        free(buf);
        return res;
    }
    return "";
#else
    const char* p = std::getenv("FISH_SHIM_LOG");
    return p ? std::string(p) : std::string();
#endif
}

void try_open_log_file_locked() {
    if (g_log_file.is_open()) {
        return;
    }
    std::string path = get_log_path();
    if (!path.empty()) {
        std::filesystem::path p(path);
        if (p.has_parent_path()) {
            std::error_code ec;
            std::filesystem::create_directories(p.parent_path(), ec);
        }
        g_log_file.open(path, std::ios::out | std::ios::app);
    }
}
}

void initialize() {
    std::lock_guard<std::mutex> lock(g_log_mutex);
    try_open_log_file_locked();
    g_initialized = true;
}

void shutdown() {
    std::lock_guard<std::mutex> lock(g_log_mutex);
    if (g_log_file.is_open()) {
        g_log_file.flush();
        g_log_file.close();
    }
    g_initialized = false;
}

void record_access(OpType op, std::string_view path) {
    if (path.empty()) {
        return;
    }

    std::lock_guard<std::mutex> lock(g_log_mutex);
    try_open_log_file_locked();
    if (!g_log_file.is_open()) {
        return;
    }

    const char* op_str = "READ";
    switch (op) {
        case OpType::Read:
            op_str = "READ";
            break;
        case OpType::Write:
            op_str = "WRITE";
            break;
        case OpType::Execute:
            op_str = "EXEC";
            break;
    }

    g_log_file << op_str << "\t" << path << "\n";
    g_log_file.flush();
}

}
