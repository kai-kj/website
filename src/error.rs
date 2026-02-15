use std::fmt::{Debug, Formatter};
use std::panic::Location;

pub struct Error {
    message: String,
    file: String,
    line: u32,
    column: u32,
    child: Option<Box<Self>>,
}

impl Error {
    #[track_caller]
    pub fn new<S: Into<String>>(message: S) -> Self {
        let location = Location::caller();
        Self {
            message: message.into(),
            file: location.file().to_string(),
            line: location.line(),
            column: location.column(),
            child: None,
        }
    }

    #[track_caller]
    pub fn context<S: Into<String>>(self, message: S) -> Self {
        let location = Location::caller();
        Self {
            message: message.into(),
            file: location.file().to_string(),
            line: location.line(),
            column: location.column(),
            child: Some(Box::new(self)),
        }
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f)?;

        let mut current = Some(self);
        while let Some(error) = current {
            writeln!(
                f,
                "{}:{}:{}: {}",
                error.file, error.line, error.column, error.message
            )?;
            current = error.child.as_ref().map(|e| &**e);
        }

        Ok(())
    }
}

impl<T: std::error::Error> From<T> for Error {
    fn from(value: T) -> Self {
        Self::new(value.to_string())
    }
}

pub trait WithContext<T, S: Into<String>> {
    fn context(self, message: S) -> Result<T, Error>;
}

impl<T, E: std::error::Error, S: Into<String>> WithContext<T, S> for Result<T, E> {
    #[track_caller]
    fn context(self, message: S) -> Result<T, Error> {
        // self.map_err(|e| Error::new(e.to_string()).context(message))

        match self {
            Ok(value) => Ok(value),
            Err(error) => {
                let location = Location::caller();
                Err(Error {
                    message: message.into(),
                    file: location.file().to_string(),
                    line: location.line(),
                    column: location.column(),
                    child: Some(Box::new(error.into())),
                })
            }
        }
    }
}

impl<T, S: Into<String>> WithContext<T, S> for Option<T> {
    #[track_caller]
    fn context(self, message: S) -> Result<T, Error> {
        // self.ok_or_else(|| Error::new(message.into()))

        match self {
            Some(value) => Ok(value),
            None => {
                let location = Location::caller();
                Err(Error {
                    message: message.into(),
                    file: location.file().to_string(),
                    line: location.line(),
                    column: location.column(),
                    child: None,
                })
            }
        }
    }
}

impl<T, S: Into<String>> WithContext<T, S> for Result<T, Error> {
    #[track_caller]
    fn context(self, message: S) -> Result<T, Error> {
        // self.map_err(|e| e.context(message))

        match self {
            Ok(value) => Ok(value),
            Err(error) => {
                let location = Location::caller();
                Err(Error {
                    message: message.into(),
                    file: location.file().to_string(),
                    line: location.line(),
                    column: location.column(),
                    child: Some(Box::new(error)),
                })
            }
        }
    }
}
