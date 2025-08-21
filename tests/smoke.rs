use std::process::Command;

fn run(file: &str, call: &str) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_amlang"))
        .args([file, "--call", call])
        .output()
        .expect("run failed");
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn add_works() {
    let s = run("examples/add.am", "Add(1,4)");
    assert!(s.trim().ends_with("= 5"));
}
