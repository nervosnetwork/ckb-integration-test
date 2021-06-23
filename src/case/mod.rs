mod case;
mod case_options;
mod fork;
mod logger;

pub use case::{run_case, Case};
pub use case_options::CaseOptions;

thread_local! {
    pub static CASE_NAME: ::std::cell::RefCell<String> = ::std::cell::RefCell::new(String::new());
}

pub fn all_cases() -> Vec<Box<dyn Case>> {
    vec![
        // Box::new(fork::networking::Networking),
        Box::new(fork::rfc0221::BeforeRFC0221Switch),
    ]
}
