pub trait OptionExt<T> {
    fn ok_or_try<F, E>(self, f: F) -> Result<T, E>
    where
        F: FnOnce() -> Result<T, E>;
}

impl<T> OptionExt<T> for Option<T> {
    fn ok_or_try<F, E>(self, f: F) -> Result<T, E>
    where
        F: FnOnce() -> Result<T, E>,
    {
        match self {
            Some(v) => Ok(v),
            None => f(),
        }
    }
}
