#include "fish_shim.h"

#ifdef _WIN32
#include <windows.h>

BOOL WINAPI DllMain(HINSTANCE hinstDLL, DWORD fdwReason, LPVOID lpvReserved) {
    (void)hinstDLL;
    (void)lpvReserved;
    switch (fdwReason) {
        case DLL_PROCESS_ATTACH:
            DisableThreadLibraryCalls(hinstDLL);
            fish::shim::initialize();
            break;
        case DLL_PROCESS_DETACH:
            fish::shim::shutdown();
            break;
    }
    return TRUE;
}

#else

__attribute__((constructor)) static void shim_init() {
    fish::shim::initialize();
}

__attribute__((destructor)) static void shim_fini() {
    fish::shim::shutdown();
}

#endif
