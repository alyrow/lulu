#[macro_export]
macro_rules! success {
    ($($arg:tt)*) => {{
        println!("{}{}", Paint::masked("✅  ").fg(Color::Green), Paint::green(format!($($arg)*)));
    }};
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {{
        eprintln!("{}{}", Paint::masked("❌  ").fg(Color::Red), Paint::red(format!($($arg)*)));
    }};
}

#[macro_export]
macro_rules! warning {
    ($($arg:tt)*) => {{
        eprintln!("{}{}", Paint::masked("⚠  ").fg(Color::Yellow), Paint::yellow(format!($($arg)*)));
    }};
}

#[macro_export]
macro_rules! title {
    ($icon:tt, $($arg:tt)*) => {{
        println!("{}  {}", Paint::masked($icon).fg(Color::Cyan), Paint::cyan(format!($($arg)*)).bold());
    }};
}

#[macro_export]
macro_rules! tip {
    ($($arg:tt)*) => {{
        println!("{}{}", Paint::masked("💡  ").fg(Color::Yellow), Paint::yellow(format!($($arg)*)).italic());
    }};
}