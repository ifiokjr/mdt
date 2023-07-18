use assert_cmd::Command;
use mdt::AnyEmptyResult;

#[test]
fn can_init() -> AnyEmptyResult {
  let mut cmd = Command::cargo_bin("mdt").unwrap();
  let assert = cmd.arg("init").assert().success();
  assert.stdout("initializing project!\n");
  Ok(())
}
