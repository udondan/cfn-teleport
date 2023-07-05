use atty::Stream;
use console::style;
use spinach::{Color, Spinach, Spinner};
use std::io::Write;

pub struct Spin {
    spinner: Option<Spinach>,
    message: String,
}

impl Spin {
    pub fn new(message: &str) -> Self {
        if atty::is(Stream::Stdout) {
            let sp = Spinach::new_with(
                Spinner::new(vec!["-", "=", "≡"], 100),
                String::from(message),
                Color::Ignore,
            );
            Self {
                spinner: Some(sp),
                message: String::from(message),
            }
        } else {
            print!("{}", message);
            flush();
            Self {
                spinner: None,
                message: String::from(message),
            }
        }
    }

    pub fn complete(&mut self) {
        let success_prefix = style("✔".to_string()).green();

        if let Some(spinner) = self.spinner.take() {
            spinner.stop_with("✔", self.message.clone(), Color::Green);
        } else {
            println!(": {}", success_prefix);
        }
    }
}

fn flush() {
    std::io::stdout()
        .flush()
        .expect("Unable to flush to stdout")
}
