pub type TemplateResult<T> = Result<T, TemplateError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateErrorKind {
    Code,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateErrorPhase {
    Parse,
    Resolve,
    Render,
    Io,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateErrorLocation {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateError {
    pub kind: TemplateErrorKind,
    pub phase: TemplateErrorPhase,
    pub template_path: Option<String>,
    pub directive: Option<String>,
    pub location: Option<TemplateErrorLocation>,
    pub message: String,
}

impl TemplateError {
    fn new(kind: TemplateErrorKind, phase: TemplateErrorPhase, message: impl Into<String>) -> Self {
        Self {
            kind,
            phase,
            template_path: None,
            directive: None,
            location: None,
            message: message.into(),
        }
    }

    pub(crate) fn code(phase: TemplateErrorPhase, message: impl Into<String>) -> Self {
        Self::new(TemplateErrorKind::Code, phase, message)
    }

    pub(crate) fn internal(phase: TemplateErrorPhase, message: impl Into<String>) -> Self {
        Self::new(TemplateErrorKind::Internal, phase, message)
    }

    pub(crate) fn with_template_path(mut self, template_path: impl Into<String>) -> Self {
        self.template_path = Some(template_path.into());
        self
    }

    pub(crate) fn with_directive(mut self, directive: impl Into<String>) -> Self {
        self.directive = Some(directive.into());
        self
    }

    pub(crate) fn with_location(mut self, line: usize, column: usize) -> Self {
        self.location = Some(TemplateErrorLocation { line, column });
        self
    }
}

impl std::fmt::Display for TemplateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} {:?}: {}", self.kind, self.phase, self.message)?;

        if let Some(path) = &self.template_path {
            write!(f, " [template: {}", path)?;
            if let Some(location) = &self.location {
                write!(f, ":{}:{}", location.line, location.column)?;
            }
            write!(f, "]")?;
        } else if let Some(location) = &self.location {
            write!(f, " [line {}:{}]", location.line, location.column)?;
        }

        if let Some(directive) = &self.directive {
            write!(f, " [directive: {}]", directive)?;
        }

        Ok(())
    }
}

impl std::error::Error for TemplateError {}
