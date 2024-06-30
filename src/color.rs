pub struct ANSIColors;

impl ANSIColors {
    pub const GREEN: &'static str = "\u{001B}[32m";
    pub const BLUE: &'static str = "\u{001B}[34m";
    pub const YELLOW: &'static str = "\u{001B}[33m";
    pub const MAGENTA: &'static str = "\u{001B}[35m";
    pub const CYAN: &'static str = "\u{001B}[36m";
    pub const RESET: &'static str = "\u{001B}[0m";
}

pub fn colorize(text: &str, color: &str) -> String {
    format!("{}{}{}", color, text, ANSIColors::RESET)
}