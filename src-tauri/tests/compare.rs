//! 与 LDDC 范例文件逐行比对（手动跑：cargo test --test compare -- --ignored --nocapture）
//!
//! 范例来源：用户提供的
//!   D:\Documents\Lyrics\周杰伦 - 晴天 (增强逐字).lrc
//!   D:\Documents\Lyrics\周杰伦 - 晴天 (普通逐字).lrc
//!   D:\Documents\Lyrics\周杰伦 - 晴天 (逐行).lrc

use lyricseeker_lib::format::{generate, CopyOptions};
use lyricseeker_lib::model::Source;
use lyricseeker_lib::providers;

const REF_DIR: &str = r"D:\Documents\Lyrics";

/// 读范例文件的正文行（去掉 `[ti:]` 之类的头部标签与空行）；文件不存在时返回 `None`。
fn body(path: &str) -> Option<Vec<String>> {
    let text = std::fs::read_to_string(path).ok()?;
    Some(
        text.lines()
            .map(|line| line.trim_end_matches('\r').to_string())
            .filter(|line| {
                // 头部标签形如 `[ti:晴天]`，方括号里第一个字符不是数字；
                // 正文行形如 `[00:00.000]…`，第一个字符是数字。
                let b = line.as_bytes();
                if b.first() == Some(&b'[') {
                    return b.get(1).is_some_and(u8::is_ascii_digit);
                }
                !line.trim().is_empty()
            })
            .collect(),
    )
}

#[tokio::test]
#[ignore = "需要联网 + 需要本机范例文件"]
async fn matches_lddc_reference() {
    let candidates = providers::qq::search("晴天 周杰伦", 10).await.expect("搜索");
    let target = candidates
        .iter()
        .find(|c| c.title == "晴天" && c.artists.iter().any(|a| a.contains("周杰伦")))
        .expect("找到《晴天》");

    let raw = providers::qq::fetch_lyrics(target).await.expect("取词");
    let parsed = lyricseeker_lib::lyrics::parse_qq(&raw.text, raw.word_synced);
    let doc = lyricseeker_lib::lyrics::into_doc(target.clone(), parsed);
    assert_eq!(target.source, Source::Qq);

    let options = CopyOptions::default();
    let cases = [
        ("enhanced", "周杰伦 - 晴天 (增强逐字).lrc"),
        ("plain", "周杰伦 - 晴天 (普通逐字).lrc"),
        ("lrc", "周杰伦 - 晴天 (逐行).lrc"),
    ];

    for (format, file) in cases {
        let got: Vec<String> = generate(&doc, format, &options)
            .expect("格式")
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.trim_end_matches('\r').to_string())
            .collect();
        let Some(want) = body(&format!("{REF_DIR}\\{file}")) else {
            println!("[{format}] 跳过：本机没有范例文件 {file}");
            continue;
        };

        let same = got.len() == want.len() && got.iter().zip(want.iter()).all(|(a, b)| a == b);
        if same {
            println!("[{format}] 与范例完全一致，{} 行", got.len());
            continue;
        }

        println!("[{format}] 不一致：我们 {} 行，范例 {} 行", got.len(), want.len());
        let mut shown = 0;
        for index in 0..got.len().max(want.len()) {
            let a = got.get(index).map(String::as_str).unwrap_or("<缺>");
            let b = want.get(index).map(String::as_str).unwrap_or("<缺>");
            if a != b {
                println!("  行{index}:\n    我们: {a}\n    范例: {b}");
                shown += 1;
                if shown >= 3 {
                    break;
                }
            }
        }
        panic!("{format} 与范例不符");
    }
}
