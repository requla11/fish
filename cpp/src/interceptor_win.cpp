#ifdef _WIN32

#include "fish_shim.h"
#include <string>
#include <vector>
#include <windows.h>

namespace {

std::string wide_to_utf8(LPCWSTR wide_str) {
    if (!wide_str) {
        return "";
    }
    int len = WideCharToMultiByte(CP_UTF8, 0, wide_str, -1, nullptr, 0, nullptr, nullptr);
    if (len <= 1) {
        return "";
    }
    std::string str(static_cast<size_t>(len - 1), '\0');
    WideCharToMultiByte(CP_UTF8, 0, wide_str, -1, str.data(), len, nullptr, nullptr);
    return str;
}

using CreateFileW_fn = HANDLE(WINAPI*)(
    LPCWSTR, DWORD, DWORD, LPSECURITY_ATTRIBUTES, DWORD, DWORD, HANDLE
);

using CreateFileA_fn = HANDLE(WINAPI*)(
    LPCSTR, DWORD, DWORD, LPSECURITY_ATTRIBUTES, DWORD, DWORD, HANDLE
);

CreateFileW_fn g_real_CreateFileW = nullptr;
CreateFileA_fn g_real_CreateFileA = nullptr;

}

namespace fish::shim {

void hook_win32_apis() {
    HMODULE kernel32 = GetModuleHandleW(L"kernel32.dll");
    if (!kernel32) {
        return;
    }
    g_real_CreateFileW = reinterpret_cast<CreateFileW_fn>(
        GetProcAddress(kernel32, "CreateFileW")
    );
    g_real_CreateFileA = reinterpret_cast<CreateFileA_fn>(
        GetProcAddress(kernel32, "CreateFileA")
    );
}

}

extern "C" {

FISH_SHIM_API HANDLE WINAPI FishCreateFileW(
    LPCWSTR lpFileName,
    DWORD dwDesiredAccess,
    DWORD dwShareMode,
    LPSECURITY_ATTRIBUTES lpSecurityAttributes,
    DWORD dwCreationDisposition,
    DWORD dwFlagsAndAttributes,
    HANDLE hTemplateFile
) {
    if (!g_real_CreateFileW) {
        fish::shim::hook_win32_apis();
    }

    if (lpFileName) {
        std::string utf8_path = wide_to_utf8(lpFileName);
        bool is_write = (dwDesiredAccess & (GENERIC_WRITE | FILE_WRITE_DATA | FILE_APPEND_DATA)) != 0;
        fish::shim::record_access(
            is_write ? fish::shim::OpType::Write : fish::shim::OpType::Read,
            utf8_path
        );
    }

    if (g_real_CreateFileW) {
        return g_real_CreateFileW(
            lpFileName, dwDesiredAccess, dwShareMode, lpSecurityAttributes,
            dwCreationDisposition, dwFlagsAndAttributes, hTemplateFile
        );
    }
    return INVALID_HANDLE_VALUE;
}

FISH_SHIM_API HANDLE WINAPI FishCreateFileA(
    LPCSTR lpFileName,
    DWORD dwDesiredAccess,
    DWORD dwShareMode,
    LPSECURITY_ATTRIBUTES lpSecurityAttributes,
    DWORD dwCreationDisposition,
    DWORD dwFlagsAndAttributes,
    HANDLE hTemplateFile
) {
    if (!g_real_CreateFileA) {
        fish::shim::hook_win32_apis();
    }

    if (lpFileName) {
        bool is_write = (dwDesiredAccess & (GENERIC_WRITE | FILE_WRITE_DATA | FILE_APPEND_DATA)) != 0;
        fish::shim::record_access(
            is_write ? fish::shim::OpType::Write : fish::shim::OpType::Read,
            lpFileName
        );
    }

    if (g_real_CreateFileA) {
        return g_real_CreateFileA(
            lpFileName, dwDesiredAccess, dwShareMode, lpSecurityAttributes,
            dwCreationDisposition, dwFlagsAndAttributes, hTemplateFile
        );
    }
    return INVALID_HANDLE_VALUE;
}

}

#endif
