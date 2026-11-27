//! Dictionary-based local preview, deliberately separate from query compilation.
use crate::{
    error::{AppError, AppResult},
    syntax,
};
use anyhow::{bail, Context};
use es_keyword_query::Query;
use jieba_rs::{Jieba, TokenizeMode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs::File,
    io::{Cursor, Read},
    path::Path,
    sync::{Arc, OnceLock},
};

const BASE: &[u8] = include_bytes!("../../../data/segmentation/dict.txt.big");
const TAIWAN: &str = include_str!("../../../data/segmentation/taiwan.txt");
const MAX_DICTIONARY_BYTES: usize = 1024 * 1024;
static BASE_ANALYZER: OnceLock<Result<Arc<Jieba>, String>> = OnceLock::new();

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AnalysisMode {
    #[default]
    Normal,
    Search,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnalysisInput {
    pub expression: String,
    #[serde(default)]
    pub mode: AnalysisMode,
    pub precise: Option<bool>,
}
#[derive(Debug, Serialize)]
pub struct AnalysisToken {
    pub text: String,
    pub start: usize,
    pub end: usize,
}
#[derive(Debug, Serialize)]
pub struct AnalysisLeaf {
    pub clause: usize,
    pub text: String,
    pub negated: bool,
    pub tokens: Vec<AnalysisToken>,
}
#[derive(Debug, Serialize)]
pub struct DictionaryIdentity {
    pub id: String,
    pub sha256: String,
}
#[derive(Debug, Serialize)]
pub struct AnalysisResult {
    pub engine: &'static str,
    pub engine_version: &'static str,
    pub mode: AnalysisMode,
    pub dictionary: DictionaryIdentity,
    pub offset_unit: &'static str,
    pub leaves: Vec<AnalysisLeaf>,
}

pub struct Analyzer {
    jieba: Arc<Jieba>,
    id: String,
    sha256: String,
}

fn entries(text: &str) -> anyhow::Result<Vec<(&str, usize, &str)>> {
    let mut result = Vec::new();
    let mut words = HashSet::new();
    for (line, row) in text.lines().enumerate() {
        if row.trim().is_empty() {
            continue;
        }
        let parts: Vec<_> = row.split_whitespace().collect();
        if !(2..=3).contains(&parts.len()) {
            bail!("詞典第 {} 列必須是 word frequency [tag]", line + 1);
        }
        let word = parts[0];
        if !(1..=100).contains(&word.chars().count()) || !words.insert(word) {
            bail!("詞典第 {} 列詞彙過長或重複", line + 1);
        }
        let frequency: usize = parts[1]
            .parse()
            .with_context(|| format!("詞典第 {} 列詞頻錯誤", line + 1))?;
        if !(1..=1_000_000_000).contains(&frequency) {
            bail!("詞典第 {} 列詞頻超出範圍", line + 1);
        }
        result.push((word, frequency, parts.get(2).copied().unwrap_or("")));
        if result.len() > 10_000 {
            bail!("自訂詞典最多 10,000 筆");
        }
    }
    Ok(result)
}

impl Analyzer {
    pub fn load(custom: Option<&Path>) -> anyhow::Result<Self> {
        let custom = custom
            .map(|path| -> anyhow::Result<String> {
                let file = File::open(path).context("無法讀取中文自訂詞典")?;
                if !file.metadata()?.is_file() {
                    bail!("自訂詞典必須是一般檔案");
                }
                let mut bytes = Vec::new();
                file.take((MAX_DICTIONARY_BYTES + 1) as u64)
                    .read_to_end(&mut bytes)?;
                if bytes.len() > MAX_DICTIONARY_BYTES {
                    bail!("自訂詞典最多 1 MiB");
                }
                let text = String::from_utf8(bytes).context("自訂詞典必須為 UTF-8")?;
                entries(&text)?;
                Ok(text)
            })
            .transpose()?;
        let base = BASE_ANALYZER
            .get_or_init(|| {
                let mut jieba =
                    Jieba::with_dict(&mut Cursor::new(BASE)).map_err(|e| e.to_string())?;
                for (word, freq, tag) in entries(TAIWAN).map_err(|e| e.to_string())? {
                    jieba.add_word(word, Some(freq), Some(tag));
                }
                Ok(Arc::new(jieba))
            })
            .as_ref()
            .map_err(|message| anyhow::anyhow!("中文基礎詞典載入失敗：{message}"))?;
        let mut jieba = Arc::clone(base);
        if let Some(text) = &custom {
            for (word, freq, tag) in entries(text)? {
                Arc::make_mut(&mut jieba).add_word(word, Some(freq), Some(tag));
            }
        }
        let mut hash = Sha256::new();
        hash.update(BASE);
        hash.update([0]);
        hash.update(TAIWAN.as_bytes());
        if let Some(text) = &custom {
            hash.update([0]);
            hash.update(text.as_bytes());
        }
        Ok(Self {
            jieba,
            id: format!(
                "jieba-tc-67fa2e3+tw-v1{}",
                if custom.is_some() { "+custom" } else { "" }
            ),
            sha256: format!("{:x}", hash.finalize()),
        })
    }

    pub fn analyze(&self, input: AnalysisInput) -> AppResult<AnalysisResult> {
        if input.expression.chars().count() > 10_000 {
            return Err(AppError::bad("斷詞預覽最多 10,000 個字元"));
        }
        if input.precise == Some(false) {
            return Err(AppError::bad(
                "原樣模式不會進行中文斷詞；請使用精確片語模式",
            ));
        }
        let query = Query::parse(&input.expression)
            .map_err(|error| AppError::bad(syntax::error_text(&error)))?;
        let mut leaves = Vec::new();
        let mut count = 0;
        for (index, clause) in query.clauses().into_iter().enumerate() {
            let Some(text) = clause.text else { continue };
            if text
                .trim()
                .strip_prefix('#')
                .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()))
            {
                return Err(AppError::bad("請先展開組合語法編號，再進行中文斷詞"));
            }
            let mode = match input.mode {
                AnalysisMode::Normal => TokenizeMode::Default,
                AnalysisMode::Search => TokenizeMode::Search,
            };
            let tokens: Vec<_> = self
                .jieba
                .tokenize(&text, mode, false)
                .into_iter()
                .filter(|t| t.word.chars().any(char::is_alphanumeric))
                .map(|t| AnalysisToken {
                    text: t.word.to_owned(),
                    start: t.start,
                    end: t.end,
                })
                .collect();
            count += tokens.len();
            if count > 20_000 {
                return Err(AppError::bad("斷詞結果超過 20,000 個詞元，請縮短語法"));
            }
            leaves.push(AnalysisLeaf {
                clause: index,
                text,
                negated: clause.negated,
                tokens,
            });
        }
        Ok(AnalysisResult {
            engine: "jieba-rs",
            engine_version: "0.11.0",
            mode: input.mode,
            dictionary: DictionaryIdentity {
                id: self.id.clone(),
                sha256: self.sha256.clone(),
            },
            offset_unit: "unicode_scalar",
            leaves,
        })
    }
}
