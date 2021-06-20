mod case;
mod case_options;
mod fork;

pub use case::{run_case, Case};
pub use case_options::CaseOptions;

pub fn all_cases() -> Vec<Box<dyn Case>> {
    vec![
        Box::new(fork::networking::Networking),
        Box::new(fork::rfc0221::RFC0221),
    ]
}
