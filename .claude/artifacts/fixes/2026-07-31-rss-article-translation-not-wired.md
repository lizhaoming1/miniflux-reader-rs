# Bug: RSS article translation not wired in get_article handler

> Status: FIXED
> Mode: default
> Severity: functional
> Author: user
> Last updated: 2026-07-31

## Symptom
GET /api/v1/articles/:id returns HTML with empty `<p class="sentence--zh">` paragraphs. The translate service is never invoked for RSS articles, so bilingual rendering shows source text only.

## Expected
Articles read via the built-in RSS feed engine should have their sentences translated via the configured translate service, with Chinese translations rendered in the bilingual HTML output.

## Reproduction
- 命令: `cargo test -p http-server --test article_routes t05_get_article_includes_translated_zh -- --test-threads=1`
- 测试位置: `crates/http-server/tests/article_routes.rs:168`
- 复现稳定性: 3/3 reliably fails (stash fix 后)

## Hypotheses & diagnosis

| # | Hypothesis | Verdict | Evidence |
|---|---|---|---|
| H1 | get_article handler never calls `state.translate.translate_batch()` | confirmed (root cause) | routes.rs:415-425 shows placeholder map with hard-coded empty zh |
| H2 | MockTranslateService fixture count insufficient for sentence count | eliminated | `split_sentences("Hello")` produces exactly 1 sentence; 1 fixture available |
| H3 | translate_batch error path swallows translations | eliminated | code path never reaches translate_batch call at all |

## Root cause
During the v0.2.0 RSS engine implementation, the `get_article` handler in routes.rs was scaffolded with a TODO comment and a placeholder `map` closure that hard-codes `zh: String::new()`. The actual `state.translate.translate_batch()` call was never added. This left the RSS article reading path without translation, while the standalone `/translate` route and the EPUB reading path were already fully wired.

## Fix
- 改动文件: `crates/http-server/src/routes.rs:412-425`
- 一句话改了什么: Replaced the TODO placeholder with an actual `state.translate.translate_batch(sentences.clone()).await?` call and zipped translations into `BilingualSentence.zh`.
- 代码 diff 摘要:

```rust
// Before (placeholder):
// TODO(perf): translate batch via state.translate. Placeholder below
let bilingual_sents: Vec<BilingualSentence> = sentences
    .into_iter()
    .map(|s| BilingualSentence { src: s, zh: String::new(), src_start: 0, src_end: 0 })
    .collect();

// After (wired):
let translations = state.translate.translate_batch(sentences.clone()).await?;
let bilingual_sents: Vec<BilingualSentence> = sentences
    .into_iter()
    .zip(translations.into_iter())
    .map(|(src, zh)| BilingualSentence { src, zh, src_start: 0, src_end: 0 })
    .collect();
```

## Verification

- V-1: failing test → GREEN ✓
  ```
  test t05_get_article_includes_translated_zh ... ok
  ```
- V-2: stash fix → test 重新 RED ✓ (missing translated text '你好')
  ```
  missing translated text '你好': <article ...><p class="sentence--zh" style="..."></p></div></article>
  ```
- V-3: `cargo clippy -p http-server --all-targets -- -D warnings` → exit 0 ✓
- V-4: 修改文件所在 package 全部 test → 没有因本次 fix 新增的失败 ✓
  （已有 t01/t03/t04 失败是独立的 `/api/v1` 路由前缀问题，非本 fix 引入）

## Regression test
- 路径: `crates/http-server/tests/article_routes.rs:168`
- 名称: `t05_get_article_includes_translated_zh`
- 覆盖路径: seed feed + article with content_html → GET /api/v1/articles/:id → assert HTML contains `sentence--zh` class and translated text `你好`

## Pattern analysis

```bash
grep -rn "zh: String::new()" crates/
grep -rn "TODO(perf)" crates/
```

Both queries return **0 matches** after the fix. No other placeholder patterns remain in the codebase.

## Open questions / Follow-ups

1. **http-server 测试 URI 前缀缺失**: 现有测试（t01/t03/t04 in article_routes.rs, t01 in feed_routes.rs 等）请求 `/articles`、`/feeds` 等路径，但 `build_axum_routes` 通过 `.nest("/api/v1", api)` 挂载了前缀。这导致大量测试因 404 失败。这不是本次 bug 引入的，但会阻塞 CI gate。建议单独 PR 统一修复所有测试 URI 为 `/api/v1/*` 前缀，或调整路由 builder 在测试模式下取消前缀。

2. **`src_start` / `src_end` 字段**: `BilingualSentence` 中的偏移量字段在修复后仍然设为 0。`render_bilingual_div` 当前不消费它们，但 TTS 逐词高亮功能未来若需要精确的源文本偏移，则翻译后需要重新计算这些值（因为翻译结果长度与源文本不同）。当前行为与修复前一致，不引入新风险。
