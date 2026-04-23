use std::env;
use std::process::Command;

fn run(cmd: &str, args: &[&str]) -> String {
    Command::new(cmd)
        .args(args)
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn main() {
    let target = env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());
    let host = env::var("HOST").unwrap_or_else(|_| "unknown".to_string());
    let profile = env::var("PROFILE").unwrap_or_else(|_| "unknown".to_string());

    // Prefer GITHUB_SHA injected by CI (first 7 hex chars); fall back to local git.
    let git_commit = env::var("GITHUB_SHA")
        .ok()
        .filter(|s| s.len() >= 7)
        .map(|s| s[..7].to_string())
        .unwrap_or_else(|| run("git", &["rev-parse", "--short", "HEAD"]));
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");
    let build_date = run("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"]);

    // -----------------------------------------------------------------------
    // Build-machine hardware fingerprint
    // Collected at compile time so the binary records where it was built.
    // Works on Linux (CI runners and local Linux), macOS (local dev), and
    // inside Docker build contexts.
    // -----------------------------------------------------------------------

    // --- CPU logical core count ---
    // Linux: nproc  |  macOS: sysctl -n hw.logicalcpu
    let cpu_cores = if !run("nproc", &[]).is_empty() && run("nproc", &[]) != "unknown" {
        run("nproc", &[])
    } else {
        run("sysctl", &["-n", "hw.logicalcpu"])
    };

    // --- CPU model name ---
    // Linux: /proc/cpuinfo  |  macOS: sysctl brand_string
    let cpu_model = {
        // Try macOS first
        let mac = run("sysctl", &["-n", "machdep.cpu.brand_string"]);
        if mac != "unknown" && !mac.is_empty() {
            mac
        } else {
            // Linux: grep first "model name" line from /proc/cpuinfo
            let linux = Command::new("sh")
                .args(&["-c", "grep -m1 'model name' /proc/cpuinfo | cut -d: -f2"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "unknown".to_string());
            if linux.is_empty() { "unknown".to_string() } else { linux }
        }
    };

    // --- Total RAM (human-readable) ---
    // Linux: /proc/meminfo MemTotal in kB → convert to GiB
    // macOS: sysctl hw.memsize in bytes → convert to GiB
    let total_ram = {
        // macOS
        let mac_bytes: Option<u64> = run("sysctl", &["-n", "hw.memsize"]).parse().ok();
        if let Some(bytes) = mac_bytes {
            format!("{:.1} GiB", bytes as f64 / (1024.0_f64).powi(3))
        } else {
            // Linux: read MemTotal kB from /proc/meminfo
            let kb: Option<u64> = Command::new("sh")
                .args(&["-c", "grep MemTotal /proc/meminfo | awk '{print $2}'"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .and_then(|s| s.trim().parse().ok());
            match kb {
                Some(k) => format!("{:.1} GiB", k as f64 / (1024.0 * 1024.0)),
                None => "unknown".to_string(),
            }
        }
    };

    // --- Total disk size of the root filesystem (human-readable) ---
    // df -BG / gives columns in GiB on Linux; -g on macOS gives GiB
    let total_disk = {
        // Try GNU df -BG (Linux / CI)
        let gnu = Command::new("sh")
            .args(&["-c", "df -BG / 2>/dev/null | awk 'NR==2{gsub(/G/,\" GiB\",$2); print $2}'"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        if !gnu.is_empty() && gnu != "unknown" {
            gnu
        } else {
            // macOS df -g
            Command::new("sh")
                .args(&["-c", "df -g / 2>/dev/null | awk 'NR==2{print $2 \" GiB\"}'"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "unknown".to_string())
        }
    };

    // -----------------------------------------------------------------------
    // Export all env vars to the compiled binary
    // -----------------------------------------------------------------------
    println!("cargo:rustc-env=BUILD_TARGET={}", target);
    println!("cargo:rustc-env=BUILD_HOST={}", host);
    println!("cargo:rustc-env=BUILD_PROFILE={}", profile);
    println!("cargo:rustc-env=GIT_COMMIT={}", git_commit);
    println!("cargo:rustc-env=BUILD_DATE={}", build_date);
    println!("cargo:rustc-env=BUILD_CPU_CORES={}", cpu_cores);
    println!("cargo:rustc-env=BUILD_CPU_MODEL={}", cpu_model);
    println!("cargo:rustc-env=BUILD_RAM={}", total_ram);
    println!("cargo:rustc-env=BUILD_DISK={}", total_disk);

    // Rerun only when git HEAD changes to avoid unnecessary rebuilds
    println!("cargo:rerun-if-changed=.git/HEAD");
}
