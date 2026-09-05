#pragma once

#include "fish_shim.h"
#include <cstdint>
#include <string>
#include <vector>

namespace fish::watcher {

struct FileChangeEvent {
    std::string path;
    uint32_t reason;
    uint64_t usn;
};

class UsnJournalWatcher {
public:
    FISH_SHIM_API UsnJournalWatcher();
    FISH_SHIM_API ~UsnJournalWatcher();

    FISH_SHIM_API bool initialize(const std::string& volume_root);
    FISH_SHIM_API std::vector<FileChangeEvent> poll_changes();
    FISH_SHIM_API uint64_t get_current_usn() const;

private:
    void* volume_handle_;
    uint64_t journal_id_;
    uint64_t next_usn_;
    std::string volume_path_;
};

}
