#ifndef _WIN32

#include "fish_shim.h"
#include <cstdarg>
#include <cstdio>
#include <dlfcn.h>
#include <fcntl.h>
#include <string>
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h>

using open_fn = int (*)(const char*, int, ...);
using openat_fn = int (*)(int, const char*, int, ...);
using fopen_fn = FILE* (*)(const char*, const char*);
using execve_fn = int (*)(const char*, char* const[], char* const[]);

static open_fn real_open = nullptr;
static openat_fn real_openat = nullptr;
static fopen_fn real_fopen = nullptr;
static execve_fn real_execve = nullptr;

static void init_real_functions() {
    if (!real_open) {
        real_open = reinterpret_cast<open_fn>(dlsym(RTLD_NEXT, "open"));
    }
    if (!real_openat) {
        real_openat = reinterpret_cast<openat_fn>(dlsym(RTLD_NEXT, "openat"));
    }
    if (!real_fopen) {
        real_fopen = reinterpret_cast<fopen_fn>(dlsym(RTLD_NEXT, "fopen"));
    }
    if (!real_execve) {
        real_execve = reinterpret_cast<execve_fn>(dlsym(RTLD_NEXT, "execve"));
    }
}

extern "C" {

int open(const char* pathname, int flags, ...) {
    init_real_functions();
    mode_t mode = 0;
    if (flags & O_CREAT) {
        va_list args;
        va_start(args, flags);
        mode = va_arg(args, mode_t);
        va_end(args);
    }

    fish::shim::OpType op = (flags & (O_WRONLY | O_RDWR | O_CREAT | O_TRUNC))
        ? fish::shim::OpType::Write
        : fish::shim::OpType::Read;

    if (pathname) {
        fish::shim::record_access(op, pathname);
    }

    return real_open ? real_open(pathname, flags, mode) : -1;
}

int openat(int dirfd, const char* pathname, int flags, ...) {
    init_real_functions();
    mode_t mode = 0;
    if (flags & O_CREAT) {
        va_list args;
        va_start(args, flags);
        mode = va_arg(args, mode_t);
        va_end(args);
    }

    fish::shim::OpType op = (flags & (O_WRONLY | O_RDWR | O_CREAT | O_TRUNC))
        ? fish::shim::OpType::Write
        : fish::shim::OpType::Read;

    if (pathname) {
        fish::shim::record_access(op, pathname);
    }

    return real_openat ? real_openat(dirfd, pathname, flags, mode) : -1;
}

FILE* fopen(const char* pathname, const char* mode) {
    init_real_functions();
    if (pathname && mode) {
        fish::shim::OpType op = (mode[0] == 'w' || mode[0] == 'a' || mode[0] == '+' || (mode[1] && mode[1] == '+'))
            ? fish::shim::OpType::Write
            : fish::shim::OpType::Read;
        fish::shim::record_access(op, pathname);
    }
    return real_fopen ? real_fopen(pathname, mode) : nullptr;
}

int execve(const char* filename, char* const argv[], char* const envp[]) {
    init_real_functions();
    if (filename) {
        fish::shim::record_access(fish::shim::OpType::Execute, filename);
    }
    return real_execve ? real_execve(filename, argv, envp) : -1;
}

}

#endif
