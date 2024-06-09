pub mod air;
pub mod rap;

#[cfg(feature = "air-logger")]
use rust_xlsxwriter::Format;

#[cfg(feature = "air-logger")]
use crate::folders::EntriesLog;

#[cfg(feature = "air-logger")]
fn generate_format<T: Copy + Ord>(
    header_format: &mut Format,
    entries: &EntriesLog<T>,
    entry: T,
) -> Format {
    use rust_xlsxwriter::Color;

    let failing = entries.failing.contains(&entry);
    let constrained = entries.constrained.contains(&entry);
    match (failing, constrained) {
        (true, _) => {
            *header_format = Format::new().set_background_color(Color::Red);
            Format::new().set_background_color(Color::Red)
        }
        (false, true) => Format::new(),
        (_, _) => {
            *header_format = Format::new().set_background_color(Color::Yellow);
            Format::new().set_background_color(Color::Yellow)
        }
    }
}
