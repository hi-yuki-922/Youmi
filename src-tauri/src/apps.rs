//! 开始菜单应用索引：后台线程扫描开始菜单快捷方式（.lnk），供命令面板模糊搜索后启动。
//!
//! 扫描结果常驻内存缓存，呼起面板只读快照——构建与刷新都在后台线程，不拖慢面板首开。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::State;

use crate::platform;

/// 快捷方式条目：名称取 .lnk 文件名去扩展名（开始菜单的文件名即面向用户的显示名，含中文）。
#[derive(Debug, Clone, Serialize)]
pub struct AppEntry {
    pub name: String,
    pub path: String,
}

/// 应用列表快照：`loading` = 后台首扫尚未完成（还没有任何数据）。
#[derive(Debug, Serialize)]
pub struct AppsSnapshot {
    pub apps: Vec<AppEntry>,
    pub loading: bool,
}

/// 缓存刷新间隔：呼起面板时超龄即在后台重扫，新装应用下次呼起可见。
const REFRESH_TTL: Duration = Duration::from_secs(5 * 60);

/// 目录递归深度上限：开始菜单层级很浅，仅防御目录环等异常。
const MAX_SCAN_DEPTH: usize = 8;

/// Clone 句柄：后台扫描线程与 tauri state 各持一份。
#[derive(Clone, Default)]
pub struct AppsState {
    apps: Arc<RwLock<Vec<AppEntry>>>,
    last_scan: Arc<Mutex<Option<Instant>>>,
    scanning: Arc<AtomicBool>,
}

/// 应用列表快照；缓存超龄时顺带触发后台重扫。零 IO，面板呼起路径上安全。
#[tauri::command]
pub fn apps_query(state: State<'_, AppsState>) -> AppsSnapshot {
    ensure_fresh(&state);
    let apps = state.apps.read().unwrap().clone();
    AppsSnapshot {
        loading: state.last_scan.lock().unwrap().is_none(),
        apps,
    }
}

/// 启动快捷方式：校验后交 shell 语义执行；放阻塞线程池，不占主线程与 async runtime。
#[tauri::command]
pub async fn apps_launch(path: String) -> Result<(), String> {
    let path = PathBuf::from(&path);
    if !path.is_file() {
        return Err(format!("快捷方式不存在: {}", path.display()));
    }
    if !is_lnk(&path) {
        return Err(format!("不是开始菜单快捷方式: {}", path.display()));
    }
    tauri::async_runtime::spawn_blocking(move || {
        platform::launch_lnk(path.to_string_lossy().as_ref()).map_err(|e| e.0)
    })
    .await
    .map_err(|e| format!("启动任务执行失败: {e}"))?
}

/// 缓存超龄或为空时在后台线程重扫；进行中的扫描用 scanning 标志去重。
pub fn ensure_fresh(state: &AppsState) {
    let stale = match *state.last_scan.lock().unwrap() {
        Some(at) => at.elapsed() >= REFRESH_TTL,
        None => true,
    };
    // compare_exchange 原子认领：并发调用（呼起竞态）只放行一个扫描线程
    if !stale
        || state
            .scanning
            .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
            .is_err()
    {
        return;
    }
    let worker = state.clone();
    let spawned = std::thread::Builder::new()
        .name("apps-scan".into())
        .spawn(move || {
            // 无论扫描成败（含 panic）都在线程退出时释放标志，刷新不会永久卡死
            let _done = ScanGuard(worker.clone());
            let apps = scan(&start_menu_roots());
            *worker.apps.write().unwrap() = apps;
            *worker.last_scan.lock().unwrap() = Some(Instant::now());
        });
    if spawned.is_err() {
        state.scanning.store(false, Ordering::Relaxed);
    }
}

/// 线程退出时释放 scanning 标志（Drop 在 panic 展开时同样执行）。
struct ScanGuard(AppsState);

impl Drop for ScanGuard {
    fn drop(&mut self) {
        self.0.scanning.store(false, Ordering::Relaxed);
    }
}

/// 用户级与系统级开始菜单 Programs 目录；rank 小者优先（同名时用户快捷方式覆盖系统级）。
fn start_menu_roots() -> Vec<(PathBuf, u8)> {
    let mut roots = Vec::new();
    if let Some(appdata) = std::env::var_os("APPDATA") {
        roots.push((
            PathBuf::from(appdata).join(r"Microsoft\Windows\Start Menu\Programs"),
            0,
        ));
    }
    if let Some(program_data) = std::env::var_os("ProgramData") {
        roots.push((
            PathBuf::from(program_data).join(r"Microsoft\Windows\Start Menu\Programs"),
            1,
        ));
    }
    roots
}

struct ScannedLink {
    name: String,
    path: String,
    rank: u8,
}

fn scan(roots: &[(PathBuf, u8)]) -> Vec<AppEntry> {
    let mut links = Vec::new();
    for (dir, rank) in roots {
        collect_dir(dir, *rank, 0, &mut links);
    }
    dedupe(links)
}

fn collect_dir(dir: &Path, rank: u8, depth: usize, out: &mut Vec<ScannedLink>) {
    if depth > MAX_SCAN_DEPTH {
        return;
    }
    // 目录不存在/不可读：该来源按空处理
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            collect_dir(&path, rank, depth + 1, out);
        } else if is_lnk(&path) {
            let Some(stem) = path.file_stem() else {
                continue;
            };
            out.push(ScannedLink {
                name: stem.to_string_lossy().into_owned(),
                path: path.to_string_lossy().into_owned(),
                rank,
            });
        }
    }
}

/// .lnk 快捷方式判定（扩展名大小写不敏感）。
fn is_lnk(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("lnk"))
}

/// 同名去重：用户目录快捷方式覆盖系统级同名项（开始菜单自身的语义）；
/// 同 rank 取路径较小者——去重结果与文件系统枚举顺序无关。输出按名称排序保证列表稳定。
fn dedupe(links: Vec<ScannedLink>) -> Vec<AppEntry> {
    let mut selected: HashMap<String, (u8, AppEntry)> = HashMap::new();
    for link in links {
        let key = link.name.to_lowercase();
        match selected.get_mut(&key) {
            Some((rank, app)) => {
                if *rank > link.rank || (*rank == link.rank && app.path > link.path) {
                    *rank = link.rank;
                    *app = AppEntry {
                        name: link.name,
                        path: link.path,
                    };
                }
            }
            None => {
                selected.insert(
                    key,
                    (
                        link.rank,
                        AppEntry {
                            name: link.name,
                            path: link.path,
                        },
                    ),
                );
            }
        }
    }
    let mut apps: Vec<AppEntry> = selected.into_values().map(|(_, app)| app).collect();
    apps.sort_by(|a, b| a.name.cmp(&b.name));
    apps
}

#[cfg(test)]
mod tests {
    use super::{dedupe, scan, ScannedLink};
    use crate::test_support::TempDir;

    fn link(name: &str, path: &str, rank: u8) -> ScannedLink {
        ScannedLink {
            name: name.into(),
            path: path.into(),
            rank,
        }
    }

    #[test]
    fn dedupe_prefers_user_root_over_common() {
        let apps = dedupe(vec![
            link("微信", r"C:\common\微信.lnk", 1),
            link("微信", r"C:\user\微信.lnk", 0),
        ]);
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].path, r"C:\user\微信.lnk");
    }

    #[test]
    fn dedupe_is_case_insensitive_first_wins_within_root() {
        let apps = dedupe(vec![
            link("WeChat", r"C:\user\WeChat.lnk", 0),
            link("wechat", r"C:\user\wechat.lnk", 0),
            link("Edge", r"C:\user\Edge.lnk", 0),
        ]);
        let names: Vec<&str> = apps.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, vec!["Edge", "WeChat"]);
        assert_eq!(apps[1].path, r"C:\user\WeChat.lnk");
    }

    #[test]
    fn dedupe_same_rank_takes_smaller_path_regardless_of_order() {
        let reversed = dedupe(vec![
            link("App", r"C:\z\dir\App.lnk", 0),
            link("App", r"C:\a\dir\App.lnk", 0),
        ]);
        let forward = dedupe(vec![
            link("App", r"C:\a\dir\App.lnk", 0),
            link("App", r"C:\z\dir\App.lnk", 0),
        ]);
        assert_eq!(reversed[0].path, r"C:\a\dir\App.lnk");
        assert_eq!(forward[0].path, r"C:\a\dir\App.lnk");
    }

    #[test]
    fn scan_walks_nested_dirs_and_keeps_only_lnk() {
        let dir = TempDir::new("apps-scan");
        let deep = dir.path().join("a").join("b");
        std::fs::create_dir_all(&deep).unwrap();
        std::fs::write(dir.path().join("记事本.lnk"), b"").unwrap();
        std::fs::write(dir.path().join("readme.txt"), b"").unwrap();
        std::fs::write(deep.join("WeChat.LNK"), b"").unwrap(); // 扩展名大小写不敏感

        let apps = scan(&[(dir.path().to_path_buf(), 0)]);
        let names: Vec<&str> = apps.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, vec!["WeChat", "记事本"]); // 按名称排序
        assert!(apps[1].path.ends_with("记事本.lnk"));
    }

    #[test]
    fn scan_dedupes_across_roots_by_rank() {
        let user = TempDir::new("apps-user");
        let common = TempDir::new("apps-common");
        std::fs::write(user.path().join("微信.lnk"), b"").unwrap();
        std::fs::write(common.path().join("微信.lnk"), b"").unwrap();
        std::fs::write(common.path().join("独有.lnk"), b"").unwrap();

        let apps = scan(&[
            (user.path().to_path_buf(), 0),
            (common.path().to_path_buf(), 1),
        ]);
        assert_eq!(apps.len(), 2);
        let by_name = |name: &str| {
            apps.iter()
                .find(|a| a.name == name)
                .unwrap_or_else(|| panic!("缺少 {name}"))
        };
        assert_eq!(
            by_name("微信").path,
            user.path().join("微信.lnk").to_string_lossy().to_string()
        );
        assert!(by_name("独有").path.ends_with("独有.lnk"));
    }

    #[test]
    fn scan_ignores_missing_roots() {
        let dir = TempDir::new("apps-missing");
        assert!(scan(&[(dir.path().join("nope"), 0)]).is_empty());
    }
}
