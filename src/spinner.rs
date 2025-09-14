use atty::Stream;
use console::style;
use spinach::{Spinner, RunningSpinner};
use std::io::Write;

pub struct Spin {
    spinner: Option<RunningSpinner>,
    message: String,
}

impl Spin {
    pub fn new(message: &str) -> Self {
        if atty::is(Stream::Stdout) {
            let sp = Spinner::new(message)
                .symbols(vec!["-", "=", "≡"])
                .frames_duration(100)
                .start();
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
            spinner.text(&self.message).symbol("✔").success();
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
