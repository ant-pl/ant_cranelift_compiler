use std::{
    collections::HashMap,
    fmt::Display,
    path::{Path, PathBuf},
    process::Command,
};

use once_cell::sync::Lazy;

use crate::args::Args;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TargetTriple {
    /// 架构
    pub arch: String,
    /// 厂商
    pub vendor: String,
    /// 操作系统
    pub os: String,
    /// 二进制接口
    pub abi: String,
}

impl Display for TargetTriple {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}-{}-{}{}",
            self.arch,
            self.vendor,
            self.os,
            if self.abi.is_empty() {
                String::new()
            } else {
                format!("-{}", self.abi)
            }
        )
    }
}

impl From<String> for TargetTriple {
    fn from(value: String) -> Self {
        let target_triple = value
            .split("-")
            .map(|it| it.to_string())
            .collect::<Vec<_>>();

        Self {
            arch: target_triple[0].clone(),
            vendor: target_triple[1].clone(),
            os: target_triple[2].clone(),
            abi: target_triple.get(3).cloned().unwrap_or("".to_owned()),
        }
    }
}

impl TargetTriple {
    pub fn new<T: ToString>(s: T) -> Self {
        Self::from(s.to_string())
    }
}

pub static ARG_TO_ABI_ARG: Lazy<HashMap<&str, HashMap<&str, &str>>> = Lazy::new(|| {
    let mut m = HashMap::new();

    let mut msvc = HashMap::new();

    msvc.insert("linkdir", "/LIBPATH");
    msvc.insert("link", "");
    msvc.insert("output", "/OUT");

    let mut gnu = HashMap::new();

    gnu.insert("linkdir", "-L");
    gnu.insert("link", "-l");
    gnu.insert("output", "-o");

    m.insert("msvc", msvc);
    m.insert("gnu", gnu);

    m
});

pub fn choose_linker(target_triple: &TargetTriple) -> Option<Command> {
    if target_triple.os == "macos" || target_triple.vendor == "apple" {
        return Some(Command::new("clang"));
    }

    match target_triple.abi.as_str() {
        "msvc" => Some(
            cc::windows_registry::find_tool(&target_triple.to_string(), "link.exe")
                .map(|it| it.to_command())
                .unwrap_or(Command::new("link")),
        ),
        "gnu" => Some(Command::new("gcc")),
        _ => None,
    }
}

/// 确定目标三元组
pub fn get_target_triple_or_default(arg: &Option<Args>) -> String {
    let default_target = get_default_target();

    if let Some(args) = arg {
        if args.target_triple.is_empty() {
            default_target.to_string()
        } else {
            args.target_triple.clone()
        }
    } else {
        default_target.to_string()
    }
}

/// 获取当前平台的默认目标三元组
pub fn get_default_target() -> &'static str {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        return "x86_64-pc-windows-gnu";
    }

    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        return "aarch64-unknown-linux-gnu";
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        return "x86_64-unknown-linux-gnu";
    }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        return "aarch64-apple-darwin";
    }

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        return "x86_64-apple-darwin";
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        panic!("Unsupported target platform");
    }
}

/// 使用 cc 构建静态库
pub fn build_static_library(
    object_file_path: &Path,
    output_path: &Path,
    target: &str,
    _arg: &Option<Args>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let stem = output_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    let out_dir = output_path.parent().unwrap_or_else(|| Path::new("."));

    // 根据目标三元组决定静态库的扩展名
    let target_triple = TargetTriple::new(target);

    let lib_extension = if target_triple.abi == "msvc" {
        "lib"
    } else {
        "a"
    };

    let lib_prefix = if target_triple.abi == "msvc" {
        ""
    } else {
        "lib"
    };

    let lib_name = format!("{lib_prefix}{}.{}", stem, lib_extension);
    let lib_path = out_dir.join(&lib_name);

    let mut build = cc::Build::new();
    build
        .object(object_file_path)
        .target(target)
        .host("CONSOLE")
        .cargo_metadata(false)
        .out_dir(out_dir);

    // 设置优化级别
    if let Some(args) = _arg {
        if !args.opt_level.0.is_empty() {
            build.opt_level_str(&args.opt_level.0);
        } else {
            build.opt_level(0);
        }
    } else {
        build.opt_level(0);
    }

    // 编译生成静态库（注意：cc 会根据目标平台自动选择 ar 或 lib.exe）
    build.try_compile(stem)?;

    Ok(lib_path)
}

/// 链接器配置信息
pub struct LinkerConfig {
    output_flag: String,
    linkdir_flag: String,
    library_flag: String,
}

/// 根据 ABI 获取链接器配置
pub fn get_linker_config(
    target_triple: &TargetTriple,
) -> Result<LinkerConfig, Box<dyn std::error::Error>> {
    let abi_config = ARG_TO_ABI_ARG
        .get(
            if target_triple.abi.is_empty() || target_triple.vendor == "apple" {
                "gnu"
            } else {
                target_triple.abi.as_str()
            },
        )
        .ok_or_else(|| format!("Unsupported ABI: {}", target_triple.abi))?;

    Ok(LinkerConfig {
        output_flag: abi_config
            .get("output")
            .ok_or("output command not found")?
            .to_string(),
        linkdir_flag: abi_config
            .get("linkdir")
            .ok_or("linkdir command not found")?
            .to_string(),
        library_flag: abi_config
            .get("link")
            .ok_or("link command not found")?
            .to_string(),
    })
}

/// 获取编译器目录
pub fn get_compiler_dir() -> String {
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        return manifest_dir;
    }

    // 直接以编译器目录为主
    std::env::current_exe()
        .unwrap_or_else(|_| PathBuf::from("."))
        .parent()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| ".".to_string())
}

/// 执行最终的链接操作
pub fn link_executable(
    target_triple: &TargetTriple,
    lib_path: &Path,
    output_path: &Path,
    compiler_dir: &str,
    linker_config: &LinkerConfig,
    arg: &Option<Args>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut command = choose_linker(&target_triple)
        .ok_or_else(|| format!("compiler for abi {} not found", target_triple.abi))?;

    // 添加调试信息标志
    if target_triple.abi == "gnu" && arg.as_ref().is_some_and(|args| args.debug_info) {
        command.arg("-g");
    } else if target_triple.abi == "msvc" && arg.as_ref().is_some_and(|args| args.debug_info) {
        command.arg("/Zi");
    }

    // 构建链接器参数
    build_linker_args(
        &mut command,
        target_triple,
        lib_path,
        output_path,
        compiler_dir,
        linker_config,
        arg,
    );

    // 添加平台特定的链接选项（仅对 GNU 风格）
    if target_triple.abi == "gnu" {
        add_platform_specific_flags(&mut command);
    }

    let status = command
        .status()
        .map_err(|e| format!("Failed to execute linker: {}", e))?;

    if !status.success() {
        return Err(format!("Linker failed with status: {}", status).into());
    }

    Ok(())
}

/// 构建链接器参数
pub fn build_linker_args(
    command: &mut dyn CommandLike,
    target_triple: &TargetTriple,
    lib_path: &Path,
    output_path: &Path,
    compiler_dir: &str,
    linker_config: &LinkerConfig,
    arg: &Option<Args>,
) {
    let clib_path = format!("{}/include/clib", compiler_dir);

    if target_triple.abi == "msvc" {
        let machine = if target_triple.arch == "x86_64" {
            "/MACHINE:X64"
        } else if target_triple.arch == "x86" {
            "/MACHINE:X86"
        } else {
            "/MACHINE:ARM64"
        };

        // 确定入口点
        let entry = if target_triple.arch == "x86_64" {
            "mainCRTStartup"
        } else {
            "_mainCRTStartup"
        };

        // MSVC 链接器参数
        command
            .arg(lib_path.to_string_lossy().as_ref()) // 输入的静态库
            .arg(&format!(
                "{}:{}",
                linker_config.output_flag,
                output_path.to_string_lossy()
            ))
            .arg(&format!("{}:{}", linker_config.linkdir_flag, clib_path))
            .arg("libarc.lib") // 链接 arc 库
            .arg("legacy_stdio_definitions.lib")
            .arg("/SUBSYSTEM:CONSOLE")
            .arg("/DEFAULTLIB:msvcrt")
            .arg(machine)
            .arg(&format!("/ENTRY:{entry}")); // 标准控制台入口

        // 添加用户指定的库
        add_user_libraries_msvc(command, linker_config, arg);
    } else {
        // GNU 链接器参数（gcc）
        command
            .arg(lib_path.to_string_lossy().as_ref()) // 输入的静态库
            .arg(linker_config.output_flag.as_str())
            .arg(output_path.to_string_lossy().as_ref())
            .arg(linker_config.linkdir_flag.as_str())
            .arg(&clib_path)
            .arg(linker_config.library_flag.as_str())
            .arg("arc"); // 链接 arc 库

        // 添加用户指定的库
        add_user_libraries_gnu(command, linker_config, arg);
    }
}

/// 为 MSVC 添加用户指定的链接库
fn add_user_libraries_msvc(
    command: &mut dyn CommandLike,
    linker_config: &LinkerConfig,
    arg: &Option<Args>,
) {
    if let Some(args) = arg {
        // 添加库搜索路径
        for path in &args.link_with {
            if let Some(parent) = Path::new(path).parent() {
                command
                    .arg(&linker_config.linkdir_flag)
                    .arg(parent.to_string_lossy().as_ref());
            }
        }

        // 添加库文件名
        for lib in &args.link_with {
            if lib.trim().is_empty() {
                continue;
            }

            // 获取文件名成功说明用户给的是路径 获取失败说明用户给的是文件名
            let lib_name = Path::new(lib)
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| lib.clone());

            command.arg(&lib_name);
        }
    }
}

/// 为 GNU 添加用户指定的链接库（-l 风格）
fn add_user_libraries_gnu(
    command: &mut dyn CommandLike,
    linker_config: &LinkerConfig,
    arg: &Option<Args>,
) {
    if let Some(args) = arg {
        // 添加库搜索路径
        for path in &args.link_with {
            if let Some(parent) = Path::new(path).parent() {
                command
                    .arg(&linker_config.linkdir_flag)
                    .arg(parent.to_string_lossy().as_ref());
            }
        }

        // 添加库链接标志（去除可能的 lib 前缀和扩展名）
        for lib in &args.link_with {
            if lib.trim().is_empty() {
                continue;
            }

            // 获取文件名成功说明用户给的是路径 获取失败说明用户给的是文件名
            let stem = Path::new(lib)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| lib.clone());

            let name = stem.strip_prefix("lib").unwrap_or(&stem);
            command.arg(&format!("{}{}", linker_config.library_flag, name));
        }
    }
}

/// 添加平台特定的链接标志（仅适用于 GNU 风格）
pub fn add_platform_specific_flags(command: &mut dyn CommandLike) {
    #[cfg(target_os = "linux")]
    {
        command.arg("-static");
        command.arg("-lc");
        command.arg("-Wl,-z,noexecstack");
    }

    #[cfg(target_os = "windows")]
    {
        command.arg("-static");
        command.arg("-lmsvcrt");
    }

    #[cfg(target_os = "macos")]
    {
        command.arg("-fPIC");
    }
}

pub trait CommandLike {
    fn arg(&mut self, arg: &str) -> &mut dyn CommandLike;
    #[allow(unused)]
    fn status(&mut self) -> std::io::Result<std::process::ExitStatus>;
}

impl CommandLike for std::process::Command {
    fn arg(&mut self, arg: &str) -> &mut dyn CommandLike {
        self.arg(arg);
        self
    }

    fn status(&mut self) -> std::io::Result<std::process::ExitStatus> {
        self.status()
    }
}
