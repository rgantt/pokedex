use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Response<T: Serialize> {
    pub data: T,
    pub actions: Vec<Action>,
    pub meta: Meta,
}

#[derive(Debug, Clone, Serialize)]
pub struct Action {
    pub rel: String,
    pub cmd: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Meta {
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorResponse {
    pub error: ErrorDetail,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
}

impl Action {
    pub fn new(rel: &str, cmd: &str) -> Self {
        Self {
            rel: rel.to_string(),
            cmd: cmd.to_string(),
            description: None,
        }
    }

    pub fn with_description(rel: &str, cmd: &str, desc: &str) -> Self {
        Self {
            rel: rel.to_string(),
            cmd: cmd.to_string(),
            description: Some(desc.to_string()),
        }
    }
}

impl Meta {
    pub fn simple(command: &str) -> Self {
        Self {
            command: command.to_string(),
            total: None,
            limit: None,
            offset: None,
        }
    }

    pub fn paginated(command: &str, total: u64, limit: u64, offset: u64) -> Self {
        Self {
            command: command.to_string(),
            total: Some(total),
            limit: Some(limit),
            offset: Some(offset),
        }
    }
}

impl<T: Serialize> Response<T> {
    pub fn new(data: T, actions: Vec<Action>, meta: Meta) -> Self {
        Self {
            data,
            actions,
            meta,
        }
    }

    pub fn print(&self, format: &OutputFormat) -> anyhow::Result<()> {
        match format {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(self)?);
            }
            OutputFormat::Table => {
                // For table format, just print the JSON for now.
                // Individual commands can override with custom table formatting.
                println!("{}", serde_json::to_string_pretty(self)?);
            }
        }
        Ok(())
    }
}

impl ErrorResponse {
    pub fn not_found(message: &str, suggestions: Vec<Action>) -> Self {
        Self {
            error: ErrorDetail {
                code: "NOT_FOUND".to_string(),
                message: message.to_string(),
            },
            actions: suggestions,
        }
    }

    pub fn print(&self) -> anyhow::Result<()> {
        eprintln!("{}", serde_json::to_string_pretty(self)?);
        std::process::exit(1)
    }
}

#[derive(Debug, Clone, Default)]
pub enum OutputFormat {
    #[default]
    Json,
    Table,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(OutputFormat::Json),
            "table" => Ok(OutputFormat::Table),
            _ => Err(format!("Unknown format: {s}. Expected 'json' or 'table'")),
        }
    }
}
