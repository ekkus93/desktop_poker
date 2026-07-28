use super::embedded_llm::{choose_index, validate_model_path};

#[test]
fn rejects_relative_model_path_with_actionable_error() {
    let error = validate_model_path("models/player.gguf")
        .expect_err("relative embedded model path must fail");
    let message = error.to_string();
    assert!(message.contains("must be absolute"), "unexpected error: {message}");
    assert!(message.contains("models/player.gguf"));
}

#[test]
fn rejects_missing_absolute_gguf_path_with_actionable_error() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let missing = directory.path().join("missing.gguf");
    let error = validate_model_path(
        missing
            .to_str()
            .expect("temporary path should be valid UTF-8"),
    )
    .expect_err("missing embedded model must fail");
    let message = error.to_string();
    assert!(
        message.contains("does not exist or is not a regular file"),
        "unexpected error: {message}"
    );
    assert!(message.contains("missing.gguf"));
}

#[test]
fn rejects_directory_even_when_name_ends_in_gguf() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let fake_model_directory = directory.path().join("directory.gguf");
    std::fs::create_dir(&fake_model_directory).expect("create fake model directory");

    let error = validate_model_path(
        fake_model_directory
            .to_str()
            .expect("temporary path should be valid UTF-8"),
    )
    .expect_err("directory must not be accepted as a model file");
    assert!(
        error
            .to_string()
            .contains("does not exist or is not a regular file")
    );
}

#[test]
fn rejects_choice_counts_before_touching_the_model_path() {
    let zero = choose_index("/definitely/not/a/model.gguf", "system", "user", 0)
        .expect_err("zero choices must fail before model validation");
    assert!(zero.to_string().contains("between 1 and 10 choices"));

    let eleven = choose_index("/definitely/not/a/model.gguf", "system", "user", 11)
        .expect_err("too many choices must fail before model validation");
    assert!(eleven.to_string().contains("between 1 and 10 choices"));
}

#[test]
fn corrupt_gguf_file_reports_load_failure_instead_of_selecting_a_choice() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let corrupt = directory.path().join("corrupt.gguf");
    std::fs::write(&corrupt, b"not a gguf model").expect("write corrupt model fixture");

    let error = choose_index(
        corrupt
            .to_str()
            .expect("temporary path should be valid UTF-8"),
        "Choose a legal poker action.",
        "0 or 1",
        2,
    )
    .expect_err("corrupt GGUF must not produce a decision");
    let message = error.to_string();
    assert!(
        message.contains("failed to load GGUF model"),
        "unexpected error: {message}"
    );
    assert!(message.contains("corrupt.gguf"));
}
