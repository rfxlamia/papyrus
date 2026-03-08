use assert_cmd::Command;

#[test]
fn invalid_arguments_exit_with_code_2() {
    Command::cargo_bin("papyrus")
        .unwrap()
        .arg("convert")
        .assert()
        .code(2);
}
