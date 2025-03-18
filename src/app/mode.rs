#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    #[default]
    Run,
    Quit,
}
