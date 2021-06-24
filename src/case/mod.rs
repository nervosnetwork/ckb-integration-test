mod basic;
mod case;
mod case_options;
mod rfc0221;

pub use case::Case;
pub use case_options::CaseOptions;

pub fn all_cases() -> Vec<Box<dyn Case>> {
    vec![
        Box::new(basic::networking::BasicNetworking),
        Box::new(rfc0221::before_switch::RFC0221BeforeSwitch),
        Box::new(rfc0221::after_switch::RFC0221AfterSwitch),
    ]
}

pub fn run_case(case: Box<dyn Case>) {
    use crate::{info, CASE_NAME};
    CASE_NAME.with(|c| {
        *c.borrow_mut() = case.case_name().to_string();
    });

    info!("********** START **********");
    let nodes = case.before_run();
    case.run(nodes);
    info!("********** END **********");
}
