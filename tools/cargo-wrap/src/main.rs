//! cargo 代理：收敛 rustc 并发、结构化日志、进度条展示。
//!
//! 用法：
//!   cargo-wrap build --features demo
//!   cargo-wrap --release run
//! 并行度来源优先级：命令行 `-j/--jobs` > 环境变量 `CARGO_WRAP_JOBS` > 默认 1（串行）。
//!
//! 设计说明（为什么这样写）：
//! - 通过命令行注入 `-j N` 强制限制 rustc 并发，避免任务管理器散落几十个进程；
//! - 用管道接管 cargo stdout/stderr 逐行解析，把冗杂原始输出折叠成结构化状态行；
//! - Windows 下尽力把整个进程树挂入 Job Object，便于统一终止（失败仅告警不阻断）；
//! - 进度条依据 "Compiling x (n/m)" 里的 m 估算总数，拿不到就按出现次数自适应累计。

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;

use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;

#[cfg(windows)]
mod job_object {
    //! Windows Job Object：把 cargo 及所有 rustc 后代进程归入同一作业，便于统一管理/终止。
    //! 这是"尽力而为"：权限不足等环境下仅打印警告，绝不阻塞主流程。

    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, IsProcessInJob, TerminateJobObject,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcess;

    /// 持有命名 job（存为 usize 以便 Sync）；cargo 及其 rustc 后代借此共享同一「Rust 编译器」归组。
    static JOB: std::sync::OnceLock<usize> = std::sync::OnceLock::new();

    /// 创建命名作业并挂入当前进程。子进程（cargo、rustc）自动继承归属同一 job，
    /// 任务管理器据此把整棵编译树折叠为「Rust 编译器」分组。失败仅告警不阻断。
    pub fn group_process_tree_into_job() {
        unsafe {
            let name: Vec<u16> = "Rust 编译器\0".encode_utf16().collect();
            let job: HANDLE = CreateJobObjectW(std::ptr::null(), name.as_ptr());
            if job.is_null() {
                eprintln!("[warn] 创建 Job Object 失败，跳过归组");
                return;
            }
            if AssignProcessToJobObject(job, GetCurrentProcess()) == 0 {
                eprintln!("[warn] 将当前进程挂入 Job Object 失败");
                return;
            }
            let _ = JOB.set(job as usize);
            if in_job() {
                println!("[job] 编译树已归入「Rust 编译器」命名作业");
            } else {
                eprintln!("[warn] 未能验证进程归组");
            }
        }
    }

    /// 当前进程是否已处于任一 Windows Job Object 中（含继承）。
    fn in_job() -> bool {
        unsafe {
            let mut result: i32 = 0;
            // job 参数传 NULL 表示"查询是否位于任一作业"
            IsProcessInJob(GetCurrentProcess(), std::ptr::null_mut(), &mut result) != 0 && result != 0
        }
    }

    /// 尝试终止整个编译树（job 内所有进程）。
    /// 仅供用户在需要整体停止时调用；当前 main 未暴露触发入口，预留能力。
    #[allow(dead_code)]
    pub fn terminate_tree() -> bool {
        match JOB.get() {
            Some(&job) if job != 0 => unsafe { TerminateJobObject(job as HANDLE, 1) != 0 },
            _ => false,
        }
    }
}

/// 从参数中确定并行度，并构造真正传给 cargo 的参数列表。
/// 已存在的 `-j/--jobs` 会优先采用；否则按 Cargo 约定插入 `-j N` 到子命令之前。
fn resolve_jobs(args: &[String]) -> (u32, Vec<String>) {
    // 默认 1：串行编译，任务管理器进程页始终只有 1 个 rustc，避免后台进程区被平铺淹屏。
    let default: u32 = 1;
    let mut cli_jobs: Option<u32> = None;
    let mut forward: Vec<String> = Vec::with_capacity(args.len());

    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if a == "-j" || a == "--jobs" {
            if let Some(v) = args.get(i + 1).and_then(|s| s.parse::<u32>().ok()) {
                cli_jobs = Some(v);
                i += 2;
                continue;
            }
        } else if let Some(rest) = a.strip_prefix("--jobs=") {
            if let Ok(v) = rest.parse::<u32>() {
                cli_jobs = Some(v);
                i += 1;
                continue;
            }
        }
        forward.push(a.clone());
        i += 1;
    }

    let jobs = cli_jobs
        .or_else(|| std::env::var("CARGO_WRAP_JOBS").ok().and_then(|s| s.parse().ok()))
        .unwrap_or(default);

    if cli_jobs.is_none() {
        // cargo 的 -j 是全局选项，须放在子命令之后（cargo check -j4）。
        // 它允许出现在参数任意位置，故直接追加到末尾，避免插入到子命令前。
        forward.push(format!("-j{jobs}"));
    }
    (jobs, forward)
}

fn main() {
    #[cfg(windows)]
    job_object::group_process_tree_into_job();

    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("用法: cargo-wrap [cargo 子命令及参数...]（如 build --features demo）");
        std::process::exit(2);
    }

    let (jobs, forward) = resolve_jobs(&args);
    println!("{} cargo {}（并行度 {}）", "[cargo]".cyan().bold(), forward.join(" ").dimmed(), jobs);

    let mut cmd = Command::new("cargo");
    cmd.args(&forward).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{} 无法启动 cargo: {e}", "[error]".red().bold());
            std::process::exit(1);
        }
    };

    let (tx, rx) = mpsc::channel::<Event>();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    spawn_reader(stdout, tx.clone(), false);
    spawn_reader(stderr, tx, true);

    // 正则合集：把原始输出归类为"分析/缓存/编译进度/完成/错误/警告"。
    let re_compiling = Regex::new(r"^\s*(Compiling|Checking)\s+(.+?)(?:\s+\((\d+)/(\d+)\))?\s*$").unwrap();
    let re_error = Regex::new(r"^\s*error(?:\[[A-Za-z0-9]+\])?\s*[: ]").unwrap();
    let re_warning = Regex::new(r"^\s*warning(?:[:\s])").unwrap();
    let re_finished = Regex::new(r"^\s*Finished").unwrap();
    let re_fresh = Regex::new(r"^\s*Fresh\s+").unwrap();
    let re_phase = Regex::new(r"^\s*(Updating|Downloading|Blocking waiting|Resolving)").unwrap();

    let pb = ProgressBar::new(0);
    pb.set_style(
        ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] {msg} {wide_bar:.cyan/blue} {pos}/{len}")
            .unwrap()
            .progress_chars("=>-"),
    );
    pb.set_message("等待 cargo 输出...".dimmed().to_string());

    let mut done: u64 = 0;
    let mut total: Option<u64> = None;

    while let Ok(ev) = rx.recv() {
        match ev {
            Event::Line(line, is_stderr) => {
                if re_phase.is_match(&line) {
                    pb.set_message(format!("[{}] {}", "分析".dimmed(), line.trim().bright_black()));
                } else if re_fresh.is_match(&line) {
                    pb.set_message(format!("[{}] {}", "缓存".dimmed(), line.trim().bright_green()));
                } else if let Some(cap) = re_compiling.captures(&line) {
                    let kind = cap.get(1).unwrap().as_str();
                    let crate_name = cap.get(2).unwrap().as_str();
                    if let (Some(n), Some(m)) = (cap.get(3), cap.get(4)) {
                        let n = n.as_str().parse().unwrap_or(0);
                        let m: u64 = m.as_str().parse().unwrap_or(0);
                        done = n;
                        if total != Some(m) {
                            total = Some(m);
                            pb.set_length(m);
                        }
                        pb.set_position(done);
                        pb.set_message(format!(
                            "[{}] {} ({}/{})",
                            kind.bold().cyan(),
                            crate_name.bright_white(),
                            done,
                            total.map_or(done, |t| t)
                        ));
                    } else {
                        done += 1;
                        pb.inc(1);
                        pb.set_message(format!(
                            "[{}] {} ({}/{})",
                            kind.bold().cyan(),
                            crate_name.bright_white(),
                            done,
                            total.map_or(done, |t| t)
                        ));
                    }
                } else if re_finished.is_match(&line) {
                    pb.finish_with_message(format!("{} {}", "[完成]".green().bold(), line.trim()));
                } else if is_stderr {
                    if re_error.is_match(&line) {
                        eprintln!("{}", line.red().bold());
                    } else if re_warning.is_match(&line) {
                        eprintln!("{}", line.yellow());
                    } else if !line.trim().is_empty() {
                        eprintln!("{}", line.dimmed());
                    }
                } else if !line.trim().is_empty() {
                    println!("{}", line.dimmed());
                }
            }
        }
    }

    // reader 线程随 cargo 的 stdout/stderr 关闭而结束、通道 drop；
    // 收尾时 wait() 拿到真实退出码（0 成功 / 其余为编译错误等失败码）。
    let exit_code = child
        .wait()
        .ok()
        .and_then(|s| s.code())
        .unwrap_or(-1);

    pb.finish_and_clear();
    let status = if exit_code == 0 {
        "0（成功）".bold().to_string()
    } else {
        exit_code.to_string().red().bold().to_string()
    };
    println!("{} cargo 退出码: {}", "[结果]".green(), status);
    std::process::exit(exit_code);
}

/// 从 cargo 的 stdout 或 stderr 逐行读取并转发为事件。
fn spawn_reader<T>(stream: Option<T>, tx: mpsc::Sender<Event>, is_stderr: bool)
where
    T: std::io::Read + Send + 'static,
{
    thread::spawn(move || {
        let Some(stream) = stream else { return };
        let reader = BufReader::new(stream);
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    if tx.send(Event::Line(l, is_stderr)).is_err() {
                        return;
                    }
                }
                Err(_) => return,
            }
        }
    });
}

/// 事件：一行输出（是否来自 stderr）。
enum Event {
    Line(String, bool),
}