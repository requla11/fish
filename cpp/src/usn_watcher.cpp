#include "usn_watcher.h"

#ifdef _WIN32
#include <windows.h>
#include <winioctl.h>
#else
#include <sys/stat.h>
#endif

namespace fish::watcher {

UsnJournalWatcher::UsnJournalWatcher()
    : volume_handle_(nullptr), journal_id_(0), next_usn_(0) {}

UsnJournalWatcher::~UsnJournalWatcher() {
#ifdef _WIN32
    if (volume_handle_ && volume_handle_ != INVALID_HANDLE_VALUE) {
        CloseHandle(static_cast<HANDLE>(volume_handle_));
        volume_handle_ = nullptr;
    }
#endif
}

bool UsnJournalWatcher::initialize(const std::string& volume_root) {
    volume_path_ = volume_root;

#ifdef _WIN32
    std::string vol = volume_root;
    if (vol.size() >= 2 && vol[1] == ':') {
        vol = "\\\\.\\" + vol.substr(0, 2);
    } else if (vol.empty()) {
        vol = "\\\\.\\C:";
    }

    HANDLE h = CreateFileA(
        vol.c_str(),
        GENERIC_READ | GENERIC_WRITE,
        FILE_SHARE_READ | FILE_SHARE_WRITE,
        nullptr,
        OPEN_EXISTING,
        0,
        nullptr
    );

    if (h == INVALID_HANDLE_VALUE) {
        h = CreateFileA(
            vol.c_str(),
            GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            nullptr,
            OPEN_EXISTING,
            0,
            nullptr
        );
    }

    if (h == INVALID_HANDLE_VALUE) {
        return false;
    }

    volume_handle_ = h;

    USN_JOURNAL_DATA_V0 journal_data{};
    DWORD bytes_returned = 0;
    BOOL ok = DeviceIoControl(
        h,
        FSCTL_QUERY_USN_JOURNAL,
        nullptr,
        0,
        &journal_data,
        sizeof(journal_data),
        &bytes_returned,
        nullptr
    );

    if (!ok) {
        CloseHandle(h);
        volume_handle_ = nullptr;
        return false;
    }

    journal_id_ = journal_data.UsnJournalID;
    next_usn_ = journal_data.NextUsn;
    return true;
#else
    return true;
#endif
}

std::vector<FileChangeEvent> UsnJournalWatcher::poll_changes() {
    std::vector<FileChangeEvent> events;

#ifdef _WIN32
    if (!volume_handle_ || volume_handle_ == INVALID_HANDLE_VALUE) {
        return events;
    }

    HANDLE h = static_cast<HANDLE>(volume_handle_);
    READ_USN_JOURNAL_DATA_V0 read_data{};
    read_data.StartUsn = next_usn_;
    read_data.ReasonMask = 0xFFFFFFFF;
    read_data.ReturnOnlyOnClose = 0;
    read_data.Timeout = 0;
    read_data.BytesToWaitFor = 0;
    read_data.UsnJournalID = journal_id_;

    std::vector<uint8_t> buffer(64 * 1024, 0);
    DWORD bytes_read = 0;

    BOOL ok = DeviceIoControl(
        h,
        FSCTL_READ_USN_JOURNAL,
        &read_data,
        sizeof(read_data),
        buffer.data(),
        static_cast<DWORD>(buffer.size()),
        &bytes_read,
        nullptr
    );

    if (!ok || bytes_read <= sizeof(USN)) {
        return events;
    }

    next_usn_ = *reinterpret_cast<USN*>(buffer.data());

    uint8_t* ptr = buffer.data() + sizeof(USN);
    uint8_t* end = buffer.data() + bytes_read;

    while (ptr < end) {
        auto* record = reinterpret_cast<USN_RECORD_V2*>(ptr);
        if (record->RecordLength == 0) {
            break;
        }

        if (record->FileNameLength > 0) {
            auto* name_ptr = reinterpret_cast<wchar_t*>(
                reinterpret_cast<uint8_t*>(record) + record->FileNameOffset
            );
            int name_chars = record->FileNameLength / sizeof(wchar_t);
            int utf8_len = WideCharToMultiByte(
                CP_UTF8, 0, name_ptr, name_chars, nullptr, 0, nullptr, nullptr
            );

            if (utf8_len > 0) {
                std::string file_name(static_cast<size_t>(utf8_len), '\0');
                WideCharToMultiByte(
                    CP_UTF8, 0, name_ptr, name_chars, file_name.data(), utf8_len, nullptr, nullptr
                );
                events.push_back(FileChangeEvent{
                    std::move(file_name),
                    record->Reason,
                    static_cast<uint64_t>(record->Usn)
                });
            }
        }

        ptr += record->RecordLength;
    }
#endif

    return events;
}

uint64_t UsnJournalWatcher::get_current_usn() const {
    return next_usn_;
}

}
