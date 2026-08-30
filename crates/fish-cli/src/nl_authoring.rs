use crate::commands::init::DetectedLanguage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Archetype {
    Cli,
    Api,
    Library,
    Worker,
    Fullstack,
    Microservice,
    Embedded,
    Generic,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedDescription {
    pub languages: Vec<DetectedLanguage>,
    pub archetype: Archetype,
}

fn lang_for(word: &str) -> Option<DetectedLanguage> {
    match word {
        "rust" | "cargo" | "rs" => Some(DetectedLanguage {
            name: "Rust",
            backend: "rust",
            build_cmd: "cargo build",
            test_cmd: "cargo test",
        }),
        "go" | "golang" => Some(DetectedLanguage {
            name: "Go",
            backend: "go",
            build_cmd: "go build ./...",
            test_cmd: "go test ./...",
        }),
        "typescript" | "ts" | "javascript" | "js" | "node" | "npm" | "react" | "vue" | "svelte"
        | "next" | "frontend" => Some(DetectedLanguage {
            name: "TypeScript / Node",
            backend: "ts",
            build_cmd: "npm run build",
            test_cmd: "npm test",
        }),
        "python" | "py" | "pip" | "poetry" | "uv" | "django" | "fastapi" | "flask" => {
            Some(DetectedLanguage {
                name: "Python",
                backend: "python",
                build_cmd: "python -m compileall .",
                test_cmd: "pytest",
            })
        }
        "c++" | "cpp" | "c" | "cmake" | "clang" | "gcc" | "make" => Some(DetectedLanguage {
            name: "C/C++",
            backend: "cc",
            build_cmd: "cmake --build build",
            test_cmd: "ctest --test-dir build",
        }),
        "java" | "jvm" | "maven" | "mvn" | "gradle" | "spring" => Some(DetectedLanguage {
            name: "Java",
            backend: "java",
            build_cmd: "mvn compile",
            test_cmd: "mvn test",
        }),
        "dotnet" | ".net" | "c#" | "csharp" | "f#" | "fsharp" | "nuget" => Some(DetectedLanguage {
            name: ".NET",
            backend: "dotnet",
            build_cmd: "dotnet build",
            test_cmd: "dotnet test",
        }),
        "swift" | "swiftpm" | "ios" | "macos" => Some(DetectedLanguage {
            name: "Swift",
            backend: "swift",
            build_cmd: "swift build",
            test_cmd: "swift test",
        }),
        "dart" | "flutter" => Some(DetectedLanguage {
            name: "Dart / Flutter",
            backend: "dart",
            build_cmd: "dart compile exe bin/main.dart",
            test_cmd: "dart test",
        }),
        "zig" | "zon" => Some(DetectedLanguage {
            name: "Zig",
            backend: "zig",
            build_cmd: "zig build",
            test_cmd: "zig test",
        }),
        "docker" | "dockerfile" | "container" => Some(DetectedLanguage {
            name: "Docker",
            backend: "docker",
            build_cmd: "docker build -t app .",
            test_cmd: "docker run --rm app test",
        }),
        _ => None,
    }
}

fn archetype_for(words: &[&str]) -> Archetype {
    for w in words {
        match *w {
            "cli" | "binary" | "tool" | "command" | "lệnh" | "công cụ" | "命令行" => {
                return Archetype::Cli;
            }
            "api" | "server" | "service" | "rest" | "grpc" | "backend" | "dịch vụ" | "máy chủ"
            | "后端" => return Archetype::Api,
            "lib" | "library" | "crate" | "sdk" | "package" | "thư viện" | "库" => {
                return Archetype::Library;
            }
            "worker" | "queue" | "cron" | "job" | "task" | "hàng đợi" | "xử lý" => {
                return Archetype::Worker;
            }
            "fullstack" | "monorepo" | "toàn diện" | "全栈" => return Archetype::Fullstack,
            "microservice" | "microservices" | "distributed" | "phân tán" | "微服务" => {
                return Archetype::Microservice;
            }
            "embedded" | "iot" | "firmware" | "nhúng" | "嵌入式" => return Archetype::Embedded,
            _ => {}
        }
    }
    Archetype::Generic
}

pub fn parse_description(input: &str) -> ParsedDescription {
    let lowered = input.to_lowercase();
    let words: Vec<&str> = lowered
        .split(|c: char| c.is_whitespace() || c == ',' || c == '+' || c == '/' || c == '&')
        .filter(|w| {
            !w.is_empty()
                && !matches!(
                    *w,
                    "a" | "an"
                        | "and"
                        | "with"
                        | "the"
                        | "for"
                        | "và"
                        | "với"
                        | "cho"
                        | "dự"
                        | "án"
                        | "ngôn"
                        | "ngữ"
                        | "和"
                        | "与"
                        | "的"
                        | "と"
                        | "の"
                )
        })
        .collect();

    let mut languages: Vec<DetectedLanguage> = Vec::new();
    for word in &words {
        if let Some(lang) = lang_for(word)
            && !languages.iter().any(|l| l.backend == lang.backend)
        {
            languages.push(lang);
        }
    }

    ParsedDescription {
        languages,
        archetype: archetype_for(&words),
    }
}

pub fn generate_from_description(parsed: &ParsedDescription) -> String {
    let mut out = String::from("# Generated by `fish init --describe`\n");
    out.push_str(&format!("# archetype: {:?}\n", parsed.archetype));
    out.push_str("version: \"1\"\n\ntasks:\n");

    if parsed.languages.is_empty() {
        out.push_str("  build:\n    command: echo \"Building project...\"\n\n");
        out.push_str(
            "  test:\n    command: echo \"Running tests...\"\n    depends_on:\n      - build\n",
        );
        return out;
    }

    for lang in &parsed.languages {
        let prefix = lang.backend;
        out.push_str(&format!("  {prefix}-build:\n"));
        out.push_str(&format!("    command: {}\n", lang.build_cmd));
        out.push_str("    cache:\n      enabled: true\n\n");
        out.push_str(&format!("  {prefix}-test:\n"));
        out.push_str(&format!("    command: {}\n", lang.test_cmd));
        out.push_str(&format!("    depends_on:\n      - {prefix}-build\n\n"));
    }

    out.push_str("  build:\n    depends_on:\n");
    for lang in &parsed.languages {
        out.push_str(&format!("      - {}-build\n", lang.backend));
    }
    out.push_str("\n  test:\n    depends_on:\n");
    for lang in &parsed.languages {
        out.push_str(&format!("      - {}-test\n", lang.backend));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_two_languages() {
        let p = parse_description("rust cli + python tools");
        assert_eq!(p.archetype, Archetype::Cli);
        assert_eq!(p.languages.len(), 2);
        assert_eq!(p.languages[0].backend, "rust");
        assert_eq!(p.languages[1].backend, "python");
    }

    #[test]
    fn recognizes_api_archetype() {
        let p = parse_description("a rust api server with postgres");
        assert_eq!(p.archetype, Archetype::Api);
        assert_eq!(p.languages.len(), 1);
    }

    #[test]
    fn recognizes_vietnamese_and_all_backends() {
        let p = parse_description("dự án microservice với go và zig và docker");
        assert_eq!(p.archetype, Archetype::Microservice);
        assert_eq!(p.languages.len(), 3);
        assert_eq!(p.languages[0].backend, "go");
        assert_eq!(p.languages[1].backend, "zig");
        assert_eq!(p.languages[2].backend, "docker");
    }

    #[test]
    fn dedupes_repeated_languages() {
        let p = parse_description("rust and cargo and rs");
        assert_eq!(p.languages.len(), 1);
    }

    #[test]
    fn unknown_words_yield_generic_empty() {
        let p = parse_description("something something postgres");
        assert!(p.languages.is_empty());
        assert_eq!(p.archetype, Archetype::Generic);
    }

    #[test]
    fn output_is_valid_yaml_shape() {
        let p = parse_description("go worker + ts frontend");
        let yaml = generate_from_description(&p);
        assert!(yaml.contains("go-build:"));
        assert!(yaml.contains("ts-test:"));
        assert!(yaml.contains("- go-build"));
        assert!(yaml.starts_with("# Generated by"));
    }

    #[test]
    fn empty_description_falls_back_to_echo() {
        let yaml = generate_from_description(&parse_description(""));
        assert!(yaml.contains("echo \"Building project...\""));
    }
}
