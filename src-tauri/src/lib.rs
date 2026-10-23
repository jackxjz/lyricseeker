//! LyricSeeker —— Windows 逐字歌词检索与复制工具。
//!
//! 分层：
//! - `providers`：QQ 音乐 / 酷狗 HTTP 接口
//! - `lyrics`：解密后的正文 → 统一 DTO
//! - `format`：四种复制格式
//! - `cache`：磁盘缓存
//! - `commands`：暴露给前端的 Tauri 命令

pub mod cache;
pub mod commands;
pub mod format;
pub mod lyrics;
pub mod model;
pub mod providers;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();

    // 单实例：第二次启动只把已有窗口带到前台。
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
        commands::show_main_window(app);
    }));

    builder
        .plugin(
            // 只写 stdout：日志文件落在系统目录，在受限环境下会因权限被拒，
            // 而插件初始化失败会直接让整个应用起不来，代价太大。
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .targets([tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Stdout,
                )])
                .build(),
        )
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            register_shortcut(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::search,
            commands::fetch_lyrics,
            commands::generate_text,
            commands::clear_cache,
            commands::cache_info,
            commands::set_always_on_top,
        ])
        .run(tauri::generate_context!())
        .expect("LyricSeeker 启动失败");
}

/// 注册全局快捷键 `Ctrl+Alt+L`：在「显示并聚焦」与「隐藏」之间切换。
fn register_shortcut(app: &tauri::AppHandle) {
    use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

    let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyL);
    let handle = app.clone();

    let result = app.global_shortcut().on_shortcut(shortcut, move |_app, _shortcut, event| {
        if event.state() != ShortcutState::Pressed {
            return;
        }
        if let Some(window) = handle.get_webview_window("main") {
            let visible = window.is_visible().unwrap_or(false);
            let focused = window.is_focused().unwrap_or(false);
            if visible && focused {
                commands::hide_main_window(&handle);
            } else {
                commands::show_main_window(&handle);
            }
        }
    });

    if let Err(error) = result {
        // 快捷键被别的程序占用时不应让应用起不来。
        log::warn!("注册全局快捷键 Ctrl+Alt+L 失败：{error}");
    }
}
