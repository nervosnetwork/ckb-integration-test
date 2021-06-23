use crate::node::Node;
use crate::TESTDATA_DIR;
use std::fs;
use std::path::PathBuf;

mod epoch2;
mod height13;

pub trait Testdata {
    fn testdata_name(&self) -> &str {
        testdata_name(self)
    }
    fn generate(&self);
}

pub fn all_testdata_generators() -> Vec<Box<dyn Testdata>> {
    vec![
        Box::new(height13::Height13TestData),
        Box::new(epoch2::Epoch2V1TestData),
        Box::new(epoch2::Epoch2V2TestData),
    ]
}

fn testdata_name<T: ?Sized>(_: &T) -> &str {
    let type_name = ::std::any::type_name::<T>();
    type_name.split_terminator("::").last().unwrap()
}

fn dump_testdata(mut node: Node, testdata_name: &str) {
    let testdata_dir = &*TESTDATA_DIR.lock();
    let working_dir = node.working_dir();
    let source_dir = format!("{}/data/db", working_dir.display());
    let target_dir = format!("{}/db/{}", testdata_dir.display(), testdata_name);
    node.stop();

    if !testdata_dir.exists() {
        fs::create_dir_all(&testdata_dir).unwrap_or_else(|err| {
            panic!(
                "failed to create TESTDATA_DIR \"{}\", error: {}",
                testdata_dir.display(),
                err
            )
        });
    }
    if PathBuf::from(&target_dir).exists() {
        fs::remove_dir_all(&target_dir).unwrap_or_else(|err| {
            panic!("failed to remove dir \"{}\", error: {}", target_dir, err)
        });
    }
    fs::rename(&source_dir, &target_dir).unwrap_or_else(|err| {
        panic!(
            "failed to rename directory from \"{}\" to \"{}\", error: {}",
            source_dir, target_dir, err
        )
    });
}
