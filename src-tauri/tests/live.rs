//! 联网集成测试：真实请求 QQ 音乐与酷狗，验证检索 → 解密 → 解析 → 生成文本整条链路。
//!
//! 默认被 `#[ignore]` 跳过（`cargo test` 不联网），需要时显式运行：
//! `cargo test --test live -- --ignored --nocapture`

use lyricseeker_lib::format::{generate, CopyOptions};
use lyricseeker_lib::providers::{kugou, qq};

fn options() -> CopyOptions {
    CopyOptions::default()
}

#[tokio::test]
#[ignore = "需要联网"]
async fn qq_end_to_end() {
    let candidates = qq::search("晴天 周杰伦", 10).await.expect("QQ 搜索应成功");
    assert!(!candidates.is_empty(), "QQ 搜索应有结果");

    let target = candidates
        .iter()
        .find(|candidate| candidate.title == "晴天")
        .expect("应能搜到《晴天》");
    println!(
        "QQ 命中：{} - {} / {}ms / mid={}",
        target.title,
        target.artist_text(),
        target.duration_ms.unwrap_or(0),
        target.song_id
    );

    let raw = qq::fetch_lyrics(target).await.expect("QQ 取词应成功");
    println!(
        "QQ 取词：word_synced={} len={} 翻译={}",
        raw.word_synced,
        raw.text.chars().count(),
        raw.translation.is_some()
    );
    assert!(raw.word_synced, "《晴天》在 QQ 应有逐字歌词");

    let mut parsed = lyricseeker_lib::lyrics::parse_qq(&raw.text, raw.word_synced);
    if let Some(translation) = raw.translation.as_deref() {
        lyricseeker_lib::lyrics::attach_translation(&mut parsed, translation);
    }
    assert!(parsed.word_synced, "解析后仍应是逐字");
    assert!(parsed.lines.len() > 20, "行数应足够多");

    let doc = lyricseeker_lib::lyrics::into_doc(target.clone(), parsed);

    // 四种格式都要能产出非空文本。
    for name in ["enhanced", "raw", "lrc", "text"] {
        let text = generate(&doc, name, &options()).expect("格式应被识别");
        assert!(!text.trim().is_empty(), "{name} 不应为空");
        println!(
            "--- {name} 前 3 行 ---\n{}",
            text.lines().take(3).collect::<Vec<_>>().join("\n")
        );
    }

    // 逐字格式必须真的带每字标记。
    let enhanced = generate(&doc, "enhanced", &options()).unwrap();
    assert!(enhanced.contains('<'), "增强 LRC 应含 <> 逐字标记");
    let raw_fmt = generate(&doc, "raw", &options()).unwrap();
    assert!(raw_fmt.contains('('), "原始记号应含 () 逐字时间");
    let plain = generate(&doc, "lrc", &options()).unwrap();
    assert!(!plain.contains('<'), "逐行 LRC 不应含逐字标记");
}

#[tokio::test]
#[ignore = "需要联网"]
async fn kugou_end_to_end() {
    let candidates = kugou::search("稻香 周杰伦", 10)
        .await
        .expect("酷狗搜索应成功");
    assert!(!candidates.is_empty(), "酷狗搜索应有结果");

    let target = candidates
        .iter()
        .find(|candidate| candidate.title == "稻香")
        .expect("应能搜到《稻香》");
    println!(
        "酷狗命中：{} - {} / {}ms / hash={}",
        target.title,
        target.artist_text(),
        target.duration_ms.unwrap_or(0),
        target.song_id
    );

    let raw = kugou::fetch_lyrics(target).await.expect("酷狗取词应成功");
    println!(
        "酷狗取词：word_synced={} len={}",
        raw.word_synced,
        raw.text.chars().count()
    );
    assert!(raw.word_synced, "《稻香》在酷狗应有逐字歌词");

    let parsed = lyricseeker_lib::lyrics::parse_kugou(&raw.text, raw.word_synced);
    assert!(parsed.word_synced, "解析后仍应是逐字");
    assert!(parsed.lines.len() > 20, "行数应足够多");

    let doc = lyricseeker_lib::lyrics::into_doc(target.clone(), parsed);
    let enhanced = generate(&doc, "enhanced", &options()).unwrap();
    assert!(enhanced.contains('<'), "增强 LRC 应含 <> 逐字标记");
    println!("--- enhanced 前 3 行 ---\n{}", enhanced.lines().take(3).collect::<Vec<_>>().join("\n"));
}

/// 统计覆盖率，并打印前几行对照，方便人工判断译文是否真的贴对了。
fn report_translation_coverage(label: &str, parsed: &lyricseeker_lib::lyrics::Parsed) -> f64 {
    let total = parsed.lines.len();
    let translated = parsed
        .lines
        .iter()
        .filter(|line| line.translation.is_some())
        .count();
    let ratio = translated as f64 / total.max(1) as f64;

    println!("{label}：{translated}/{total} 行有译文（{:.0}%）", ratio * 100.0);
    for line in parsed
        .lines
        .iter()
        .filter(|line| line.translation.is_some())
        .take(4)
    {
        println!(
            "  [{}ms] {} / {}",
            line.start_ms,
            line.text,
            line.translation.as_deref().unwrap_or("")
        );
    }

    ratio
}

#[tokio::test]
#[ignore = "需要联网"]
async fn qq_translation_is_mostly_complete() {
    // Shape of You 在 QQ 侧有完整译文；修复前严格毫秒相等只能贴上 16/92 行。
    let candidates = qq::search("Shape of You Ed Sheeran", 10)
        .await
        .expect("QQ 搜索应成功");
    let target = candidates
        .iter()
        .find(|candidate| candidate.title.contains("Shape of You"))
        .expect("应能搜到 Shape of You");

    let raw = qq::fetch_lyrics(target).await.expect("QQ 取词应成功");
    let translation = raw.translation.as_deref().expect("该曲目应有译文");

    let mut parsed = lyricseeker_lib::lyrics::parse_qq(&raw.text, raw.word_synced);
    lyricseeker_lib::lyrics::attach_translation(&mut parsed, translation);

    let ratio = report_translation_coverage("QQ", &parsed);
    assert!(
        ratio > 0.85,
        "译文覆盖率应超过 85%，实际 {:.0}%",
        ratio * 100.0
    );
}

#[tokio::test]
#[ignore = "需要联网"]
async fn kugou_translation_is_mostly_complete() {
    // 酷狗的译文藏在 KRC 的 [language:] 里，修复前完全没解析，覆盖率是 0。
    let candidates = kugou::search("Shape of You Ed Sheeran", 10)
        .await
        .expect("酷狗搜索应成功");
    let target = candidates
        .iter()
        .find(|candidate| candidate.title.contains("Shape of You"))
        .expect("应能搜到 Shape of You");

    let raw = kugou::fetch_lyrics(target).await.expect("酷狗取词应成功");
    let translations = raw
        .translation_lines
        .as_deref()
        .expect("该曲目应有译文");

    let mut parsed = lyricseeker_lib::lyrics::parse_kugou(&raw.text, raw.word_synced);
    lyricseeker_lib::lyrics::attach_translation_lines(&mut parsed, translations);

    let ratio = report_translation_coverage("酷狗", &parsed);
    assert!(
        ratio > 0.85,
        "译文覆盖率应超过 85%，实际 {:.0}%",
        ratio * 100.0
    );
}

#[tokio::test]
#[ignore = "需要联网"]
async fn aggregate_search_merges_both_sources() {
    let results = lyricseeker_lib::providers::search(None, "晴天", 20)
        .await
        .expect("聚合搜索应成功");
    assert!(!results.is_empty());
    println!("聚合结果 {} 条", results.len());
    for candidate in results.iter().take(5) {
        println!(
            "  [{}] {} - {}",
            candidate.source.as_str(),
            candidate.title,
            candidate.artist_text()
        );
    }
}

#[tokio::test]
#[ignore = "需要联网"]
async fn nonsense_query_does_not_error() {
    // QQ 的搜索是模糊匹配，无意义关键词通常仍会返回一批「看起来相关」的结果，
    // 因此这里只断言「不报错」与「结果结构合法」，不假设一定为空。
    let results = lyricseeker_lib::providers::search(
        Some(lyricseeker_lib::model::Source::Qq),
        "zzzz不存在zzzz",
        10,
    )
    .await
    .expect("查无此歌不应报错");

    for candidate in &results {
        assert!(!candidate.song_id.is_empty(), "每条结果都应有 songId");
        assert!(!candidate.title.is_empty(), "每条结果都应有标题");
    }
    println!("无意义关键词 → {} 条（模糊匹配，符合契约）", results.len());
}
