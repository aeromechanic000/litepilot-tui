// @LITE_DESC Rust library skeleton with public API, trait definitions, thiserror-based error types, and module organization
// @LITE_SCENE Well-structured library template showcasing trait-based design, error handling, and idiomatic Rust patterns
// @LITE_TAGS rust, library, trait, module, api

mod core;
mod error;
mod impls;

pub use error::{Error, Result};

// Public API - Core trait that all processors must implement
pub trait Processor {
    /// Process the input data and return the result
    fn process(&self, input: &str) -> Result<String>;

    /// Get the processor name
    fn name(&self) -> &str;

    /// Check if this processor can handle the given input
    fn can_process(&self, input: &str) -> bool {
        !input.is_empty()
    }
}

// Public API - Configuration builder
pub struct Config {
    pub max_length: usize,
    pub strict_mode: bool,
    pub timeout_ms: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_length: 1024,
            strict_mode: false,
            timeout_ms: 5000,
        }
    }
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_max_length(mut self, max_length: usize) -> Self {
        self.max_length = max_length;
        self
    }

    pub fn with_strict_mode(mut self, strict_mode: bool) -> Self {
        self.strict_mode = strict_mode;
        self
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }
}

// Public API - Main library struct
pub struct Library {
    config: Config,
    processors: Vec<Box<dyn Processor>>,
}

impl Library {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            processors: Vec::new(),
        }
    }

    pub fn register_processor(&mut self, processor: Box<dyn Processor>) -> &mut Self {
        self.processors.push(processor);
        self
    }

    pub fn process(&self, input: &str) -> Result<String> {
        if input.len() > self.config.max_length {
            return Err(Error::InputTooLong {
                length: input.len(),
                max: self.config.max_length,
            });
        }

        for processor in &self.processors {
            if processor.can_process(input) {
                return processor.process(input);
            }
        }

        Err(Error::NoProcessorFound)
    }
}

// Public API - Trait for additional functionality
pub trait Validator {
    fn validate(&self, input: &str) -> Result<()>;
}

// Public API - Extension trait for string processing
pub trait StringExt {
    fn sanitize(&self) -> String;
    fn truncate(&self, max_len: usize) -> String;
}

impl StringExt for String {
    fn sanitize(&self) -> String {
        self.chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect()
    }

    fn truncate(&self, max_len: usize) -> String {
        if self.len() <= max_len {
            self.clone()
        } else {
            format!("{}...", &self[..max_len.saturating_sub(3)])
        }
    }
}

// Module: error.rs
mod error {
    use thiserror::Error;

    #[derive(Debug, Error)]
    pub enum Error {
        #[error("Input too long: {length} bytes (max: {max})")]
        InputTooLong { length: usize, max: usize },

        #[error("No processor found for the given input")]
        NoProcessorFound,

        #[error("Processor '{name}' failed: {reason}")]
        ProcessorFailed { name: String, reason: String },

        #[error("Invalid input: {0}")]
        InvalidInput(String),

        #[error("Configuration error: {0}")]
        ConfigError(String),
    }

    pub type Result<T> = std::result::Result<T, Error>;
}

// Module: core.rs
mod core {
    use super::{Error, Processor, Result};

    pub struct BasicProcessor {
        name: String,
    }

    impl BasicProcessor {
        pub fn new(name: impl Into<String>) -> Self {
            Self {
                name: name.into(),
            }
        }
    }

    impl Processor for BasicProcessor {
        fn process(&self, input: &str) -> Result<String> {
            Ok(format!("Processed by {}: {}", self.name, input))
        }

        fn name(&self) -> &str {
            &self.name
        }
    }
}

// Module: impls.rs
mod impls {
    use super::{Error, Processor, Result, Validator};

    pub struct StrictProcessor {
        validator: Box<dyn Validator>,
    }

    impl StrictProcessor {
        pub fn new(validator: Box<dyn Validator>) -> Self {
            Self { validator }
        }
    }

    impl Processor for StrictProcessor {
        fn process(&self, input: &str) -> Result<String> {
            self.validator.validate(input)?;
            Ok(format!("Strictly processed: {}", input))
        }

        fn name(&self) -> &str {
            "strict_processor"
        }

        fn can_process(&self, input: &str) -> bool {
            input.len() >= 3
        }
    }

    // Example validator implementation
    pub struct LengthValidator {
        min_length: usize,
    }

    impl LengthValidator {
        pub fn new(min_length: usize) -> Self {
            Self { min_length }
        }
    }

    impl Validator for LengthValidator {
        fn validate(&self, input: &str) -> Result<()> {
            if input.len() < self.min_length {
                return Err(Error::InvalidInput(format!(
                    "Input too short: minimum {} characters",
                    self.min_length
                )));
            }
            Ok(())
        }
    }
}
