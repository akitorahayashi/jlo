use crate::harness::TestContext;
use predicates::prelude::*;

#[test]
fn version_flag_prints_package_version() {
    let ctx = TestContext::new();

    ctx.cli()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn help_lists_visible_aliases() {
    let ctx = TestContext::new();

    ctx.cli().arg("--help").assert().success().stdout(
        predicate::str::contains("[aliases: i]").and(predicate::str::contains("[aliases: cr]")),
    );
}
