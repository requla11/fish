#!/usr/bin/env python3
import os
import sys

def main():
    path = os.path.join("submodules", "apple", "src", "isolation", "process.rs")
    if not os.path.isfile(path):
        print(f"File {path} not found, skipping patch.")
        return 0

    with open(path, "r", encoding="utf-8") as f:
        content = f.read()

    target = """        #[cfg(unix)]
        {
            let rules = request.profile.mount_rules.clone();
            unsafe {
                cmd.pre_exec(move || {"""

    replacement = """        #[cfg(unix)]
        {
            let rules = request.profile.mount_rules.clone();
            #[cfg(target_os = "linux")]
            let is_full_hermetic = request.profile.level == IsolationLevel::FullHermetic;
            unsafe {
                cmd.pre_exec(move || {
                    #[cfg(target_os = "linux")]
                    if !is_full_hermetic {
                        if libc::setpgid(0, 0) != 0 {
                            return Err(std::io::Error::last_os_error());
                        }
                        return Ok(());
                    }"""

    if target in content:
        content = content.replace(target, replacement, 1)
        with open(path, "w", encoding="utf-8") as f:
            f.write(content)
        print("Successfully patched submodules/apple/src/isolation/process.rs")
    else:
        print("Target pattern already patched or not found.")

    return 0

if __name__ == "__main__":
    sys.exit(main())
