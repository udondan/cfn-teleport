use atty::Stream;
use console::style;
use spinoff::{spinners, Spinner};
use std::io::Write;

pub struct Spin {
    spinner: Option<Spinner>,
    message: String,
}

impl Spin {
    pub fn new(message: &str) -> Self {
        if atty::is(Stream::Stdout) {
            let sp = Spinner::new(spinners::Dots, String::from(message), None);
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
            spinner
                .stop_with_message(format!("{} {}", success_prefix, self.message.clone()).as_str());
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
