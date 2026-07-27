use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaChatMessage, LlamaChatTemplate, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;

use super::llm_client::LlmError;

const CONTEXT_TOKENS: u32 = 2_048;
const MAX_GENERATED_TOKENS: usize = 4;
const MAX_CHOICES: usize = 10;

struct LoadedModel {
    canonical_path: PathBuf,
    model: LlamaModel,
}

struct EmbeddedRuntime {
    backend: LlamaBackend,
    loaded: Option<LoadedModel>,
}

// llama.cpp permits one backend initialization per process. The mutex also
// serializes inference so every NPC shares one resident model safely.
static EMBEDDED_RUNTIME: Mutex<Option<EmbeddedRuntime>> = Mutex::new(None);

pub fn validate_model_path(model_path: &str) -> Result<PathBuf, LlmError> {
    let trimmed = model_path.trim();
    if trimmed.is_empty() {
        return Err(LlmError::Embedded(
            "embedded model path is empty; select a GGUF file in Settings".to_string(),
        ));
    }

    let path = Path::new(trimmed);
    if !path.is_absolute() {
        return Err(LlmError::Embedded(format!(
            "embedded model path must be absolute: {}",
            path.display()
        )));
    }
    if !path.is_file() {
        return Err(LlmError::Embedded(format!(
            "embedded model file does not exist or is not a regular file: {}",
            path.display()
        )));
    }

    let is_gguf = path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("gguf"));
    if !is_gguf {
        return Err(LlmError::Embedded(format!(
            "embedded model must be a .gguf file: {}",
            path.display()
        )));
    }

    path.canonicalize().map_err(|e| {
        LlmError::Embedded(format!(
            "failed to resolve embedded model path {}: {e}",
            path.display()
        ))
    })
}

pub fn choose_index(
    model_path: &str,
    system: &str,
    user: &str,
    choice_count: usize,
) -> Result<usize, LlmError> {
    if !(1..=MAX_CHOICES).contains(&choice_count) {
        return Err(LlmError::Embedded(format!(
            "embedded inference requires between 1 and {MAX_CHOICES} choices; got {choice_count}"
        )));
    }

    let canonical_path = validate_model_path(model_path)?;
    let mut guard = EMBEDDED_RUNTIME
        .lock()
        .map_err(|_| LlmError::Embedded("embedded model runtime lock poisoned".to_string()))?;

    if guard.is_none() {
        let mut backend = LlamaBackend::init()
            .map_err(|e| LlmError::Embedded(format!("failed to initialize llama.cpp: {e}")))?;
        backend.void_logs();
        *guard = Some(EmbeddedRuntime {
            backend,
            loaded: None,
        });
    }

    let runtime = guard
        .as_mut()
        .ok_or_else(|| LlmError::Embedded("embedded runtime initialization failed".to_string()))?;

    let needs_load = runtime
        .loaded
        .as_ref()
        .is_none_or(|loaded| loaded.canonical_path != canonical_path);

    if needs_load {
        let model = LlamaModel::load_from_file(
            &runtime.backend,
            &canonical_path,
            &LlamaModelParams::default(),
        )
        .map_err(|e| {
            LlmError::Embedded(format!(
                "failed to load GGUF model {}: {e}",
                canonical_path.display()
            ))
        })?;
        runtime.loaded = Some(LoadedModel {
            canonical_path: canonical_path.clone(),
            model,
        });
    }

    let loaded = runtime
        .loaded
        .as_ref()
        .ok_or_else(|| LlmError::Embedded("embedded model was not loaded".to_string()))?;

    generate_choice(&runtime.backend, &loaded.model, system, user, choice_count)
}

fn generate_choice(
    backend: &LlamaBackend,
    model: &LlamaModel,
    system: &str,
    user: &str,
    choice_count: usize,
) -> Result<usize, LlmError> {
    let messages = [
        LlamaChatMessage::new("system".to_string(), system.to_string())
            .map_err(|e| LlmError::Embedded(format!("invalid embedded system prompt: {e}")))?,
        LlamaChatMessage::new("user".to_string(), user.to_string())
            .map_err(|e| LlmError::Embedded(format!("invalid embedded user prompt: {e}")))?,
    ];

    let template = match model.chat_template(None) {
        Ok(template) => template,
        Err(_) => LlamaChatTemplate::new("chatml")
            .map_err(|e| LlmError::Embedded(format!("failed to build chat template: {e}")))?,
    };
    let prompt = model
        .apply_chat_template(&template, &messages, true)
        .map_err(|e| LlmError::Embedded(format!("failed to apply model chat template: {e}")))?;
    let tokens = model
        .str_to_token(&prompt, AddBos::Always)
        .map_err(|e| LlmError::Embedded(format!("failed to tokenize embedded prompt: {e}")))?;

    if tokens.is_empty() {
        return Err(LlmError::Embedded(
            "embedded prompt tokenized to an empty sequence".to_string(),
        ));
    }

    let required_tokens = tokens.len() + MAX_GENERATED_TOKENS;
    if required_tokens > CONTEXT_TOKENS as usize {
        return Err(LlmError::Embedded(format!(
            "embedded prompt is too large: requires {required_tokens} tokens but context is {CONTEXT_TOKENS}"
        )));
    }

    let threads = std::thread::available_parallelism()
        .map(|n| n.get().min(8) as i32)
        .unwrap_or(2);
    let context_params = LlamaContextParams::default()
        .with_n_ctx(NonZeroU32::new(CONTEXT_TOKENS))
        .with_n_threads(threads)
        .with_n_threads_batch(threads);
    let mut context = model
        .new_context(backend, context_params)
        .map_err(|e| LlmError::Embedded(format!("failed to create embedded context: {e}")))?;

    let mut batch = LlamaBatch::new(tokens.len().max(512), 1);
    let last_index = tokens.len() - 1;
    for (index, token) in tokens.iter().copied().enumerate() {
        batch
            .add(token, index as i32, &[0], index == last_index)
            .map_err(|e| LlmError::Embedded(format!("failed to build prompt batch: {e}")))?;
    }
    context
        .decode(&mut batch)
        .map_err(|e| LlmError::Embedded(format!("failed to evaluate embedded prompt: {e}")))?;

    let alternatives = (0..choice_count)
        .map(|index| format!("\"{index}\""))
        .collect::<Vec<_>>()
        .join(" | ");
    let grammar = format!("root ::= {alternatives}");
    let grammar_sampler = LlamaSampler::grammar(model, &grammar, "root")
        .map_err(|e| LlmError::Embedded(format!("failed to create choice grammar: {e}")))?;
    let mut sampler = LlamaSampler::chain_simple([grammar_sampler, LlamaSampler::greedy()]);

    let mut output = Vec::new();
    let mut next_position = tokens.len() as i32;
    for _ in 0..MAX_GENERATED_TOKENS {
        let token = sampler.sample(&context, batch.n_tokens() - 1);
        if model.is_eog_token(token) {
            break;
        }

        let bytes = model
            .token_to_piece_bytes(token, 32, true, None)
            .map_err(|e| LlmError::Embedded(format!("failed to decode embedded output: {e}")))?;
        output.extend_from_slice(&bytes);

        if let Some(choice) = output
            .iter()
            .copied()
            .find(|byte| byte.is_ascii_digit())
            .map(|digit| usize::from(digit - b'0'))
        {
            if choice < choice_count {
                return Ok(choice);
            }
        }

        batch.clear();
        batch
            .add(token, next_position, &[0], true)
            .map_err(|e| LlmError::Embedded(format!("failed to extend output batch: {e}")))?;
        context
            .decode(&mut batch)
            .map_err(|e| LlmError::Embedded(format!("failed during embedded generation: {e}")))?;
        next_position += 1;
    }

    Err(LlmError::Embedded(format!(
        "embedded model did not return a valid choice; raw output={:?}",
        String::from_utf8_lossy(&output)
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_model_path() {
        let error = validate_model_path(" ").expect_err("empty path must fail");
        assert!(error.to_string().contains("path is empty"));
    }

    #[test]
    fn rejects_non_gguf_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("model.bin");
        std::fs::write(&path, b"not a model").unwrap();
        let error = validate_model_path(path.to_str().unwrap()).expect_err("extension must fail");
        assert!(error.to_string().contains(".gguf"));
    }
}
